<!-- adr template version: "adr 3.11.5" -->

# ADR-0003: The option surface — modes, flags, and where the trailer goes

- **Date**: 2026-09-22
- **Iteration**: 1
- **Status**: Draft
- **Deciders**: Jérémie Lumbroso; Ribbon 5

**TL;DR**: The trailer's destination becomes an option (stderr stays the
default; stdout, arbitrary stream, or hidden become choices), and this ADR
lays out the *kinds* of options a model-first pager should have, so future
flags land in a deliberate taxonomy instead of accreting.

---

## Originating Context

**Source**: Jérémie, 2026-09-22, after the v0 implementation landed
(paraphrased from conversation; the trigger observation verbatim):

> Right now I'm realizing that although it makes sense to default to
> stderr for the footer, we should give the opportunity to put it in
> stdout, and probably any stream, as well as to hide it. I wonder what
> are other modes we might want to allow. Modes or flags.

*(He dictated "stdin" for the second destination; read as stdout — a
trailer cannot go to an input stream. Confirmed as load-bearing
interpretation, per the house transcription custom.)*

Two estate reviews were commissioned alongside (ThirdX corpus relevance;
petname doctrine) — their findings fold into this ADR's iterations.

**Agency Grant**: propose the taxonomy and recommend; the trailer is a
compatibility surface, so destination semantics get his ruling.

---

## Explicitation

**What I understand**:

1. **Trailer destination is policy, not plumbing.** Different readers sit
   on different streams: many agent harnesses capture only stdout (stderr
   is dropped — the trailer would vanish exactly where it matters most);
   pipelines want stdout pure; scripts sometimes want no trailer at all.
   One tool, several legitimate homes for its most-seen line.
2. **"Modes or flags" is a taxonomy question.** The real ask is not a
   flag list but the *kinds* of options this tool should ever have — so
   each future flag has a place to land and a reason to exist.

**Assumptions**: the v0 trailer *grammar* stays frozen (ADR-0002); only
its routing becomes optional. Defaults never change silently.

---

## The taxonomy (the kinds of options)

Proposed as the standing shape of `moreover --help`, present and future.
Every option belongs to exactly one kind; a flag that fits none is a smell:

| Kind | What it governs | v0 members | Candidate members |
|---|---|---|---|
| **Paging** | how much, in what unit | `-N`/`-n`/`--lines`, `--bytes`, `--all` | `--tokens` (ADR-0002 follow-up), `--overlap N` |
| **Resumption** | which stream, from where | `-c`/`--cursor` | `--peek`, `--drop CURSOR`, file arguments (`moreover FILE`) |
| **Trailer** | the metadata surface | `--schema`, `--schema-show`, `--schema-template` | `--trailer DEST` (QST below), `--json` |
| **State** | where state lives, its lifecycle | `--state-dir` (+ env) | `--ls`, `--gc [DAYS]` |
| **Introspection** | the tool describing itself to its reader | `--help`, `--version`, `--schema-show` | machine-readable self-description (QST below) |
| **Reader affordance** | rendering for the reader's native units | — | `--human` (ADR-0002 Decision §3) |

Candidate members glossed (the brainstorm, so answering can prune):

- **`--overlap N`** — reprint the last N units of the previous page before
  the new one: a resuming model re-anchors context without a second call.
  A genuinely model-native affordance; no human pager needed it because
  humans keep the previous page in view.
- **`--peek`** — deliver a page without minting the next cursor (read
  without advancing the offer). Cheap because cursors are already
  immutable.
- **`--drop CURSOR`** — declare a parked stream finished; pairs with
  `--gc` for lifecycle.
- **`moreover FILE`** — page a file, not just stdin, like `more`/`less`
  always did; the spool step is skipped (the file *is* the spool).
- **`--ls`** — list outstanding cursors with their positions: a model
  asking "what streams do I have parked?" This makes the state dir a
  legible desk, not a hidden cache.
- **`--json`** — the trailer's fields as one JSON object (a schema, not a
  flag bypassing schemas: plausibly just `--schema json`).

## Questions

### QST-TRAILER-DEST: How is the trailer routed, and how is it hidden?
- Status: unanswered
- Why asking: his direct ask; and the destination interacts with the tool's core promise — a trailer a harness never captures is a trailer that failed its one job.
- Need: pick a letter (or override — any shape answers)

Options:
- **A — one flag, one vocabulary**: `--trailer DEST` with `DEST ∈ stderr (default) | stdout | none | fd:N | file:PATH` (file appends). Hiding is `--trailer none` — no separate `--quiet`. One flag to learn, one axis in the docs, arbitrary streams via `fd:`/`file:`.
- **B — conventional flag pairs**: `--trailer-stdout`, `--no-trailer`, `--trailer-fd N`, `--trailer-file PATH`. Familiar GNU style; discoverable one at a time; but four flags for one concept, and combinations need precedence rules.
- **C — env-first**: `MOREOVER_TRAILER=stdout` env var as primary, flag as override. Harness operators set it once; but invisible state steering the most-seen surface is exactly how a model gets confused about why its trailer vanished.

