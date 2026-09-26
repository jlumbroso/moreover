// Paging: the one algorithm, shared by first-read and resume. With the
// store settled, this really is "io::copy with a bookmark"
// (ADR-0002 Consequences) — and since ADR-0006, resume honors the
// bookmark literally: it seeks, and reads work proportional to the
// requested bytes plus indexing/overlap (never "O(1)"; one line can be
// huge). Spools or cursors minted before the metadata existed fall back
// to the legacy whole-read path, which is always correct, just costlier.

use std::io::{self, Write};

use crate::chunker::{advance, retreat, total_units, Unit};
use crate::store::{Cursor, SpoolMeta, Store};
use crate::trailer::TrailerData;

/// What the caller asked for: a sized page or everything remaining.
#[derive(Debug, Clone, Copy)]
pub enum Take {
    Units(usize),
    All,
}

/// Bounded-scan chunk size. Overlap's backward scan doubles from here,
/// so its cost tracks the overlap's actual bytes, not the spool.
const CHUNK: u64 = 64 * 1024;

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
    deliver_in_memory(store, &spool, input, 0, 1, take, unit, out)
}

/// Resume from a cursor minted by an earlier invocation. With
/// `overlap > 0`, the last `overlap` units before the cursor are
/// reprinted first — a re-print, not progress: the trailer's numbers
/// are unaffected by it (ADR-0003, `--overlap`).
pub fn page_resume(
    store: &dyn Store,
    cursor_id: &str,
    take: Take,
    unit: Unit,
    overlap: usize,
    out: &mut dyn Write,
) -> io::Result<TrailerData> {
    let cursor = store.get_cursor(cursor_id)?;
    match (store.spool_meta(&cursor.spool)?, cursor.nl) {
        (Some(meta), Some(nl0)) => {
            resume_bounded(store, &cursor, meta, nl0, take, unit, overlap, out)
        }
        _ => {
            // Legacy record or legacy spool: whole-read path.
            let data = store.read_spool(&cursor.spool)?;
            let start = (cursor.offset as usize).min(data.len());
            if overlap > 0 && start > 0 {
                let back = retreat(&data, start, overlap, unit);
                out.write_all(&data[back..start])?;
            }
            deliver_in_memory(store, &cursor.spool, &data, start, cursor.page, take, unit, out)
        }
    }
}

/// The in-memory delivery (page_new always; legacy resume). Mints the
/// next cursor with full metadata, so every new record can take the
/// bounded path next time.
#[allow(clippy::too_many_arguments)]
fn deliver_in_memory(
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
    let partial_at_end = end > 0 && data[end - 1] != b'\n';
    let cursor = mint_next(store, spool, data.len(), end, count_nl(&data[..end]), partial_at_end, page)?;
    Ok(TrailerData { page, shown, total, unit, cursor, all: matches!(take, Take::All) })
}

