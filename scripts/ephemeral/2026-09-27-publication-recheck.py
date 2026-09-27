#!/usr/bin/env python3
"""Reproduce the bounded publication audit of e591e6e; not a product test.

Authorship: GPT-6 Astra delegated reviewer (contract_audit).
Prepared for review and integration by Lector 6 (GPT-6 Astra).

Run from any directory:
    python3 scripts/ephemeral/2026-09-27-publication-recheck.py

Requires git, rustc, and a filesystem supporting hard links. The reviewed
store is extracted from the pinned commit, never from the current worktree.
This matters if a later implementation retries exclusive temporary creation:
fixing its random suffix could otherwise make that retry loop infinite.

Two cases run against copies of that snapshot:
  1. Four threads mint the same cursor using the UNMODIFIED snapshot. This
     is a small successful concurrency check, not proof against every race.
  2. A controlled temporary-name collision changes ONLY `mint_id(6)` to the
     fixed suffix `"a1b2c3"`. After the first mint, the fixture restores the
     temporary hard link that exists between publication and unlinking. A
     second, different mint chooses that same temporary name. Its File::create
     truncates the inode also named by the first published cursor.

Case 2 forces the collision condition; it does not claim that a random-name
collision or an uncontrolled thread race was observed. No other store code
is instrumented. This fixture exercises record storage; no spool reads occur.
Source copies, executable, and state are created in one owned temporary
directory under scripts/ephemeral and removed when the script exits.
"""

import hashlib
from pathlib import Path
import subprocess
import sys
import tempfile


REVIEWED_COMMIT = "e591e6e0af80c1fa46f6a7f59adce05bd2e6bbc0"

RUST_PROBE = r'''
#[allow(dead_code)]
mod real { include!("store_reviewed.rs"); }
#[allow(dead_code)]
mod forced { include!("store_forced_temp.rs"); }

use std::{fs, path::PathBuf, sync::{Arc, Barrier}, thread};
use real::Store as RealStore;
use forced::Store as ForcedStore;

fn main() {
    let root = PathBuf::from(std::env::args().nth(1).expect("scratch directory"));
    let store = Arc::new(real::FsStore::open(root.join("native-concurrent")).unwrap());
    let barrier = Arc::new(Barrier::new(4));
    let mut joins = Vec::new();
    for _ in 0..4 {
        let s = Arc::clone(&store);
        let b = Arc::clone(&barrier);
        joins.push(thread::spawn(move || {
            let cursor = real::Cursor {
                spool: "0123456789abcdef".into(), offset: 1, line: 1,
                page: 2, desk: String::new(), nl: Some(0),
            };
            b.wait();
            s.put_cursor(&cursor, real::MintMode::Stable).unwrap()
        }));
    }
    let ids: Vec<String> = joins.into_iter().map(|j| j.join().unwrap()).collect();
    let equal = ids.iter().all(|id| id == &ids[0]);
    let readable = store.get_cursor(&ids[0]).is_ok();
    let entries = fs::read_dir(store.root().join("cursors")).unwrap().count();
    println!(
        "unmodified_snapshot: concurrent_ids={ids:?} equal={equal} readable={readable} cursor_entries={entries}"
    );
    assert!(equal && readable);
    assert_eq!(entries, 1);

    let store = forced::FsStore::open(root.join("forced-temp-collision")).unwrap();
    let first = forced::Cursor {
        spool: "0123456789abcdef".into(), offset: 1, line: 1,
        page: 2, desk: String::new(), nl: Some(0),
    };
    let second = forced::Cursor { offset: 2, ..first.clone() };
    let first_id = store.put_cursor(&first, forced::MintMode::Stable).unwrap();
    let temporary_name = store.root().join("cursors").join(format!(
        ".mint-{}-a1b2c3", std::process::id()
    ));
    // Restore the inode relationship immediately after hard-link publication
    // and before the first writer unlinks its temporary name.
    fs::hard_link(
        store.root().join("cursors").join(&first_id), &temporary_name
    ).unwrap();
    let second_id = store.put_cursor(&second, forced::MintMode::Stable).unwrap();
    let observed = store.get_cursor(&first_id).unwrap();
    println!(
        "forced_temp_collision: original_id={first_id} original_offset={} second_id={second_id} original_id_now_offset={} original_record_unchanged={} random_collision_observed=false",
        first.offset, observed.offset, observed == first
    );
    assert_ne!(first_id, second_id);
    assert_eq!(observed, second, "the pinned defect must reproduce");
}
'''


def checked_output(args, cwd):
    return subprocess.run(
        args, cwd=cwd, capture_output=True, text=True, check=True, timeout=60
    ).stdout


def main():
    artifact_home = Path(__file__).resolve().parent
    repo = artifact_home.parents[1]
    revision = checked_output(
        ["git", "rev-parse", f"{REVIEWED_COMMIT}^{{commit}}"], repo
    ).strip()
    if revision != REVIEWED_COMMIT:
        raise RuntimeError(f"unexpected reviewed revision: {revision}")
    reviewed = checked_output(
        ["git", "show", f"{revision}:src/store.rs"], repo
    )
    if reviewed.count("mint_id(6)") != 1:
        raise RuntimeError("reviewed snapshot does not contain exactly one temp-suffix expression")
    instrumented = reviewed.replace("mint_id(6)", '"a1b2c3"')

    print(f"reviewed_commit={revision}")
    print(f"reviewed_store_sha256={hashlib.sha256(reviewed.encode()).hexdigest()}")
    print('instrumentation=one replacement: mint_id(6) -> "a1b2c3"')
    with tempfile.TemporaryDirectory(
        prefix=".publication-recheck-", dir=artifact_home
    ) as temporary:
        scratch = Path(temporary)
        (scratch / "store_reviewed.rs").write_text(reviewed)
        (scratch / "store_forced_temp.rs").write_text(instrumented)
        (scratch / "probe.rs").write_text(RUST_PROBE)
        binary = scratch / "probe"
        checked_output(
            ["rustc", "--edition=2021", str(scratch / "probe.rs"), "-o", str(binary)],
            repo,
        )
        print(checked_output([str(binary), str(scratch)], repo), end="")
    print("owned_temporary_artifacts_removed=true")


if __name__ == "__main__":
    try:
        main()
    except subprocess.CalledProcessError as error:
        sys.stderr.write(error.stdout or "")
        sys.stderr.write(error.stderr or "")
        sys.exit(f"audit command failed with exit {error.returncode}: {error.cmd}")
    except (OSError, RuntimeError, subprocess.TimeoutExpired) as error:
        sys.exit(f"audit failed: {error}")
