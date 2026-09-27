// Store-level audit probe, not a product implementation or a passing test.
// Origin: GPT-6 Astra delegated reviewer (contract_audit); adapted and
// retained by Lector 6 (GPT-6 Astra; gpt-6-astra).
//
// Run from the repository root:
// rustc --edition=2021 scripts/ephemeral/2026-09-26-stable-mint-audit.rs -o target/lector-stable-mint-audit
// target/lector-stable-mint-audit
//
// Includes the actual store implementation. The short collision is searched
// for, not assumed. The incomplete-file and occupied-ladder cases are
// controlled filesystem fixtures, not claims of naturally observed races
// or saturation. No spool reads are needed for these record-store checks.
// Scratch records live under scripts/ephemeral and are removed on exit.
//
// At 0f3f128: a real short collision used another four-character candidate;
// removing its earlier occupant changed a surviving triple's returned ID;
// an incomplete primary record caused duplicate IDs; filling the ladder
// through length 16 produced a length-17 ID that get_cursor rejected.

#[allow(dead_code)]
mod store {
    include!("../../src/store.rs");

    pub fn candidate(cursor: &Cursor, len: usize, rung: u64) -> String {
        derive_id(cursor, len, rung)
    }
}

use std::{collections::HashMap, fs, path::PathBuf};
use store::{Cursor, FsStore, MintMode, Store};

struct Scratch(PathBuf);
impl Drop for Scratch {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).expect("remove this probe's scratch records");
    }
}

fn cursor(offset: u64) -> Cursor {
    Cursor {
        spool: "0123456789abcdef".into(),
        offset,
        line: u64::from(offset > 0),
        page: 2,
        desk: "/audit".into(),
        nl: Some(0),
    }
}

fn record(cursor: &Cursor) -> String {
    format!(
        "spool={}\noffset={}\nline={}\npage={}\ndesk={}\nnl=0\nmode=stable\n",
        cursor.spool, cursor.offset, cursor.line, cursor.page, cursor.desk
    )
}

fn main() {
    let root = PathBuf::from("scripts/ephemeral")
        .join(format!(".stable-mint-audit-{}", std::process::id()));
    // Refuse an existing directory; the cleanup guard owns only this one.
    fs::create_dir(&root).expect("create fresh audit directory");
    let scratch = Scratch(root);

    let mut seen = HashMap::new();
    let mut pair = None;
    for offset in 0..100_000 {
        let id = store::candidate(&cursor(offset), 4, 0);
        if let Some(prior) = seen.insert(id.clone(), offset) {
            pair = Some((prior, offset, id));
            break;
        }
    }
    let (a, b, candidate) = pair.expect("short collision in bounded search");
    let store = FsStore::open(scratch.0.join("collision")).unwrap();
    let aid = store.put_cursor(&cursor(a), MintMode::Stable).unwrap();
    let bid = store.put_cursor(&cursor(b), MintMode::Stable).unwrap();
    let replay = store.put_cursor(&cursor(b), MintMode::Stable).unwrap();
    println!(
        "collision: offsets={a},{b}; candidate={candidate}; ids={aid},{bid}; replay_b={replay}"
    );
    fs::remove_file(store.root().join("cursors").join(&aid)).unwrap();
    let after = store.put_cursor(&cursor(b), MintMode::Stable).unwrap();
    println!(
        "after removing A: b={after}; prior_b_record_survives={}",
        store.root().join("cursors").join(&bid).exists()
    );

    let store = FsStore::open(scratch.0.join("partial-publication")).unwrap();
    let c = cursor(1);
    let primary = store::candidate(&c, 4, 0);
    // The state visible immediately after a first writer's create_new,
    // while that writer is paused before writing the record body.
    fs::write(store.root().join("cursors").join(&primary), b"").unwrap();
    let during = store.put_cursor(&c, MintMode::Stable).unwrap();
    fs::write(store.root().join("cursors").join(&primary), record(&c)).unwrap();
    let after = store.put_cursor(&c, MintMode::Stable).unwrap();
    println!(
        "publication fixture: primary={primary}; while_incomplete={during}; after_completion={after}; records={}",
        fs::read_dir(store.root().join("cursors")).unwrap().count()
    );

    let store = FsStore::open(scratch.0.join("occupied-ladder")).unwrap();
    let c = cursor(42);
    let blocker = record(&cursor(999_999));
    let mut rung = 0;
    for len in 4..=16 {
        for _ in 0..8 {
            let id = store::candidate(&c, len, rung);
            fs::write(store.root().join("cursors").join(id), &blocker).unwrap();
            rung += 1;
        }
    }
    let id = store.put_cursor(&c, MintMode::Stable).unwrap();
    println!(
        "occupied ladder fixture: blocked_rungs={rung}; id={id}; length={}; normalizes={}; readback_error={:?}",
        id.len(),
        store::normalize_id(&id).is_some(),
        store.get_cursor(&id).err().map(|e| e.to_string())
    );
}