/// The bounded resume (ADR-0006): seek, stream `[start, end)` in chunks,
/// count newlines as they pass. Reads: the overlap window + the page
/// itself + at most one chunk of scan-ahead in line mode.
#[allow(clippy::too_many_arguments)]
fn resume_bounded(
    store: &dyn Store,
    cursor: &Cursor,
    meta: SpoolMeta,
    nl0: u64,
    take: Take,
    unit: Unit,
    overlap: usize,
    out: &mut dyn Write,
) -> io::Result<TrailerData> {
    let spool = &cursor.spool;
    let b = meta.bytes;
    let start = cursor.offset.min(b);

    // --- overlap: a bounded backward scan, window doubling until the
    // requested units fit (cost tracks the overlap's bytes).
    if overlap > 0 && start > 0 {
        let ov_start = match unit {
            Unit::Bytes => start.saturating_sub(overlap as u64),
            Unit::Lines => {
                let mut window = CHUNK;
                loop {
                    let w_start = start.saturating_sub(window);
                    let buf = store.read_spool_range(spool, w_start, start - w_start)?;
                    // line starts strictly before `start`, newest last
                    let mut starts: Vec<u64> = Vec::new();
                    if w_start == 0 {
                        starts.push(0);
                    }
                    for (i, &byte) in buf.iter().enumerate() {
                        let p = w_start + i as u64 + 1;
                        if byte == b'\n' && p < start {
                            starts.push(p);
                        }
                    }
                    if starts.len() >= overlap || w_start == 0 {
                        break starts[starts.len().saturating_sub(overlap)];
                    }
                    window *= 2;
                }
            }
        };
        let buf = store.read_spool_range(spool, ov_start, start - ov_start)?;
        out.write_all(&buf)?;
    }

    // --- the page: stream chunks, counting newlines as they pass.
    let mut pos = start;
    let mut nl_in_page: u64 = 0;
    let mut last_byte: Option<u8> = None;
    let mut remaining_lines = match (take, unit) {
        (Take::Units(n), Unit::Lines) => Some(n as u64),
        _ => None,
    };
    let end_bytes = match (take, unit) {
        (Take::Units(n), Unit::Bytes) => (start + n as u64).min(b),
        _ => b, // line-sized pages stop by count; All stops at b
    };
    while pos < end_bytes {
        let want = (end_bytes - pos).min(CHUNK);
        let buf = store.read_spool_range(spool, pos, want)?;
        if buf.is_empty() {
            break;
        }
        let mut cut = buf.len();
        if let Some(rem) = remaining_lines.as_mut() {
            let mut taken = 0;
            for (i, &byte) in buf.iter().enumerate() {
                if byte == b'\n' {
                    taken += 1;
                    if taken == *rem {
                        cut = i + 1;
                        break;
                    }
                }
            }
            *rem -= taken.min(*rem);
            if cut < buf.len() {
                remaining_lines = Some(0);
            }
        }
        let emit = &buf[..cut];
        out.write_all(emit)?;
        nl_in_page += count_nl(emit) as u64;
        last_byte = emit.last().copied().or(last_byte);
        pos += cut as u64;
        if remaining_lines == Some(0) {
            break;
        }
    }
    let end = pos;

    // --- exact line accounting from local information only. A trailing
    // partial line counts as shown (house rule). Three cases: at the
    // spool's end the meta's flag governs; an empty page inherits the
    // cursor's own stored count (line = nl + partial, the historical
    // semantics); otherwise the last emitted byte decides.
    let nl_end = nl0 + nl_in_page;
    let partial = if end == b {
        b > 0 && !meta.ends_nl
    } else if end == start {
        cursor.line > nl0
    } else {
        last_byte != Some(b'\n')
    };
    let (shown, total) = match unit {
        Unit::Bytes => (end as usize, b as usize),
        Unit::Lines => (
            (nl_end + u64::from(partial && end > 0)) as usize,
            meta.total_lines() as usize,
        ),
    };

    let next = mint_next(store, spool, b as usize, end as usize, nl_end as usize, partial, cursor.page)?;
    Ok(TrailerData {
        page: cursor.page,
        shown,
        total,
        unit,
        cursor: next,
        all: matches!(take, Take::All),
    })
}

/// Mint the next position's petname (immutable; re-reads idempotent),
/// or None at exhaustion. `line` keeps its historical count-lines
/// semantics (nl + trailing-partial); `nl` is the authoritative count
/// the bounded path does arithmetic with.
fn mint_next(
    store: &dyn Store,
    spool: &str,
    spool_len: usize,
    end: usize,
    nl_through_end: usize,
    partial_at_end: bool,
    page: u64,
) -> io::Result<Option<String>> {
    if end >= spool_len {
        return Ok(None);
    }
    Ok(Some(store.put_cursor(&Cursor {
        spool: spool.to_string(),
        offset: end as u64,
        line: (nl_through_end + usize::from(partial_at_end && end > 0)) as u64,
        page: page + 1,
        desk: std::env::current_dir().map(|p| p.display().to_string()).unwrap_or_default(),
        nl: Some(nl_through_end as u64),
    })?))
}

fn count_nl(data: &[u8]) -> usize {
    data.iter().filter(|&&b| b == b'\n').count()
}
