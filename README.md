# moreover

[![crates.io](https://img.shields.io/crates/v/moreover.svg)](https://crates.io/crates/moreover)
[![API docs](https://img.shields.io/docsrs/moreover?label=API%20docs)](https://docs.rs/moreover/latest/moreover/)
[![Release build](https://github.com/jlumbroso/moreover/actions/workflows/release.yml/badge.svg)](https://github.com/jlumbroso/moreover/actions/workflows/release.yml)
[![Tests: source](https://img.shields.io/badge/tests-source-blue)](tests/)
[![Homebrew tap](https://img.shields.io/badge/Homebrew-jlumbroso%2Ftap-orange)](https://github.com/jlumbroso/homebrew-tap/blob/main/Formula/moreover.rb)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue)](LICENSE)

> **`more`, `less`, and now `moreover` — a pager for readers who can't press space.**

`moreover` is a **non-interactive pager** for LLMs, agents, and scripts.
It saves input from a pipe or file and prints a page. If more remains,
it returns a cursor that another invocation can use to read the next page.

```console
$ big-output | moreover -10
[first 10 lines]
<moreover: page 1, 10/123 lines, cursor: ae2e>

$ moreover -c ae2e --all
[the remaining 113 lines]
<moreover: 123/123 lines, cursor: null>
```

The trailer reports progress and the cursor for the next page.
`cursor: null` means the saved input is exhausted. Reuse the cursor your
invocation printed; `ae2e` above is an example, not a cursor to copy.

**Input must finish before the first page appears.** The current version
reads the whole input into memory and saves it on disk before printing.
Use it for finite output, not an ongoing stream such as `tail -f`.

## Install

### Homebrew

Install a prebuilt binary from [Jérémie Lumbroso's tap](https://github.com/jlumbroso/homebrew-tap):

```console
$ brew install jlumbroso/tap/moreover
```

Available for macOS and Linux on ARM64 and x86-64. No Rust toolchain is
required. Upgrade to a new release with:

```console
$ brew update && brew upgrade moreover
```

### Cargo

Requires Rust 1.89 or later; no external Rust dependencies.

```console
$ cargo install moreover
```

To install from a checkout of this repository, use `cargo install --path .`.

## Reading pages

The default page size is 10 lines. Use `-25` or `--lines 25` for a
different line count, `--bytes 1024` for bytes, or `--all` for the rest.
Each invocation chooses its own size and unit; a cursor does not retain
those options. Byte pages can split lines and encoded characters.

```console
$ moreover output.txt -25
$ moreover -c CURSOR -25 --overlap 3
```

Replace `CURSOR` with the printed cursor. `--overlap 3` repeats up to
three preceding units before the next page; the trailer does not count
them again. Resuming the same cursor with the same size, unit, and overlap
repeats the same content.

If the printed cursor is lost — scrolled away, or compacted out of a
model's context — `moreover -c last` recovers it: `last` selects the
cursor most recently used *from the current working directory*, so
concurrent readers in other directories don't collide. It answers
"which stream?", not "how much?": pair it with a page size or `--all`
like any resume (`moreover -c last --all`).

Content goes to stdout and the trailer to stderr by default. If your
caller captures only stdout, use `--trailer stdout`. Use `--trailer none`
to suppress the trailer, or `--trailer file:PATH` to append it to a file.
`--trailer fd:N` writes it to an inherited file descriptor.

Input and cursor records remain in the selected state directory. Its
precedence is `--state-dir PATH`, then `MOREOVER_STATE_DIR`, then
`$XDG_STATE_HOME/moreover`, then `~/.local/state/moreover`. Later
invocations need that same saved state. Resuming reads the saved input;
changes to the original file do not change it. Reaching the end does not
delete saved state.

To page a file named `contract`, `ls`, `stat`, `drop`, or `gc`, prefix
its name with `./`.

## The tool's own surfaces

The two authoritative descriptions are the binary's: `moreover --help`
is the IO for humans, `moreover contract` the IO for models. The
sections below are **generated from the binary itself**
(`scripts/readme-sync.py`), and a test fails the build whenever they
drift — the README sources these surfaces; it never paraphrases them.

<details>
<summary><b><code>moreover --help</code></b> — the option list, for humans</summary>

<!-- surface:help:begin -->

```text
moreover — a pager for readers who can't press space

Usage:
  <producer> | moreover [-N | -n N | --bytes N | --all]     (pipe mode)
  moreover FILE [-N | -n N | --bytes N | --all]             (file mode)
  moreover -c CURSOR [-N | --all] [--overlap N]             (resume; no input needed)

Paging:
  -N, -n N, --lines N   page size in lines (default: 10)
  --bytes N             page size in bytes
  --all                 everything (remaining)
  --overlap N           on resume, reprint the last N units before the new
                        page (context re-anchoring; a no-op on first pages)

Resumption:
  -c, --cursor ID       resume the stream that ID names
                        (a cursor is only valid if moreover printed it —
                        never invent or extrapolate one)
  -c last               select the newest saved cursor for this working
                        directory in the selected state directory
  --mint stable|fresh   cursor-id policy, asserted (default: stable —
                        the same resume repeated yields the same next
                        cursor; fresh mints a new id every time)

State:
  --state-dir PATH      spool/cursor store (default: $MOREOVER_STATE_DIR,
                        else $XDG_STATE_HOME/moreover, else ~/.local/state/moreover)

Trailer (the v0 grammar is a compatibility promise):
  --trailer DEST        route the trailer: stderr | stdout |
                        none | fd:N | file:PATH (append)
                        This flag overrides MOREOVER_TRAILER for this call.
                        Without this flag, MOREOVER_TRAILER sets the
                        default; unset or empty uses stderr.
  --schema NAME         trailer schema (default: v0)
  --schema-show         print the active schema's templates and exit
  --schema-template T   render the trailer with template T instead

Desk (subcommands — the standalone world; they never appear in pipes):
  moreover contract     print the model-facing contract
  moreover ls           list this directory's cursors (--everywhere:
                        every directory's, in the selected state dir)
  moreover drop CURSOR  declare a parked stream finished; free what it held
  moreover gc [DAYS]    sweep cursors whose last recorded use is DAYS or
                        more days old (default: 7; 0 removes every
                        record); orphaned input follows. Mint, reuse,
                        and resume attempt to refresh that time; a
                        failed refresh leaves the old one. gc and drop
                        act on the WHOLE selected state directory, not
                        just this working directory's cursors.
  (stat: reserved for the desk, not yet available)
  Desk verbs accept --state-dir like the pipe world does.

Introspection:
  --help                this text
  --version             version
```

<!-- surface:help:end -->

</details>

<details>
<summary><b><code>moreover contract</code></b> — the rules, for models</summary>

<!-- surface:contract:begin -->

```text
moreover: contract (v0)

purpose:
  Save input from a pipe or file, print a page, and resume the saved input
  in a later invocation using a printed cursor.
  Input must finish before the first page appears: the whole input is
  read into memory and saved on disk. Unbounded input never reaches a page.

invocations:
  stdin:    <producer> | moreover -10
  file:     moreover FILE -10
  resume:   moreover -c CURSOR --all
  recover:  moreover -c last
  contract: moreover contract
  list:     moreover ls
  drop:     moreover drop CURSOR
  sweep:    moreover gc [DAYS]
  Resume reads saved input, ignores stdin, and rejects an input file.
  Subcommands never read stdin. stat is reserved, not yet available.
  Prefix a filename matching a subcommand with ./ (for example, ./ls).
  With no file and no cursor, a stdin that is a terminal — or an open
  device delivering neither data nor end-of-file within 2 seconds, the
  shape many agent harnesses give — prints a short guide and exits 2
  instead of blocking forever. An empty but finished input is not that
  case: it earns its 0/0 trailer.

desk verbs:
  ls lists the cursors first minted from this working directory, one
  per line, newest first: ID, page, line (with the saved input's total
  when known), creation mode, the saved input's name, and age. The line
  -c last would select is marked. If the current recovery cursor was
  minted from another directory, it is listed with that note; the
  listing is not a history of every foreign cursor ever reused here.
  ls --everywhere lists every directory's cursors in the selected state
  directory — one store, not every store on the machine. Listing
  changes nothing, and an unreadable record is noted, not hidden.
  drop CURSOR removes that cursor's record FOR EVERY directory that
  uses it. Saved input still referenced by another cursor is kept;
  otherwise it is freed. Recovery records naming the dropped cursor are
  cleared, so -c last there selects an older record or reports none.
  Other cursors are unaffected.
  gc DAYS removes every cursor record in the selected state directory —
  all working directories — whose last recorded use was DAYS or more
  days ago; gc 0 removes every record unconditionally. GC uses the
  cursor record's last recorded use time. Minting, reuse, and resume
  attempt to refresh it. If that update fails, GC uses the previous
  timestamp, so a recently used cursor can still be collected. GC then
  frees saved input and recovery records nothing references.
  When a removed record occupies a slot that a kept stable ID's
  derivation walked past, an empty placeholder file is left in its
  place so that stable IDs stay reproducible; placeholders are swept
  once nothing depends on them. A record that cannot be read makes gc
  and drop refuse without removing anything: an unreadable record's
  references are unknown, and guessing could destroy another cursor's
  saved input. gc and drop wait for any in-progress paging in the same
  state directory, and paging waits for them. Both are permanent: a
  removed cursor ID stops resolving, and freed input cannot be resumed.
  Desk verbs accept --state-dir and never print a trailer.

paging:
  Every invocation defaults to 10 lines, including resume.
  -N, -n N, or --lines N selects lines; --bytes N selects bytes.
  --all prints the remainder with line counts; use it without a page size.
  Byte pages can split lines and encoded characters. Changing units on
  resume keeps the saved byte position, which may be inside a line.
  --overlap N repeats up to N preceding units before a resumed page,
  using this invocation's unit. It has no effect on the first page.

trailer (schema v0; a compatibility promise):
  paged form:  <moreover: page {page}, {shown}/{total} {unit}, cursor: {cursor}>
  --all form:  <moreover: {shown}/{total} {unit}, cursor: {cursor}>
  {shown} counts units from the start of saved input through this page's
  end; {total} counts the whole saved input. Both use this invocation's
  unit (lines or bytes). Overlap is not added again. Totals are numeric.
  {page} starts at 1 and advances through successive cursors.
  cursor: null means no input remains after this page; do not resume null.
  Otherwise, use the printed cursor to start the next page.
  --schema v0 selects the only current schema; --schema-show prints it.
  --schema-template T replaces the template using the placeholders above.

output:
  Content goes to stdout.
  --trailer DEST accepts stderr, stdout, none, fd:N, or file:PATH.
  Destination precedence: --trailer > MOREOVER_TRAILER > stderr.
  MOREOVER_TRAILER accepts the same destinations and sets the default
  for invocations that inherit it. Unset or empty uses stderr.
  An explicit --trailer overrides even an invalid environment value.
  If used as the default, an unrecognized environment destination causes
  a usage error before any content is emitted.
  The --help, --version, and --schema-show flags and the contract
  subcommand ignore MOREOVER_TRAILER.
  stdout places the trailer after the content; none suppresses it;
  fd:N writes to an inherited descriptor; file:PATH appends to a file.

shell redirection (MOREOVER_TRAILER unset; no --trailer flag):
  moreover FILE -10
    Content goes to stdout; the trailer goes to stderr. Capture both.
  moreover FILE -10 2>&1
    Content, trailer, and errors share stdout. A downstream pipe or
    captured value receives the trailer as data alongside the content.
  moreover FILE -10 2>/dev/null
    The trailer and errors are discarded. A cursor may still be saved,
    but its ID is lost from this output.
  For callers that capture only stdout, use --trailer stdout to include
  the trailer after the content. This still mixes content and metadata;
  use --trailer fd:N or file:PATH when they need separate destinations.
  Changing --trailer does not redirect errors; they still use stderr.
  Check the exit status before using the output.

cursors:
  Use a printed cursor ID — never invent or extrapolate one.
  The reserved word last selects a saved cursor as described below.
  A cursor fixes a position in saved input, not page size, unit, or overlap.
  Resuming a printed cursor ID with the same page size, unit, and overlap
  repeats the same content. The next cursor ID may differ.
  Resumption leaves the original cursor unchanged.
  IDs are case-insensitive; o folds to 0, and i and l fold to 1.
  IDs start at four characters and can be longer.
  --mint stable (the default) and --mint fresh assert the ID policy;
  each is an assertion, valid under any default. Under stable, the next
  cursor ID is determined by the saved input, the byte position, and
  the page number, so repeating the same resume returns the same ID;
  a matching record made under any mode is reused. Under fresh, every
  resume mints a new ID. mode= in a record names its creation mode.
  No mode creates a successor when the trailer says cursor: null.

last (recovery):
  -c last selects this working directory's most recently used cursor,
  from a per-directory recovery record in the state directory. Records
  from before this mechanism are matched by their stored directory
  instead. No matching record is an error; a damaged recovery record is
  reported as an error rather than silently selecting an older stream.
  last is resolved again on each call. Other invocations in the same
  working directory and state directory can change its selection,
  including to another stream. Use a printed ID for a fixed position.
  A call that creates no cursor leaves last unchanged, even at exhaustion.
  Stop at cursor: null; another -c last can repeat already-read content.
  Older records without a working directory do not match last;
  they can still be resumed by their printed IDs.

state (first applicable entry wins):
  --state-dir PATH > $MOREOVER_STATE_DIR > $XDG_STATE_HOME/moreover
  > ~/.local/state/moreover
  A cursor requires its record and saved input in the selected directory.
  Keep that state to resume. The original file or producer is not reread.
  Reaching the end does not delete saved state.

exit codes:
  0 success; 1 reported I/O, state, or cursor error; 2 usage or schema error.
  Error messages go to stderr.
```

<!-- surface:contract:end -->

</details>

## Design

Two demonstrations share this small codebase:

- **[ADRs4AI](https://adrs.systems/) — deliberative programming.**
  [`docs/adr/`](docs/adr/) records the questions, alternatives,
  recommendations, and answers behind the tool's design.
  [ADR-0001](docs/adr/0001-the-name-moreover.md) covers the name;
  [ADR-0002](docs/adr/0002-pagination-for-a-reader-without-hands.md) is
  the design space;
  [ADR-0003](docs/adr/0003-the-option-surface-modes-flags-and-where-the-trailer-goes.md)
  is the option surface. The scaffold comes from
  [human-ai-collaboration-template-A](https://github.com/jlumbroso/human-ai-collaboration-template-A).
- **A tool whose primary user is a model.** The default trailer supplies
  a continuation cursor when input remains. An unknown-cursor error
  explains that only printed cursors are valid. The contract describes
  paging, state, and cursor rules without requiring the caller to open
  this README.

Builds on [ThirdX](https://thirdx.design) — "ThirdX (3X) is a design
discipline for model-facing interfaces, after UX and DX —
evidence-based design patterns for the interfaces models consume."
`moreover` is its first command-line tool. Related work:
[Biilmann's Agent Experience (AX)](https://biilmann.blog/articles/introducing-ax/),
[Arcade's Machine Experience Engineering (MX)](https://www.arcade.dev/blog/the-birth-of-machine-experience-engineering/),
[Anthropic's tool-writing guidance](https://www.anthropic.com/engineering/writing-tools-for-agents),
and work on identifiers for model readers by
[Dexter](https://engineering.getdexter.co/2026/03/10/short-ids-for-llms/) and
[BAML](https://boundaryml.com/blog/uuid-swap).

## License

MIT © Jérémie Lumbroso

Built with Ribbon 5 (Claude, Fable 5) and Lector 6 (GPT-6, Astra).
