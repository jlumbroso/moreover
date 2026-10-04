# Roadmap

Where every staked idea stands. Each line links to where the thinking
lives (the ADRs and seeds are the substance; this file is the index).
Updated when trains ship or ideas move; last pass 2026-10-04.

## Shipped — v0.4.0 (2026-10-04)

Stable cursor ids with per-desk recovery (`--mint stable|fresh`,
`-c last`); the lifecycle verbs `ls`/`drop`/`gc` under the
mapping-preservation constraint; bounded-read resume; the agent-harness
null-call guard. Audited through seven iterations —
[ADR-0005](docs/adr/0005-the-cursor-lifecycle-mint-modes-the-desk-verbs-and-equality.md),
[ADR-0006](docs/adr/0006-bounded-read-resume-benchmark-gated.md).

## Next train — 0.4.1 (docket open)

| Item | Origin | State |
|---|---|---|
| Retry an initial `Interrupted` read within the stdin deadline | auditor's nonblocking follow-up (ADR-0005 It-8) | spec'd |
| Contract: "wait for EOF; for a live log, page a copy" | Ferrier's feedback, adopted verbatim | spec'd |
| Contract: worked example of `--overlap` on byte pages (UTF-8 split) | Ferrier's feedback | spec'd |
| `MOREOVER_DESK` — named desks within one directory | designed in ADR-0003 (naming verdict 4); **promoted** by Ferrier's parallel-captures report, exactly the falsifier's trigger | designed, promoted |

## In design — seeds iterating

| Thread | Where | Waits on |
|---|---|---|
| **Live streams / the follow cursor** — follow-by-resume as the honest `tail -f` borrowing; touches spool identity, pipes-can't-follow, and the trailer | [seed 2026-10-04](docs/adr/seed-2026-10-04-live-streams-and-the-follow-cursor.md) | iteration; QST-GROWING-TOTALS is the maintainer's (trailer grammar is frozen) |
| **Multiplexed compositions** — demux `grep -A`-style output into per-substream cursors; `--until PATTERN` as the stepping stone; inline trailers get their own schema (normalize-not-match recommended) | [seed 2026-09-26](docs/adr/seed-2026-09-26-multiplexed-compositions.md) | one checkbox: the maintainer's nod to chunk |
| **The reporting pipeline** — `feedback --kind bug\|wish\|note`, the capture-receipt conformance rule | [seed 2026-09-26](docs/adr/seed-2026-09-26-the-reporting-pipeline-standard.md) | schema draft (Ribbon's deliverable, next large block) |

## Designed, waiting for evidence or a dependency

| Item | Where staked | Trigger |
|---|---|---|
| `stat CURSOR` — describe without consuming (map before territory) | ADR-0003 (reserved verb) | usage evidence |
| `--peek` — a page without minting the next cursor | ADR-0003 | usage evidence |
| `--schema json` — trailer fields as one JSON object | ADR-0003 (naming: no bare format flags) | a consumer that wants it |
| `--tokens` / `--human` rendering | ADR-0003 | usage evidence |
| Config home + settings-level `--mint` default | ADR-0003/0005 | the config-home design (wizard dependency) |
| `calibrate` — the adoption wizard, measure-don't-guess | ADR-0003 lineage; ruled "empirical" | config home |
| Benchmark graduation: in-harness output verification, acceptance rule, failure exit status | ADR-0006 | next perf-touching change |
| Follow-form / demux trailer schemas | both seeds | their ADRs |

## Launch surfaces (after the code settles)

Announcement post (carries the frozen ThirdX definition line, which the
README already does); companion sandbox site. Timing is the
maintainer's call.

---

*The two authoritative interface descriptions — `moreover --help` and
`moreover contract` — live in the binary and are spliced into the
README by `scripts/readme-sync.py`; a test fails the gate if they
drift. This file deliberately contains no interface text at all.*
