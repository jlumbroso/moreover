#!/usr/bin/env python3
"""Pin the maintenance-lock repair's read-only resume behavior.

Authorship: GPT-6 Astra delegated reviewer (verbs_concurrency), for Lector 6.
Run: python3 scripts/ephemeral/2026-10-02-readonly-resume-audit.py

Requires Unix permissions, a non-root user, git, and rustc. Sources are
unchanged snapshots, and all generated files stay in owned temporary
scratch beneath scripts/ephemeral. No repository or user state is changed.

The same native probe creates a real continuation, makes its entire state
tree read-only, and resumes to exhaustion (no new cursor required). The
pre-repair snapshot succeeds. The repaired snapshot fails PermissionDenied
because shared-lock acquisition requires opening .txn-lock for writing.
For repaired state, restoring only the existing lock file's write bit makes
the same resume succeed while cursor/spool files and directories stay
read-only. A legacy-style store with no lock file also fails.

A separate repaired-state case leaves directories and the lock writable,
ages a cursor eight days, and makes only that cursor read-only. Exhausting
resume succeeds but fails to refresh its age; gc 7 then removes it and its
spool. This tests the stated writable-state-directory qualification on age
refresh independently of the lock-access failure.

Successfully run 2026-10-02: these outcomes were reproduced. Historical
audit evidence, not a regression intended to pass on subsequently fixed code.
"""

import os
from pathlib import Path
import subprocess
import tempfile


REVISIONS = (
    ("923178f7fed534c16ef9096a11a07c0a0fc7c9d8", "before"),
    ("3309c5c84a22b64c85b54e45e56b340b4c16faab", "after"),
)

RUST_PROBE = r'''
#[allow(dead_code)] mod store;
#[allow(dead_code)] mod paging;
#[allow(dead_code)] mod chunker;
#[allow(dead_code)] mod trailer;

use std::{fs, io, os::unix::fs::PermissionsExt, path::{Path, PathBuf}};
use store::{FsStore, MintMode};
use paging::{page_new, page_resume, Take};
use chunker::Unit;

fn modes(path: &Path, readonly: bool) {
    if path.is_dir() {
        if !readonly { fs::set_permissions(path, fs::Permissions::from_mode(0o700)).unwrap(); }
        for entry in fs::read_dir(path).unwrap() {
            modes(&entry.unwrap().path(), readonly);
        }
        fs::set_permissions(path, fs::Permissions::from_mode(if readonly { 0o500 } else { 0o700 })).unwrap();
    } else {
        fs::set_permissions(path, fs::Permissions::from_mode(if readonly { 0o400 } else { 0o600 })).unwrap();
    }
}

struct Restore(PathBuf);
impl Drop for Restore {
    fn drop(&mut self) { modes(&self.0, false); }
}

fn resume(store: &FsStore, id: &str) -> io::Result<Vec<u8>> {
    let mut output = Vec::new();
    let trailer = page_resume(
        store, id, Take::All, Unit::Lines, 0, MintMode::Stable, &mut output,
    )?;
    assert!(trailer.cursor.is_none());
    assert_eq!(output, b"line2\nline3\n");
    Ok(output)
}

fn run(path: PathBuf, repaired: bool, remove_lock: bool) {
    let store = FsStore::open(path.clone()).unwrap();
    let id = page_new(
        &store, b"line1\nline2\nline3\n", Take::Units(1), Unit::Lines,
        MintMode::Stable, &mut Vec::new(),
    ).unwrap().cursor.unwrap();
    let lock = path.join(".txn-lock");
    if remove_lock && lock.exists() { fs::remove_file(&lock).unwrap(); }
    let _restore = Restore(path.clone());
    modes(&path, true);
    let result = resume(&store, &id);
    println!(
        "repaired={repaired} legacy_lock_absent={remove_lock} readonly_resume={:?}",
        result.as_ref().map(|_| "ok").map_err(|e| e.kind()),
    );
    if repaired {
        assert_eq!(result.unwrap_err().kind(), io::ErrorKind::PermissionDenied);
        if !remove_lock {
            // Isolate the failure to shared-lock write access. All directory,
            // cursor, metadata, and spool permissions remain read-only.
            fs::set_permissions(&lock, fs::Permissions::from_mode(0o600)).unwrap();
            assert!(resume(&store, &id).is_ok());
            println!("only_lock_made_writable: exhaustion_resume=ok");
        }
    } else {
        assert!(result.is_ok());
    }
}

fn readonly_cursor_age(path: PathBuf) {
    use std::time::{Duration, SystemTime};
    let store = FsStore::open(path.clone()).unwrap();
    let id = page_new(
        &store, b"line1\nline2\nline3\n", Take::Units(1), Unit::Lines,
        MintMode::Stable, &mut Vec::new(),
    ).unwrap().cursor.unwrap();
    let cursor_path = path.join("cursors").join(&id);
    let old = SystemTime::now() - Duration::from_secs(8 * 86_400);
    fs::OpenOptions::new().write(true).open(&cursor_path).unwrap()
        .set_modified(old).unwrap();
    let stored_old = fs::metadata(&cursor_path).unwrap().modified().unwrap();
    let _restore = Restore(path.clone());
    fs::set_permissions(&cursor_path, fs::Permissions::from_mode(0o444)).unwrap();
    assert!(resume(&store, &id).is_ok());
    assert_eq!(fs::metadata(&cursor_path).unwrap().modified().unwrap(), stored_old);
    let report = store.gc(7).unwrap();
    assert_eq!(report.removed, 1);
    assert_eq!(report.spools_freed, 1);
    assert!(!cursor_path.exists());
    println!(
        "writable_directories_and_lock_readonly_cursor: resume=ok age_refreshed=false gc7={report:?}"
    );
}

fn main() {
    let path = PathBuf::from(std::env::args().nth(1).expect("owned scratch"));
    let repaired = std::env::args().nth(2).unwrap() == "after";
    run(path.join("normal"), repaired, false);
    run(path.join("legacy"), repaired, true);
    if repaired { readonly_cursor_age(path.join("readonly-cursor-age")); }
}
'''


def main():
    if os.geteuid() == 0:
        raise SystemExit("Run as a non-root user; root bypasses permissions.")
    repository = Path(__file__).resolve().parents[2]
    with tempfile.TemporaryDirectory(
        prefix="2026-10-02-readonly-resume-", dir=Path(__file__).parent,
    ) as temporary:
        scratch = Path(temporary)
        for revision, label in REVISIONS:
            resolved = subprocess.check_output(
                ["git", "rev-parse", f"{revision}^{{commit}}"],
                cwd=repository, text=True,
            ).strip()
            assert resolved == revision
            print(f"{label}_commit={revision}", flush=True)
            build = scratch / label
            build.mkdir()
            for module in ("store", "paging", "chunker", "trailer"):
                source = subprocess.check_output(
                    ["git", "show", f"{revision}:src/{module}.rs"], cwd=repository,
                )
                (build / f"{module}.rs").write_bytes(source)
            (build / "probe.rs").write_text(RUST_PROBE)
            binary = build / "probe"
            subprocess.run(
                ["rustc", "--edition=2021", str(build / "probe.rs"), "-o", str(binary)],
                check=True, cwd=repository, timeout=60,
            )
            subprocess.run([str(binary), str(build / "state"), label], check=True, timeout=15)


if __name__ == "__main__":
    main()
