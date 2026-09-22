// The founding sketch (ADR-0002, DOC: the spec sketch) as an executable
// acceptance test — the same move system3-blog makes with `bin/note`.
// If these tests fail, the tool no longer does what the founding record
// promised. Each test drives the real pipeline against a real FsStore in
// an isolated scratch directory; nothing is mocked (Fourth Directive).

use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use moreover::chunker::Unit;
use moreover::paging::{page_new, page_resume, Take};
use moreover::store::{Cursor, FsStore, Store};
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
    let t1 = page_new(&store, &input, Take::Units(10), Unit::Lines, &mut page1).unwrap();
    assert_eq!(page1, numbered_lines(10));
    let id = t1.cursor.clone().expect("113 lines remain, so a cursor must exist");
    assert_eq!(trailer(&t1), format!("<moreover: page 1, 10/123 lines, cursor: {id}>"));

    let mut rest = Vec::new();
    let t2 = page_resume(&store, &id, Take::All, Unit::Lines, 0, &mut rest).unwrap();
    assert_eq!([page1, rest].concat(), input, "page 1 + the rest must be the whole stream");
    assert_eq!(trailer(&t2), "<moreover: 123/123 lines, cursor: null>");
}

#[test]
fn sized_resumes_continue_page_numbers_to_exhaustion() {
    let store = scratch_store("pages");
    let input = numbered_lines(25);

    let mut out = Vec::new();
    let t1 = page_new(&store, &input, Take::Units(10), Unit::Lines, &mut out).unwrap();
    let t2 = page_resume(&store, t1.cursor.as_ref().unwrap(), Take::Units(10), Unit::Lines, 0, &mut out)
        .unwrap();
    assert_eq!((t2.page, t2.shown, t2.total), (2, 20, 25));
    let t3 = page_resume(&store, t2.cursor.as_ref().unwrap(), Take::Units(10), Unit::Lines, 0, &mut out)
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
    let t1 = page_new(&store, &input, Take::Units(10), Unit::Lines, &mut first).unwrap();
    let id = t1.cursor.unwrap();

    let mut a = Vec::new();
    let mut b = Vec::new();
    let ta = page_resume(&store, &id, Take::Units(10), Unit::Lines, 0, &mut a).unwrap();
    let tb = page_resume(&store, &id, Take::Units(10), Unit::Lines, 0, &mut b).unwrap();
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
    let t1 = page_new(&store, &input, Take::Units(10), Unit::Lines, &mut out).unwrap();
    let shouted = t1.cursor.unwrap().to_ascii_uppercase();

    let mut rest = Vec::new();
    let t2 = page_resume(&store, &shouted, Take::All, Unit::Lines, 0, &mut rest).unwrap();
    assert_eq!(t2.cursor, None);
    assert_eq!(rest, numbered_lines(12)[out.len()..].to_vec());
}

#[test]
fn bytes_mode_pages_by_bytes() {
    let store = scratch_store("bytes");
    let input = b"abcdefghij".to_vec();

    let mut out = Vec::new();
    let t1 = page_new(&store, &input, Take::Units(4), Unit::Bytes, &mut out).unwrap();
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
    let t = page_new(&store, b"", Take::Units(10), Unit::Lines, &mut out).unwrap();
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
            .put_cursor(&Cursor { spool: "s".into(), offset: 0, line: 0, page: 1 })
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
    let t1 = page_new(&store, &input, Take::Units(10), Unit::Lines, &mut page1).unwrap();

    let mut resumed = Vec::new();
    let t2 = page_resume(&store, t1.cursor.as_ref().unwrap(), Take::Units(5), Unit::Lines, 3, &mut resumed)
        .unwrap();
    // lines 8,9,10 reprinted, then 11..=15 delivered
    let expected: Vec<u8> = (8..=15).map(|i| format!("line {i}\n")).collect::<String>().into_bytes();
    assert_eq!(resumed, expected);
    assert_eq!((t2.page, t2.shown, t2.total), (2, 15, 20), "overlap must not count as progress");
}

#[test]
fn identical_streams_share_one_spool() {
    // Content-addressed spools: rerunning the same pipeline twice must not
    // duplicate state on disk.
    let store = scratch_store("dedup");
    let input = numbered_lines(50);
    let mut out = Vec::new();
    page_new(&store, &input, Take::Units(5), Unit::Lines, &mut out).unwrap();
    page_new(&store, &input, Take::Units(5), Unit::Lines, &mut out).unwrap();
    let spools = std::fs::read_dir(store.root().join("spools")).unwrap().count();
    assert_eq!(spools, 1);
}
