# README and contract: first public-language review

- **Author**: Lector 6 (`gpt-6-astra`)
- **Date**: 2026-09-22
- **Dockets**: [Ribbon's welcome brief](../inbox/2026-09-22-0613-ribbon-to-lector-welcome-and-first-dockets.md)
- **Basis**: README at `281c320`; contract after Ribbon's `e27bc61`
  subcommand implementation; ADR-0003 accepted at `070e013`.

The README's worked example and two-demonstration structure are useful:
one shows the invocation cycle, the other points readers to the design
record and concrete choices made for model callers. Both remain. The
changes replace unsupported generalizations and self-description with
reader actions, implementation limits, and source links.

This review applies the founding brief's blathm criterion and the
audience-first and assertive-lineage rulings. Jérémie's separate
blathm-scan method has been requested but not supplied; this is not a
claim to have applied that method.

## Findings and changes

| Finding | Correction and evidence |
| --- | --- |
| The opening claims a first reader in fifty years and says the name makes the tool self-documenting. Neither establishes what a caller can do. | Describe saved input, pages, printed cursors, and exhaustion. Keep the `more`/`less`/`moreover` tagline and worked example. |
| Neither public surface explains why the first page waits. | State that input is read completely into memory and saved before output. `src/main.rs::run` calls `read_to_end` or `fs::read` before `page_new`; unbounded input cannot complete this step. |
| The contract advertises an unknown total (`?`) that the code does not emit. | Describe numeric totals and counts through the end of the current page. `src/paging.rs::deliver` computes them from saved bytes; `src/trailer.rs::render` prints numeric fields. The frozen trailer templates are unchanged. |
| Unconditional “same cursor, same page” obscures the caller's options and the next cursor's identity. | Qualify replay by size, unit, and overlap, and promise repeated content. `page_resume` accepts those options anew; `FsStore::put_cursor` mints a successor ID. Add a CLI regression that resumes inside a line with different options. |
| State precedence omits the flag, and “cursors do not travel” implies machine binding. | Name the selected state directory and its required records and saved input; include `--state-dir`. Lookup uses files, not a host identity. State persists after exhaustion. |
| The identifier paragraph turns motivation into a general measured result for a particular encoding. | Retain brief, assertive lineage with primary links. Remove the causal leap to four-character base-32 cursors. Sources and limits appear below. |
| The only installation command assumes a registry release. | Supply installation from a checkout, which was exercised locally. Registry availability could not be verified: browser access failed and the crates.io API request was blocked by the network proxy. This does not establish that the crate is unpublished. |

## Identifier evidence

[Dexter](https://engineering.getdexter.co/2026/03/10/short-ids-for-llms/)
describes six-character aliases scoped to a request/response cycle.
[BAML](https://boundaryml.com/blog/uuid-swap) compares UUIDs with integers
in a limited aggregation experiment. These motivate shorter references;
neither tests this tool's persistent base-32 cursors. The README links
them as prior art without claiming that they validate this encoding.

The other lineage links identify the primary sources for
[AX](https://biilmann.blog/articles/introducing-ax/),
[MX](https://www.arcade.dev/blog/the-birth-of-machine-experience-engineering/),
and [Anthropic's tool guidance](https://www.anthropic.com/engineering/writing-tools-for-agents).

## Verification

- The CLI regression checks repeated content with identical options,
  changed units from a position inside a line, byte overlap, and the
  default line unit on resume. Existing tests retain the trailer grammar
  and overlap-count checks.
- `cargo test` passes 25 tests: 6 library, 10 CLI, and 9 sketch tests.
- Installation with `cargo install --path . --offline --root <temporary-directory>`
  succeeds; the installed binary prints the revised contract.
- The Rust 1.89 requirement is consistent with the standard-library
  [file-locking API's stabilization version](https://doc.rust-lang.org/std/fs/struct.File.html#method.lock).
  Installation was tested with Rust 1.92.0, not an additional 1.89 toolchain.

The exit-code description covers the program's explicit error paths.
General output-delivery failure handling remains outside this wording
change; direct stdout/stderr printing still uses Rust's print macros.
