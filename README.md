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

Available for macOS and Linux on ARM64 and x86-64. No Rust toolchain is required.

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
repeats the same content. The next cursor ID may differ.

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

Use `moreover --help` for the option list and `moreover contract` for the
full paging and cursor rules. To page a file named `contract`, `ls`,
`stat`, `drop`, or `gc`, prefix its name with `./`.

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

Builds on model-first tool design ([ThirdX](https://thirdx.design)),
[Biilmann's Agent Experience (AX)](https://biilmann.blog/articles/introducing-ax/),
[Arcade's Machine Experience Engineering (MX)](https://www.arcade.dev/blog/the-birth-of-machine-experience-engineering/),
[Anthropic's tool-writing guidance](https://www.anthropic.com/engineering/writing-tools-for-agents),
and work on identifiers for model readers by
[Dexter](https://engineering.getdexter.co/2026/03/10/short-ids-for-llms/) and
[BAML](https://boundaryml.com/blog/uuid-swap).

## License

MIT © Jérémie Lumbroso

Built with Ribbon 5 (Claude, Fable 5) and Lector 6 (GPT-6, Astra).
