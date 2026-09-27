// The founding sketch (ADR-0002, DOC: the spec sketch) as an executable
// acceptance test — the same move system3-blog makes with `bin/note`.
// If these tests fail, the tool no longer does what the founding record
// promised. Each test drives the real pipeline against a real FsStore in
// an isolated scratch directory; nothing is mocked (Fourth Directive).

use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use moreover::chunker::Unit;
use moreover::paging::{page_new, page_resume, Take};
use moreover::store::{Cursor, FsStore, MintMode, Store};
use moreover::trailer::{render_with, TrailerData, V0};

static SCRATCH_SEQ: AtomicU64 = AtomicU64::new(0);

fn scratch_store(tag: &str) -> FsStore {
    let dir: PathBuf = std::env::temp_dir().join(format!(
        "moreover-test-{}-{}-{}",
        tag,
        std::process::id(),
        SCRATCH_SEQ.fetch_add(1, Ordering::SeqCst),
    ));
    FsStore::open(dir).expect("scratch state dir")
}

fn numbered_lines(n: usize) -> Vec<u8> {
    (1..=n).map(|i| format!("line {i}\n")).collect::<String>().into_bytes()
}

fn trailer(d: &TrailerData) -> String {
    render_with(&V0, None, d)
}

#[test]
fn the_founding_sketch_roundtrip() {
    // $ big-output | moreover -10
    // [first 10 lines]
    // <moreover: page 1, 10/123 lines, cursor: Ae2e>
    // $ moreover -c Ae2e --all
    // [the remaining 113 lines]
    // <moreover: 123/123 lines, cursor: null>
    let store = scratch_store("sketch");
    let input = numbered_lines(123);

    let mut page1 = Vec::new();
    let t1 = page_new(&store, &input, Take::Units(10), Unit::Lines, MintMode::Fresh, &mut page1).unwrap();
    assert_eq!(page1, numbered_lines(10));
    let id = t1.cursor.clone().expect("113 lines remain, so a cursor must exist");
    assert_eq!(trailer(&t1), format!("<moreover: page 1, 10/123 lines, cursor: {id}>"));

    let mut rest = Vec::new();
    let t2 = page_resume(&store, &id, Take::All, Unit::Lines, 0, MintMode::Fresh, &mut rest).unwrap();
    assert_eq!([page1, rest].concat(), input, "page 1 + the rest must be the whole stream");
    assert_eq!(trailer(&t2), "<moreover: 123/123 lines, cursor: null>");
}

#[test]
fn sized_resumes_continue_page_numbers_to_exhaustion() {
    let store = scratch_store("pages");
    let input = numbered_lines(25);

    let mut out = Vec::new();
    let t1 = page_new(&store, &input, Take::Units(10), Unit::Lines, MintMode::Fresh, &mut out).unwrap();
    let t2 = page_resume(&store, t1.cursor.as_ref().unwrap(), Take::Units(10), Unit::Lines, 0, MintMode::Fresh, &mut out)
        .unwrap();
    assert_eq!((t2.page, t2.shown, t2.total), (2, 20, 25));
    let t3 = page_resume(&store, t2.cursor.as_ref().unwrap(), Take::Units(10), Unit::Lines, 0, MintMode::Fresh, &mut out)
        .unwrap();
    // The last page is short and final: exact exhaustion prints null even
    // on a sized call.
    assert_eq!((t3.page, t3.shown, t3.total), (3, 25, 25));
    assert_eq!(t3.cursor, None);
    assert_eq!(out, input);
}

#[test]
fn cursors_are_immutable_so_rereads_are_idempotent() {
    // A model that lost track and replays the same cursor must get the
    // same page, not a mysteriously advanced stream.
    let store = scratch_store("idempotent");
    let input = numbered_lines(30);

    let mut first = Vec::new();
    let t1 = page_new(&store, &input, Take::Units(10), Unit::Lines, MintMode::Fresh, &mut first).unwrap();
    let id = t1.cursor.unwrap();

    let mut a = Vec::new();
    let mut b = Vec::new();
    let ta = page_resume(&store, &id, Take::Units(10), Unit::Lines, 0, MintMode::Fresh, &mut a).unwrap();
    let tb = page_resume(&store, &id, Take::Units(10), Unit::Lines, 0, MintMode::Fresh, &mut b).unwrap();
    assert_eq!(a, b);
    assert_eq!((ta.page, ta.shown), (tb.page, tb.shown));
}

