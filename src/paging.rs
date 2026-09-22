// Paging: the one algorithm, shared by first-read and resume. With the
// store settled, this really is "io::copy with a bookmark"
// (ADR-0002 Consequences).

use std::io::{self, Write};

use crate::chunker::{advance, total_units, Unit};
use crate::store::{Cursor, Store};
use crate::trailer::TrailerData;

/// What the caller asked for: a sized page or everything remaining.
#[derive(Debug, Clone, Copy)]
pub enum Take {
    Units(usize),
    All,
}

/// First contact with a stream: drain it, spool it, deliver page 1.
/// v0 drains fully before printing — that is how the sketch's first
/// trailer can already say `10/123` (trade-off recorded in ADR-0002;
/// live/unbounded streams are the `Y = ?` follow-up).
pub fn page_new(
    store: &dyn Store,
    input: &[u8],
    take: Take,
    unit: Unit,
    out: &mut dyn Write,
) -> io::Result<TrailerData> {
    let spool = store.put_spool(input)?;
    deliver(store, &spool, input, 0, 1, take, unit, out)
}

/// Resume from a cursor minted by an earlier invocation.
pub fn page_resume(
    store: &dyn Store,
    cursor_id: &str,
    take: Take,
    unit: Unit,
    out: &mut dyn Write,
) -> io::Result<TrailerData> {
    let cursor = store.get_cursor(cursor_id)?;
    let data = store.read_spool(&cursor.spool)?;
    deliver(store, &cursor.spool, &data, cursor.offset as usize, cursor.page, take, unit, out)
}

#[allow(clippy::too_many_arguments)]
fn deliver(
    store: &dyn Store,
    spool: &str,
    data: &[u8],
    start: usize,
    page: u64,
    take: Take,
    unit: Unit,
    out: &mut dyn Write,
) -> io::Result<TrailerData> {
    let start = start.min(data.len());
    let end = match take {
        Take::All => data.len(),
        Take::Units(n) => advance(data, start, n, unit),
    };
    out.write_all(&data[start..end])?;

    let total = total_units(data, unit);
    let shown = total_units(&data[..end], unit);
    let cursor = if end < data.len() {
        // More remains: mint the next position's petname. The old cursor
        // is untouched — positions are immutable, re-reads idempotent.
        Some(store.put_cursor(&Cursor {
            spool: spool.to_string(),
            offset: end as u64,
            line: total_units(&data[..end], Unit::Lines) as u64,
            page: page + 1,
        })?)
    } else {
        None
    };

    Ok(TrailerData {
        page,
        shown,
        total,
        unit,
        cursor,
        all: matches!(take, Take::All),
    })
}
