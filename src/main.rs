// moreover — a pager for readers who can't press space.
//
// Day one: the deliberation record precedes the implementation.
// Design space (open questions + recommendations):
//   docs/adr/0002-pagination-for-a-reader-without-hands.md
// The name's full story:
//   docs/adr/0001-the-name-moreover.md

fn main() {
    eprintln!("moreover 0.0.1-dev — a pager for readers who can't press space.");
    eprintln!("Building in public; the design record lives in docs/adr/.");
    std::process::exit(2);
}