#[test]
fn cursor_ids_resume_case_insensitively() {
    // Base-32 petnames are read case-insensitively (QST-CURSOR-SEMANTICS):
    // a reader retyping the id in caps must still turn the page.
    let store = scratch_store("case");
    let input = numbered_lines(12);

    let mut out = Vec::new();
    let t1 = page_new(&store, &input, Take::Units(10), Unit::Lines, MintMode::Fresh, &mut out).unwrap();
    let shouted = t1.cursor.unwrap().to_ascii_uppercase();

    let mut rest = Vec::new();
    let t2 = page_resume(&store, &shouted, Take::All, Unit::Lines, 0, MintMode::Fresh, &mut rest).unwrap();
    assert_eq!(t2.cursor, None);
    assert_eq!(rest, numbered_lines(12)[out.len()..].to_vec());
}

#[test]
fn bytes_mode_pages_by_bytes() {
    let store = scratch_store("bytes");
    let input = b"abcdefghij".to_vec();

    let mut out = Vec::new();
    let t1 = page_new(&store, &input, Take::Units(4), Unit::Bytes, MintMode::Fresh, &mut out).unwrap();
    assert_eq!(out, b"abcd");
    assert_eq!(
        trailer(&t1),
        format!("<moreover: page 1, 4/10 bytes, cursor: {}>", t1.cursor.as_ref().unwrap())
    );
}

#[test]
fn empty_input_is_calm() {
    // `true | moreover -10` happens constantly in real pipelines; it must
    // produce an honest trailer, not an error.
    let store = scratch_store("empty");
    let mut out = Vec::new();
    let t = page_new(&store, b"", Take::Units(10), Unit::Lines, MintMode::Fresh, &mut out).unwrap();
    assert!(out.is_empty());
    assert_eq!(trailer(&t), "<moreover: page 1, 0/0 lines, cursor: null>");
}

#[test]
fn cursor_ids_always_mix_letters_and_digits() {
    // Estate petname doctrine (folded in via ADR-0003 iteration 2): an
    // all-digit id reads as a counter — a model reader may extrapolate a
    // sequence instead of reusing the printed id — and an all-letter id
    // reads as a word. Minting must guarantee the mixed, code-like form.
    let store = scratch_store("mixed");
    for _ in 0..100 {
        let id = store
            .put_cursor(&Cursor { spool: "s".into(), offset: 0, line: 0, page: 1, desk: String::new(), nl: Some(0) }, MintMode::Fresh)
            .unwrap();
        assert!(
            id.bytes().any(|b| b.is_ascii_digit()) && id.bytes().any(|b| b.is_ascii_alphabetic()),
            "minted id '{id}' is not mixed letter+digit"
        );
    }
}

#[test]
fn overlap_reprints_context_without_counting_it() {
    // --overlap (ADR-0003, reader-first cut): a resuming model re-anchors
    // by seeing the tail of the previous page again. The reprint must not
    // move the trailer's numbers — it is context, not progress.
    let store = scratch_store("overlap");
    let input = numbered_lines(20);

    let mut page1 = Vec::new();
    let t1 = page_new(&store, &input, Take::Units(10), Unit::Lines, MintMode::Fresh, &mut page1).unwrap();

    let mut resumed = Vec::new();
    let t2 = page_resume(&store, t1.cursor.as_ref().unwrap(), Take::Units(5), Unit::Lines, 3, MintMode::Fresh, &mut resumed)
        .unwrap();
    // lines 8,9,10 reprinted, then 11..=15 delivered
    let expected: Vec<u8> = (8..=15).map(|i| format!("line {i}\n")).collect::<String>().into_bytes();
    assert_eq!(resumed, expected);
    assert_eq!((t2.page, t2.shown, t2.total), (2, 15, 20), "overlap must not count as progress");
}

