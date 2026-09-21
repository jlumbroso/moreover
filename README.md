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
<moreover: page 1, 10/123 lines, cursor: Ae2e>

$ moreover -c Ae2e --all
[the remaining 113 lines]
<moreover: 123/123 lines, cursor: null>
```

The trailer line **is** the interface: it lands in the reader's context at
exactly the moment the reader needs to know there is more. A name that means
*"and there is more,"* printed at the point where there is more — the tool
self-documents by being named truthfully.

## Status

**Day one, building in public.** This is the first of
[Jérémie Lumbroso](https://github.com/jlumbroso)'s projects built in the open
from its first commit: the design deliberations are as public as the code,
and they came first. Implementation language: **Rust** (see
[ADR-0002](docs/adr/0002-pagination-for-a-reader-without-hands.md)).

## Why this repo looks the way it does

Two demonstrations share this small codebase:

- **[ADRs4AI](https://adrs.systems/) — deliberative programming, at espresso
  scale.** Decisions live in [`docs/adr/`](docs/adr/) as *Architectural
  Deliberation Records*: questions, alternatives, reasons, and confidence
  with grounds — not just outcomes.
  [ADR-0001](docs/adr/0001-the-name-moreover.md) is the complete deliberation
  behind the name, killed candidates and all. The scaffold comes from
  [human-ai-collaboration-template-A](https://github.com/jlumbroso/human-ai-collaboration-template-A).
- **A tool whose primary user is a model, designed accordingly.** Fifty years
  of two-letter Unix names optimized for human keystrokes; a pager whose
  reader is a model can afford a whole English word — models pay for
  *ambiguity*, not length.

## License

MIT © Jérémie Lumbroso
