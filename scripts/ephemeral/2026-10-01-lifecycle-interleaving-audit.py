#!/usr/bin/env python3
"""Reproduce lifecycle/paging interleavings at the pinned verbs commit.

Authorship: GPT-6 Astra delegated reviewer (verbs_concurrency), for Lector 6.
Run: python3 scripts/ephemeral/2026-10-01-lifecycle-interleaving-audit.py

The Rust store, paging, chunking, and trailer modules are extracted unchanged
from the reviewed commit. A custom output writer runs a maintenance operation
during first-page output, after page_new has published the spool and before
it mints the continuation. This controls a legal process interleaving; it is
not evidence of an uncontrolled race observed in production.

Two cases use separate fresh state: gc 365, and drop of an unrelated cursor.
Both operations delete the in-flight spool, then page_new succeeds with a
readable cursor record whose stream cannot be resumed (NotFound).

Only owned scratch beneath scripts/ephemeral is created, then removed.
Successfully run 2026-10-01: both pinned cases reproduced. This preserves
historical evidence, not a regression expected to pass on a repaired tree.
"""

from pathlib import Path
import subprocess
import tempfile


REVIEWED_COMMIT = "923178f7fed534c16ef9096a11a07c0a0fc7c9d8"

RUST_PROBE = r'''
#[allow(dead_code)] mod store;
#[allow(dead_code)] mod paging;
#[allow(dead_code)] mod chunker;
#[allow(dead_code)] mod trailer;

use std::{io::{self, Write}, path::PathBuf};
use store::{FsStore, MintMode, Store};
use paging::{page_new, page_resume, Take};
use chunker::Unit;

struct ControlledOutput<'a> {
    store: &'a FsStore,
    drop_id: Option<String>,
    fired: bool,
    bytes: Vec<u8>,
}

impl Write for ControlledOutput<'_> {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.bytes.extend_from_slice(bytes);
        if !self.fired {
            self.fired = true;
            if let Some(id) = &self.drop_id {
                let report = self.store.drop_cursor(id)?;
                println!("controlled drop of unrelated cursor {id}: {report:?}");
                assert_eq!(report.spools_freed, 2);
            } else {
                let report = self.store.gc(365)?;
                println!("controlled gc 365: {report:?}");
                assert_eq!(report.spools_freed, 1);
            }
        }
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> { Ok(()) }
}

fn run(root: PathBuf, dropping: bool) {
    let store = FsStore::open(root).unwrap();
    let drop_id = if dropping {
        let parked = page_new(
            &store, b"old1\nold2\nold3\n", Take::Units(1),
            Unit::Lines, MintMode::Stable, &mut Vec::new(),
        ).unwrap();
        Some(parked.cursor.unwrap())
    } else { None };
    let mut output = ControlledOutput {
        store: &store, drop_id, fired: false, bytes: Vec::new(),
    };
    let delivered = page_new(
        &store, b"new1\nnew2\nnew3\n", Take::Units(1),
        Unit::Lines, MintMode::Stable, &mut output,
    ).expect("first page returns success after maintenance deleted its spool");
    assert_eq!(output.bytes, b"new1\n");
    let id = delivered.cursor.expect("successful first page promises more");
    let cursor = store.get_cursor(&id).expect("minted cursor is readable");
    let err = page_resume(
        &store, &id, Take::All, Unit::Lines, 0,
        MintMode::Stable, &mut Vec::new(),
    ).expect_err("pinned race must lose the remaining input");
    assert_eq!(err.kind(), io::ErrorKind::NotFound);
    assert!(!store.root().join("spools").join(&cursor.spool).exists());
    println!(
        "{}: first_page_ok=true cursor={id} cursor_readable=true resume={:?}",
        if dropping { "drop-unrelated" } else { "gc-during-first-page" },
        err.kind(),
    );
}

fn main() {
    let root = PathBuf::from(std::env::args().nth(1).expect("owned scratch"));
    run(root.join("gc-case"), false);
    run(root.join("drop-case"), true);
}
'''


def main():
    repository = Path(__file__).resolve().parents[2]
    revision = subprocess.check_output(
        ["git", "rev-parse", f"{REVIEWED_COMMIT}^{{commit}}"],
        cwd=repository, text=True,
    ).strip()
    assert revision == REVIEWED_COMMIT
    print(f"reviewed_commit={revision}", flush=True)
    with tempfile.TemporaryDirectory(
        prefix="2026-10-01-lifecycle-interleaving-", dir=Path(__file__).parent,
    ) as temporary:
        scratch = Path(temporary)
        for module in ("store", "paging", "chunker", "trailer"):
            source = subprocess.check_output(
                ["git", "show", f"{revision}:src/{module}.rs"], cwd=repository,
            )
            (scratch / f"{module}.rs").write_bytes(source)
        (scratch / "probe.rs").write_text(RUST_PROBE)
        binary = scratch / "probe"
        subprocess.run(
            ["rustc", "--edition=2021", str(scratch / "probe.rs"), "-o", str(binary)],
            check=True, cwd=repository, timeout=60,
        )
        subprocess.run([str(binary), str(scratch / "state")], check=True, timeout=15)


if __name__ == "__main__":
    main()
