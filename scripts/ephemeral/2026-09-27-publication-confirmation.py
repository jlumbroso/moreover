#!/usr/bin/env python3
"""Confirm exclusive cursor/desk staging at 7914897; not a product test.

Authorship: GPT-6 Astra delegated reviewer (contract_audit).
Prepared for review and integration by Lector 6 (GPT-6 Astra).

Run from any directory using this file's path:
    python3 scripts/ephemeral/2026-09-27-publication-confirmation.py

Requires git, rustc, and a filesystem supporting hard links. Extracts the
reviewed store from the pinned commit, never from the current worktree.
An unmodified copy creates the first cursor and desk recovery record.
The fixture then hard-links each published record to the corresponding
staging name that the next mint will try. For the desk case, this alias
is a stronger ownership check than the ordinary rename path requires.

Exactly two expressions are instrumented in a second store copy: the
cursor and desk temporary suffix calls to mint_id(8). Each replacement
returns one occupied suffix and then a distinct free suffix; a third
request panics. Thus retry behavior is finite and cannot hang on a fixed
suffix. No publication, write, cleanup, or error-handling code is changed.

The second mint must retry both names, preserve both occupied aliases
and the original published cursor, atomically replace desk recovery with
the second id, and remove the newly owned staging files. These are forced
name collisions, not observed random collisions or concurrent execution.
No spool reads occur. Source copies, binary, and state stay in one owned
temporary directory under scripts/ephemeral and are removed on exit.
"""

import hashlib
from pathlib import Path
import subprocess
import sys
import tempfile


# Validation, 2026-09-27: invoked by absolute path from /private/tmp; passed.
# c2dc (offset 1) remained intact while 9rgn (offset 2) was minted. Each
# staging path used exactly two attempts; both occupied aliases stayed
# unchanged; the complete desk record and last selected 9rgn. Owned new
# staging files and the surrounding scratch directory were removed.
# Evidence closes the two temporary-ownership findings for this snapshot;
# it does not decide the separate recovery/error-path release verdict.
REVIEWED_COMMIT = "7914897bd90a160f22f7bd8361efa9d00670fe32"

