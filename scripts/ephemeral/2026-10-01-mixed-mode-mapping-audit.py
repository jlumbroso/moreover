#!/usr/bin/env python3
"""Audit tombstone protection after stable reuse of fresh/legacy records.

Authorship: GPT-6 Astra delegated reviewer (contract_audit).
Prepared for review and integration by Lector 6 (GPT-6 Astra).

Run using this file's path from any working directory:
    python3 scripts/ephemeral/2026-10-01-mixed-mode-mapping-audit.py

Requires git, rustc, and a filesystem supporting the store's hard links.
Pins runtime 923178f; reads its store through git show, never the worktree.
The Rust harness includes the unmodified source and exposes derive_id only
through a fixture helper in the same module. No store behavior is patched.

Six isolated cases compare stable-created, fresh-created, and legacy-form
survivors, using either drop or gc(1) to remove an earlier ladder occupant.
For fresh and legacy cases, the harness moves an actual fresh-created record
to the target's second stable candidate, simulating that possible random
placement without waiting for it. The legacy fixture additionally removes
the mode field. Thus placement is controlled, not an observed random-id
collision. The stable control mints normally past the same first occupant.

After a stable mint has established the second-candidate mapping, drop or
GC retires only the first occupant. GC ages only that fixture record by
three days. The script reports whether a tombstone remains, whether the
survivor is readable, and whether stable replay changes the established ID.
These are store-record tests; no spool reads or user state are involved.
All source copies, binaries, and state are owned by a TemporaryDirectory
under scripts/ephemeral and removed when the script exits.
"""

import hashlib
from pathlib import Path
import subprocess
import sys
import tempfile


# Validation, 2026-10-01: invoked by absolute path from /private/tmp; passed.
# Both stable-created controls retained sf5x as a tombstone and replayed
# e3w2. All four fresh/legacy x drop/GC cases removed sf5x while e3w2
# survived, then stable replay returned sf5x. Owned scratch was removed.
# This supports HOLD on the mapping-preservation release requirement;
# it is controlled fixture evidence, not a naturally observed collision.
REVIEWED_COMMIT = "923178f7fed534c16ef9096a11a07c0a0fc7c9d8"

RUST_PROBE = r'''
#[allow(dead_code)]
mod reviewed {
    include!("store_reviewed.rs");
    pub fn audit_candidate(cursor: &Cursor, rung: u64) -> String {
        derive_id(cursor, 4, rung)
    }
}
use reviewed::{Cursor, FsStore, MintMode, Store};
use std::{fs, path::Path, time::{Duration, SystemTime}};

fn exercise(root: &Path, mode: &str, operation: &str) {
    let dir = root.join(format!("{mode}-{operation}"));
    let store = FsStore::open(dir.clone()).unwrap();
    let target = Cursor {
        spool: "0123456789abcdef".into(), offset: 42, line: 3,
        page: 2, desk: String::new(), nl: Some(3),
    };
    let first = reviewed::audit_candidate(&target, 0);
    let second = reviewed::audit_candidate(&target, 1);
    assert_ne!(first, second);
    let first_path = dir.join("cursors").join(&first);
    let second_path = dir.join("cursors").join(&second);
    fs::write(&first_path,
        "spool=ffffffffffffffff\noffset=9\nline=0\npage=9\ndesk=\nnl=0\nmode=fresh\n"
    ).unwrap();

    if mode == "stable" {
        assert_eq!(store.put_cursor(&target, MintMode::Stable).unwrap(), second);
    } else {
        let generated = store.put_cursor(&target, MintMode::Fresh).unwrap();
        let generated_path = dir.join("cursors").join(&generated);
        if generated != second {
            assert!(!second_path.exists());
            fs::rename(generated_path, &second_path).unwrap();
        }
        if mode == "legacy" {
            let text = fs::read_to_string(&second_path).unwrap();
            let text: String = text.lines().filter(|line| !line.starts_with("mode="))
                .map(|line| format!("{line}\n")).collect();
            fs::write(&second_path, text).unwrap();
        }
    }
    let established = store.put_cursor(&target, MintMode::Stable).unwrap();
    assert_eq!(established, second, "the stable mapping must be established first");
    let entries = store.list_cursors().unwrap();
    let stored_mode = entries.iter().find(|entry| entry.id == established).unwrap()
        .mode.clone();
    assert_eq!(stored_mode.as_deref(), match mode {
        "stable" => Some("stable"), "fresh" => Some("fresh"), "legacy" => None,
        _ => unreachable!(),
    });

    let tombstoned = match operation {
        "drop" => store.drop_cursor(&first).unwrap().tombstoned,
        "gc" => {
            let old = SystemTime::now() - Duration::from_secs(3 * 86_400);
            fs::File::open(&first_path).unwrap()
                .set_times(fs::FileTimes::new().set_modified(old)).unwrap();
            let report = store.gc(1).unwrap();
            assert_eq!(report.kept, 1, "only the target mapping should survive");
            report.tombstoned == 1
        }
        _ => unreachable!(),
    };
    let first_present = first_path.exists();
    let surviving_readable = store.get_cursor(&established).unwrap() == target;
    let replayed = store.put_cursor(&target, MintMode::Stable).unwrap();
    let changed = replayed != established;
    println!(
        "mode={mode} operation={operation} first={first} established={established} stored_mode={stored_mode:?} tombstoned={tombstoned} first_present={first_present} surviving_readable={surviving_readable} replayed={replayed} mapping_changed={changed}"
    );
    assert!(surviving_readable);
    if mode == "stable" {
        assert!(tombstoned && first_present);
        assert!(!changed);
        assert_eq!(fs::metadata(&first_path).unwrap().len(), 0);
    } else {
        assert!(!tombstoned && !first_present);
        assert!(changed, "the pinned mixed-mode defect must reproduce");
        assert_eq!(replayed, first);
    }
}

fn main() {
    let root = std::path::PathBuf::from(std::env::args().nth(1).expect("scratch directory"));
    for mode in ["stable", "fresh", "legacy"] {
        for operation in ["drop", "gc"] {
            exercise(&root, mode, operation);
        }
    }
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
    reviewed = checked_output(["git", "show", f"{revision}:src/store.rs"], repo)
    print(f"reviewed_commit={revision}")
    print(f"reviewed_store_sha256={hashlib.sha256(reviewed.encode()).hexdigest()}")
    print("instrumentation=none; helper exposes derive_id; fresh/legacy candidate placement is controlled")
    with tempfile.TemporaryDirectory(
        prefix=".mixed-mode-mapping-audit-", dir=artifact_home
    ) as temporary:
        scratch = Path(temporary)
        (scratch / "store_reviewed.rs").write_text(reviewed)
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
