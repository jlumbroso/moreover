# moreover

> **`more`, `less`, and now `moreover` — a pager for readers who can't press space.**

`moreover` is a **non-interactive pager**: cursor-based pagination for readers
— LLMs, agents, scripts — that consume streams but cannot hold a keyboard.

`more` (1978) taught Unix to pause. `less` taught it to go back. `moreover`
teaches it to **resume** — for the first reader in fifty years who can't
press the space bar.

```console
$ big-output | moreover -10
[first 10 lines]
<moreover: page 1, 10/123 lines, cursor: ae2e>

$ moreover -c ae2e --all
[the remaining 113 lines]
<moreover: 123/123 lines, cursor: null>
```

The trailer line **is** the interface: it lands in the reader's context at
exactly the moment the reader needs to know there is more. A name that means
*"and there is more,"* printed at the point where there is more — the tool
self-documents by being named truthfully.

## Install

```console
$ cargo install moreover
```

Zero dependencies; requires Rust 1.89+. Content goes to stdout; the trailer
goes to stderr, so pipes stay clean. Paged streams are spooled under
`$XDG_STATE_HOME/moreover` (override with `--state-dir` or
`MOREOVER_STATE_DIR`), and a cursor resumes them from any later invocation.
A cursor is only valid if `moreover` printed it.

## Design

Two demonstrations share this small codebase:

- **[ADRs4AI](https://adrs.systems/) — deliberative programming, at
  espresso scale.** Decisions live in [`docs/adr/`](docs/adr/) as
  *Architectural Deliberation Records*: questions, alternatives, reasons,
  and confidence with grounds — not just outcomes.
  [ADR-0001](docs/adr/0001-the-name-moreover.md) is the complete
  deliberation behind the name, killed candidates and all;
  [ADR-0002](docs/adr/0002-pagination-for-a-reader-without-hands.md) is
  the design space;
  [ADR-0003](docs/adr/0003-the-option-surface-modes-flags-and-where-the-trailer-goes.md)
  is the option surface. The scaffold comes from
  [human-ai-collaboration-template-A](https://github.com/jlumbroso/human-ai-collaboration-template-A).
- **A tool whose primary user is a model, designed accordingly.** Fifty
  years of two-letter Unix names optimized for human keystrokes; a pager
  whose reader is a model can afford a whole English word — models pay
  for *ambiguity*, not length.

Builds on model-first tool design (ThirdX), agent-experience prior art
(Biilmann's AX, Arcade's MX), Anthropic's tool-writing guidance, and the
measured result that short stable identifiers cut model reference errors
(Dexter/BAML) — hence the four-character base-32 cursors.

## License

MIT © Jérémie Lumbroso
