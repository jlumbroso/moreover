// moreover — a pager for readers who can't press space.
//
// Library core, kept separate from the binary so the founding sketch can
// run as a real test (tests/sketch.rs). Design authority:
//   docs/adr/0002-pagination-for-a-reader-without-hands.md (Accepted)
// v0 is deliberately zero-dependency: std file locking (Rust >= 1.89)
// covers the spool discipline, and the trailer templating is bare
// {placeholder} substitution (minijinja is the recorded upgrade path).

pub mod chunker;
pub mod paging;
pub mod store;
pub mod trailer;