#[test]
fn stable_minting_is_idempotent_and_keyed_on_the_full_triple() {
    // ADR-0005: under --mint stable, his million-call scenario costs one
    // record — replaying the same resume yields the SAME next cursor —
    // while Lector's equality case holds: two histories meeting at one
    // byte offset with different page ordinals keep DISTINCT records,
    // so the trailer's page number stays exactly truthful.
    let store = scratch_store("stable");
    let input = numbered_lines(40);

    let mut out = Vec::new();
    let t1 = page_new(&store, &input, Take::Units(10), Unit::Lines, MintMode::Stable, &mut out).unwrap();
    let id = t1.cursor.unwrap();

    // replay the same resume three times: identical next id every time
    let mut next_ids = Vec::new();
    for _ in 0..3 {
        let mut sink = Vec::new();
        let t = page_resume(&store, &id, Take::Units(10), Unit::Lines, 0, MintMode::Stable, &mut sink).unwrap();
        next_ids.push(t.cursor.unwrap());
    }
    assert_eq!(next_ids[0], next_ids[1]);
    assert_eq!(next_ids[1], next_ids[2]);

    // Lector's equality dimension: reach byte offset of line 20 via
    // 10+10 (page ordinal 3) and via a single -20 (page ordinal 2) —
    // same (spool, offset), different page → distinct stable ids.
    let mut sink = Vec::new();
    let via_two = page_resume(&store, &id, Take::Units(10), Unit::Lines, 0, MintMode::Stable, &mut sink).unwrap();
    let mut sink2 = Vec::new();
    let via_one = page_new(&store, &input, Take::Units(20), Unit::Lines, MintMode::Stable, &mut sink2).unwrap();
    let (a, b) = (via_two.cursor.unwrap(), via_one.cursor.unwrap());
    assert_ne!(a, b, "same offset, different page ordinal must keep distinct records");
    let (ca, cb) = (store.get_cursor(&a).unwrap(), store.get_cursor(&b).unwrap());
    assert_eq!(ca.offset, cb.offset, "the two histories do meet at one byte offset");
    assert_ne!(ca.page, cb.page);

    // fresh mode still varies (the assertion flag means what it says)
    let mut s1 = Vec::new();
    let mut s2 = Vec::new();
    let f1 = page_resume(&store, &id, Take::Units(10), Unit::Lines, 0, MintMode::Fresh, &mut s1).unwrap();
    let f2 = page_resume(&store, &id, Take::Units(10), Unit::Lines, 0, MintMode::Fresh, &mut s2).unwrap();
    assert_ne!(f1.cursor.unwrap(), f2.cursor.unwrap());

    // every successful mint round-trips through get_cursor (audit,
    // finding 3's invariant)
    for nid in &next_ids {
        store.get_cursor(nid).expect("a minted id its own reader rejects is a defect");
    }
}

#[test]
fn stable_histories_converging_on_the_same_triple_share_the_successor() {
    // The audit's complementary equality test: 5+15 and 10+10 both land
    // at (spool, offset-of-line-20, page 3) — different histories, same
    // identity triple, same successor id.
    let store = scratch_store("converge");
    let input = numbered_lines(40);

    let mut sink = Vec::new();
    let a1 = page_new(&store, &input, Take::Units(5), Unit::Lines, MintMode::Stable, &mut sink).unwrap();
    let a2 = page_resume(&store, a1.cursor.as_ref().unwrap(), Take::Units(15), Unit::Lines, 0, MintMode::Stable, &mut sink).unwrap();
    let b1 = page_new(&store, &input, Take::Units(10), Unit::Lines, MintMode::Stable, &mut sink).unwrap();
    let b2 = page_resume(&store, b1.cursor.as_ref().unwrap(), Take::Units(10), Unit::Lines, 0, MintMode::Stable, &mut sink).unwrap();
    assert_eq!(a2.cursor.unwrap(), b2.cursor.unwrap(), "same (spool, offset, page) must share one record");
}

#[test]
fn unreadable_debris_at_a_stable_candidate_climbs_the_ladder_deterministically() {
    // The audit's publication-window class, as its reachable legacy
    // form: an empty (crash-truncated) file squatting on a stable
    // candidate. Publication is atomic now, so the tool can't create
    // this state itself — but it must survive finding it: the mint
    // climbs the ladder, deterministically, and the result round-trips.
    let store = scratch_store("debris");
    let input = numbered_lines(20);

    let mut sink = Vec::new();
    let t1 = page_new(&store, &input, Take::Units(10), Unit::Lines, MintMode::Stable, &mut sink).unwrap();
    let honest = t1.cursor.unwrap();

    // simulate the debris: destroy the record, squat its name with 0 bytes
    let path = store.root().join("cursors").join(&honest);
    std::fs::write(&path, b"").unwrap();

    let mut s2 = Vec::new();
    let r2 = page_new(&store, &input, Take::Units(10), Unit::Lines, MintMode::Stable, &mut s2).unwrap();
    let laddered = r2.cursor.unwrap();
    assert_ne!(laddered, honest, "an unreadable occupant must not be reused");
    store.get_cursor(&laddered).expect("laddered id must round-trip");

    let mut s3 = Vec::new();
    let r3 = page_new(&store, &input, Take::Units(10), Unit::Lines, MintMode::Stable, &mut s3).unwrap();
    assert_eq!(r3.cursor.unwrap(), laddered, "the ladder is deterministic given the same debris");
}