RUST_PROBE = r'''
#[allow(dead_code)]
mod real { include!("store_reviewed.rs"); }
#[allow(dead_code)]
mod forced { include!("store_forced_temp.rs"); }

use std::{fs, path::PathBuf, sync::atomic::{AtomicUsize, Ordering}};
use real::Store as RealStore;
use forced::Store as ForcedStore;

const OCCUPIED: &str = "a1b2c3d4";
const FREE: &str = "e5f6g7h8";
static CURSOR_ATTEMPTS: AtomicUsize = AtomicUsize::new(0);
static DESK_ATTEMPTS: AtomicUsize = AtomicUsize::new(0);

fn next_suffix(counter: &AtomicUsize, label: &str) -> &'static str {
    match counter.fetch_add(1, Ordering::SeqCst) {
        0 => OCCUPIED,
        1 => FREE,
        attempt => panic!("{label} requested unexpected suffix attempt {attempt}"),
    }
}
fn next_cursor_suffix() -> &'static str { next_suffix(&CURSOR_ATTEMPTS, "cursor") }
fn next_desk_suffix() -> &'static str { next_suffix(&DESK_ATTEMPTS, "desk") }

fn main() {
    let root = PathBuf::from(std::env::args().nth(1).expect("scratch directory"));
    let state = root.join("forced-staging-collisions");
    let original_store = real::FsStore::open(state.clone()).unwrap();
    let first = real::Cursor {
        spool: "0123456789abcdef".into(), offset: 1, line: 1,
        page: 2, desk: "/audit/desk".into(), nl: Some(0),
    };
    let first_id = original_store.put_cursor(&first, real::MintMode::Stable).unwrap();
    let cursor_path = state.join("cursors").join(&first_id);
    let cursor_before = fs::read(&cursor_path).unwrap();
    let desk_paths: Vec<_> = fs::read_dir(state.join("desks")).unwrap()
        .map(|entry| entry.unwrap().path()).collect();
    assert_eq!(desk_paths.len(), 1);
    let desk_path = &desk_paths[0];
    let desk_before = fs::read(desk_path).unwrap();
    assert_eq!(String::from_utf8(desk_before.clone()).unwrap(),
        format!("desk={}\nid={first_id}\n", first.desk));

    let cursor_alias = state.join("cursors").join(format!(
        ".mint-{}-{OCCUPIED}", std::process::id()
    ));
    let desk_alias = state.join("desks").join(format!(
        ".part-{}-{OCCUPIED}", std::process::id()
    ));
    fs::hard_link(&cursor_path, &cursor_alias).unwrap();
    fs::hard_link(desk_path, &desk_alias).unwrap();

    let store = forced::FsStore::open(state.clone()).unwrap();
    let second = forced::Cursor {
        spool: first.spool.clone(), offset: 2, line: first.line,
        page: first.page, desk: first.desk.clone(), nl: first.nl,
    };
    let second_id = store.put_cursor(&second, forced::MintMode::Stable).unwrap();
    let cursor_attempts = CURSOR_ATTEMPTS.load(Ordering::SeqCst);
    let desk_attempts = DESK_ATTEMPTS.load(Ordering::SeqCst);
    let cursor_unchanged = fs::read(&cursor_path).unwrap() == cursor_before;
    let cursor_alias_unchanged = fs::read(&cursor_alias).unwrap() == cursor_before;
    let desk_alias_unchanged = fs::read(&desk_alias).unwrap() == desk_before;
    let desk_updated = fs::read_to_string(desk_path).unwrap()
        == format!("desk={}\nid={second_id}\n", second.desk);
    let last = store.last_cursor_for_desk(&second.desk).unwrap().unwrap();
    let cursor_temp_removed = !state.join("cursors").join(format!(
        ".mint-{}-{FREE}", std::process::id()
    )).exists();
    let desk_temp_removed = !state.join("desks").join(format!(
        ".part-{}-{FREE}", std::process::id()
    )).exists();

    println!("forced_staging_collisions: first_id={first_id} second_id={second_id} cursor_attempts={cursor_attempts} desk_attempts={desk_attempts}");
    println!("published_cursor_unchanged={cursor_unchanged} occupied_cursor_alias_unchanged={cursor_alias_unchanged} occupied_desk_alias_unchanged={desk_alias_unchanged}");
    println!("desk_record_complete_and_updated={desk_updated} last={last} owned_cursor_temp_removed={cursor_temp_removed} owned_desk_temp_removed={desk_temp_removed}");
    println!("random_collision_observed=false concurrent_execution=false");

    assert_ne!(first_id, second_id);
    assert_eq!(cursor_attempts, 2);
    assert_eq!(desk_attempts, 2);
    assert!(cursor_unchanged && cursor_alias_unchanged && desk_alias_unchanged);
    assert!(desk_updated && cursor_temp_removed && desk_temp_removed);
    assert_eq!(last, second_id);
    assert_eq!(store.get_cursor(&second_id).unwrap(), second);
    assert_eq!(original_store.get_cursor(&first_id).unwrap(), first);
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
    instrumented = reviewed
    for prefix, helper in ((".mint", "next_cursor_suffix"), (".part", "next_desk_suffix")):
        expression = f'format!("{prefix}-{{}}-{{}}", std::process::id(), mint_id(8))'
        if instrumented.count(expression) != 1:
            raise RuntimeError(f"expected exactly one staging expression: {expression}")
        replacement = expression.replace("mint_id(8)", f"crate::{helper}()")
        instrumented = instrumented.replace(expression, replacement)
    if instrumented.count("mint_id(8)") != reviewed.count("mint_id(8)") - 2:
        raise RuntimeError("unexpected instrumentation count")

    print(f"reviewed_commit={revision}")
    print(f"reviewed_store_sha256={hashlib.sha256(reviewed.encode()).hexdigest()}")
    print("instrumentation=two staging suffix calls; each returns occupied then free, then panics")
    with tempfile.TemporaryDirectory(
        prefix=".publication-confirmation-", dir=artifact_home
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