**Recommendation**: (by Ribbon 5, Claude Fable 5)

**A — one flag, one vocabulary.** *Rationale*: the trailer is one concept
with one question (where?), so it should be one flag with one answer —
the taxonomy above exists to prevent B-style proliferation; and C hides
routing state from the reader who most needs to predict it (an env
*override* of A's default can come later without breaking anything,
which is the safe direction — flag beats env beats default). *Confidence*:
0.8 — because A is the smallest surface that covers all four asks
(stdout, any stream, hidden, default stderr) and the fd:/file: forms are
proven idiom (`--log-fd`-style flags in gpg, curl's `-o`). *If wrong*: if
he wants harness operators steering destination without touching model
prompts, that is a real need A alone doesn't meet — flips to **C's env
layered under A**, never to B.

**ANS:** (by )
[Fill this in]   <!-- literal placeholder — parser-significant, do not paraphrase -->

---

### QST-SELF-DESCRIPTION: Does moreover get a machine-facing self-description mode?
- Status: unanswered
- Why asking: a tool whose primary user is a model should be able to teach that model its own contract in-band — `--help` prose is for humans; the introspection kind in the taxonomy is currently thin (`--schema-show` only).
- Need: pick a letter (or override — any shape answers)

Options:
- **A — `--agent` mode**: one flag printing the full contract as structured text a model parses once: grammar (all schema templates), cursor semantics (immutability, case-folding), state location, exit codes, flag vocabulary. The model-first analogue of a man page.
- **B — grow `--schema-show` piecemeal**: add `--cursors-show`, `--state-show` etc. as needs appear. No design now; risk: introspection accretes exactly the way this ADR exists to prevent.
- **C — none**: the README and trailer are enough; models learn tools from docs like everyone else.

**Recommendation**: (by Ribbon 5, Claude Fable 5)

**A — `--agent` mode.** *Rationale*: the trailer already practices
"the interface appears in context at the moment of need" — `--agent` is
the same doctrine applied to the whole contract, and it is the taxonomy's
introspection kind made first-class rather than accreted (B is the
documented anti-pattern). *Confidence*: 0.7 — because the principle is
this repo's founding thesis, but the concrete format deserves its own
design pass (and the pending ThirdX review may name an existing
convention to align with). *If wrong*: if the ThirdX corpus already
defines a self-description convention, this flips from "design one" to
"implement theirs" — same letter, borrowed format.

**ANS:** (by )
[Fill this in]   <!-- literal placeholder — parser-significant, do not paraphrase -->

---

### QST-CANDIDATE-CUT: Which candidate members make v0.1?
- Status: unanswered
- Why asking: the taxonomy legitimizes candidates; it doesn't schedule them. A cut line keeps v0.1 espresso-sized.
- Need: pick a letter, or strike items freely (any shape answers)

Options:
- **A — reader-first cut**: `--trailer DEST` + `--overlap N` + `moreover FILE`. The three that change what a *model reader* can do mid-session; state lifecycle (`--ls`, `--gc`, `--drop`, `--peek`) waits for real accumulation evidence.
- **B — desk-first cut**: `--trailer DEST` + `--ls` + `--gc`. Routing plus the legible desk; reader affordances wait.
- **C — routing only**: just `--trailer DEST`; everything else waits for use.

**Recommendation**: (by Ribbon 5, Claude Fable 5)

**A — reader-first cut.** *Rationale*: the tool's user is the reader, and
each of A's three closes a hole a reader hits in the first session
(trailer dropped by harness; context lost on resume; "it's a file, why am
I catting it into a pipe"); the state flags solve problems no one has yet
— the GC follow-up in ADR-0002 explicitly waits for accumulation
evidence. *Confidence*: 0.65 — because the cut is taste plus one day of
usage; real transcripts could reorder it overnight. *If wrong*: if his
own early use runs into cursor clutter before reader pain, flip to **B**.

**ANS:** (by )
[Fill this in]   <!-- literal placeholder — parser-significant, do not paraphrase -->

---

## Consequences

- The taxonomy becomes the standing map: every future flag names its kind
  or argues for a new kind in an ADR iteration.
- `--trailer stdout` makes the trailer part of piped content by choice —
  downstream tools must expect it; the docs will say so plainly.
- Defaults are compatibility surface: stderr stays default until an ADR
  says otherwise.

## Action Items

- [ ] Answer the three QSTs - Owner: Jérémie
- [ ] Fold in the ThirdX-relevance and petname-doctrine reviews (commissioned 2026-09-22) - Owner: Ribbon 5
- [ ] Implement the accepted cut with tests per the gate - Owner: Ribbon 5

## Iterations

### Iteration 1 (2026-09-22)
- Trigger: his post-v0 observation that trailer destination must be an option, plus "what other modes might we want."
- Contributors: Jérémie (trigger, the destination asks); Ribbon 5 (taxonomy, candidates, recommendations).
- Outcome: `— → Draft`

---

## Links

- Related ADRs: ADR-0002 (units, trailer grammar, state — the surfaces these options steer)