#[test]
fn legacy_state_without_meta_or_nl_still_resumes_correctly() {
    // Regression guard for ADR-0006's compatibility promise: state dirs
    // written by v0.2.0 have neither spool .meta sidecars nor cursor nl
    // fields. Both degradations must fall back to the whole-read path
    // and produce byte-identical output — correct, just costlier.
    let store = scratch_store("legacy");
    let input = numbered_lines(30);
    let mut p1 = Vec::new();
    let t1 = page_new(&store, &input, Take::Units(10), Unit::Lines, MintMode::Fresh, &mut p1).unwrap();
    let id = t1.cursor.unwrap();

    // simulate a legacy spool: remove the sidecar
    let meta = std::fs::read_dir(store.root().join("spools"))
        .unwrap()
        .map(|e| e.unwrap().path())
        .find(|p| p.extension().is_some_and(|x| x == "meta"))
        .unwrap();
    std::fs::remove_file(&meta).unwrap();

    let mut rest = Vec::new();
    let t2 = page_resume(&store, &id, Take::All, Unit::Lines, 0, MintMode::Fresh, &mut rest).unwrap();
    assert_eq!([p1.clone(), rest].concat(), input);
    assert_eq!((t2.shown, t2.total, t2.cursor), (30, 30, None));

    // simulate a legacy cursor: strip its nl= line (meta restored via re-page)
    let mut p1b = Vec::new();
    let t1b = page_new(&store, &input, Take::Units(10), Unit::Lines, MintMode::Fresh, &mut p1b).unwrap();
    let idb = t1b.cursor.unwrap();
    let cpath = store.root().join("cursors").join(&idb);
    let stripped: String = std::fs::read_to_string(&cpath)
        .unwrap()
        .lines()
        .filter(|l| !l.starts_with("nl="))
        .map(|l| format!("{l}\n"))
        .collect();
    std::fs::write(&cpath, stripped).unwrap();
    let mut restb = Vec::new();
    let t2b = page_resume(&store, &idb, Take::All, Unit::Lines, 0, MintMode::Fresh, &mut restb).unwrap();
    assert_eq!([p1b, restb].concat(), input);
    assert_eq!((t2b.shown, t2b.total, t2b.cursor), (30, 30, None));
}

#[test]
fn bounded_resume_is_exact_across_a_line_larger_than_its_chunk() {
    // The bounded path streams 64KiB chunks; a single line can dwarf
    // that ("one line can be huge" — the ruled wording's own caveat).
    // Ground truth is the input slice itself.
    let store = scratch_store("hugeline");
    let huge = "x".repeat(200_000);
    let input: Vec<u8> = format!("alpha\nbeta\n{huge}\ngamma\ndelta\n").into_bytes();

    let mut p1 = Vec::new();
    let t1 = page_new(&store, &input, Take::Units(2), Unit::Lines, MintMode::Fresh, &mut p1).unwrap();
    assert_eq!(p1, b"alpha\nbeta\n");

    // the huge line arrives whole through the chunked scanner
    let mut p2 = Vec::new();
    let t2 = page_resume(&store, t1.cursor.as_ref().unwrap(), Take::Units(1), Unit::Lines, 0, MintMode::Fresh, &mut p2)
        .unwrap();
    assert_eq!(p2, format!("{huge}\n").into_bytes());
    assert_eq!((t2.page, t2.shown, t2.total), (2, 3, 5));

    // overlap's backward scan must cross the huge line intact
    let mut p3 = Vec::new();
    let t3 = page_resume(&store, t2.cursor.as_ref().unwrap(), Take::Units(1), Unit::Lines, 1, MintMode::Fresh, &mut p3)
        .unwrap();
    assert_eq!(p3, format!("{huge}\ngamma\n").into_bytes());
    assert_eq!((t3.shown, t3.total), (4, 5), "overlap reprint must not count as progress");
}

#[test]
fn identical_streams_share_one_spool() {
    // Content-addressed spools: rerunning the same pipeline twice must not
    // duplicate state on disk.
    let store = scratch_store("dedup");
    let input = numbered_lines(50);
    let mut out = Vec::new();
    page_new(&store, &input, Take::Units(5), Unit::Lines, MintMode::Fresh, &mut out).unwrap();
    page_new(&store, &input, Take::Units(5), Unit::Lines, MintMode::Fresh, &mut out).unwrap();
    // one spool + its one drain-time .meta sidecar (ADR-0006), both deduped
    let names: Vec<String> = std::fs::read_dir(store.root().join("spools"))
        .unwrap()
        .map(|e| e.unwrap().file_name().into_string().unwrap())
        .collect();
    assert_eq!(names.len(), 2, "expected spool + meta, got: {names:?}");
    assert_eq!(names.iter().filter(|n| n.ends_with(".meta")).count(), 1);
}
