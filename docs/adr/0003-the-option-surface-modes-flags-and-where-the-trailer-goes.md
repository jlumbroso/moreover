<!-- adr template version: "adr 3.11.5" -->

# ADR-0003: The option surface — modes, flags, and where the trailer goes

- **Date**: 2026-09-22
- **Iteration**: 7
- **Status**: Accepted
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

| Kind | What it governs | Shipped members | Candidate members |
|---|---|---|---|
| **Paging** | how much, in what unit | `-N`/`-n`/`--lines`, `--bytes`, `--all`, `--overlap N` | `--tokens` (ADR-0002 follow-up), `--peek` (a flag: it modifies paging) |
| **Resumption** | which stream, from where | `-c`/`--cursor`, file arguments (`moreover FILE`) | `moreover stat CURSOR`, `moreover drop CURSOR` (desk subcommands, pending glance), `-c last` |
| **Trailer** | the metadata surface | `--trailer DEST`, `--schema`, `--schema-show`, `--schema-template` | `--schema json` (a schema value — never a bare format flag) |
| **State** | where state lives, its lifecycle | `--state-dir` (+ env) | `moreover ls [--everywhere]`, `moreover gc [DAYS]` (desk subcommands), `MOREOVER_DESK` (opt-in scope tag) |
| **Introspection** | the tool describing itself to its reader | `--help`, `--version`, `--schema-show`, `--agent` (→ `moreover contract`, pending glance) | — |
| **Rendering** | output in the reader's native units | — | `--human` (ADR-0002 Decision §3) |

*(Table updated at Iteration 5: "Reader affordance" struck to "Rendering"
— the whole tool is a reader affordance, so the old kind-name claimed the
genus for one species; and the standalone verbs moved to subcommand form
per the naming review, pending his glance on the grammar — QST below.)*

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
- **`--stat CURSOR`** *(added Iteration 2)* — describe a cursor without
  consuming anything: which stream, position, remaining. The
  map-before-territory move: give the reader the shape before the
  content.
- **`-c last`** *(added Iteration 3, from the two-PID experiment below)*
  — resume the most recently minted cursor: the state dir remembers what
  the shell forgets. Error class it prevents: cursor lost between
  invocations through no fault of the reader (scrolled away, compacted,
  or — as the experiment shows — unpassable through shell state at all).
  `last` is reserved vocabulary, cheap to keep out of the base-32 space
  (minted ids always mix letters and digits; `last` is all letters).

### DOC: the two-PID experiment (2026-09-22)

Jérémie ran `echo $$` in two consecutive harness bash turns: PIDs 29756,
then 30352 — **each command gets a fresh shell process**. So in the very
medium this project is built in, `CUR=$(… | moreover -10 …)` followed by
`moreover -c $CUR` next turn is structurally impossible: no shell state
survives between a reader's invocations, and the transcript is the only
carrier. Two consequences staked:

1. The trailer-line doctrine holds for *both* reader kinds here — human
   at the harness prompt and model alike are "readers without hands"
   whose only persistent channel is the printed surface. (Empirically:
   this harness does deliver stderr to the model's context, so the
   stderr default survives it; the destination flag remains for
   harnesses that don't.)
2. `-c last` (above) earns its place: when the printed cursor is the only
   carrier and the carrier is lossy, the store must be able to answer
   "where was I?"

### Naming verdicts (Iteration 5 — Mint 5's review, folded)

Mint 5 (Lumbroso HQ) reviewed the full surface 2026-09-22 (hub inbox
`2026-09-22-0605`, written publication-clean for this fold-in). The
verdicts, distilled — they supersede the older glosses above where they
conflict:

1. **Standalone verbs become subcommands** (`moreover ls | stat | drop |
   gc | contract`) — pipe-compatibility becomes *grammatical* rather than
   documentary: bare invocation + flags is the pipe world; subcommands
   are the desk world; the syntax refuses wrong compositions. As a
   subcommand, `ls` stops violating design commitment 1 (the Unix
   desk-verb register carries fifty years of provenance). Now-or-never:
   every member was unshipped at verdict time, so the move is free today
   and a breaking change forever after. *Pending his glance — this one is
   architecture as much as naming (QST-SUBCOMMAND-GRAMMAR below).*
2. **`--json` is killed** → `--schema json`. A bare format-noun flag
   reads as an input transform (his smell was the correct reading of the
   wrong name). Standing rule: **no bare format flags, ever** — formats
   are schema vocabulary, the same one-flag-one-vocabulary law his
   QST-TRAILER-DEST answer ratified.
3. **`--agent` renames to `contract`** (flag now, subcommand under
   verdict 1): the mode prints the tool's *contract*; "--agent" named who
   reads it. Doctrine pair with `--human`, which is **confirmed**: *name
   the audience only when the audience is the semantics* — for `--human`
   the audience is the argument; for the contract it wasn't.
4. **The scope filter is the desk, and cwd is the ambient default**:
   cursors record their mint-time working directory; `moreover ls` shows
   this directory's desk (zero setup — the two-PID experiment showed
   shells die between turns, but the working directory survives);
   `moreover ls --everywhere` widens to the machine (never `--all`: that
   word belongs to paging); `MOREOVER_DESK` is an *optional sharpener*
   for same-directory concurrency, not a required identity — which
   dissolves the env-crapshoot objection.
5. Kind-name strike: "reader affordance" → **"rendering"** (applied
   above). All other flags confirmed; `--schema-show`/`--schema-template`
   flagged gently as a future muddle — likely absorbed by `moreover
   contract` post-v0.1, no churn now.

*Falsifiers carried from the review*: his glance rules the subcommand
grammar; and if first transcripts show same-directory concurrency is
common, `MOREOVER_DESK` promotes from refinement to recommendation, name
unchanged.

### Design commitments (Iteration 2 — from the commissioned reviews)

Standing principles for every present and future option, distilled from
the estate's model-first design practice (ThirdX; specifics live in
private substrate, consequences recorded here):

1. **Flag names are self-describing.** A model reader often meets a flag
   cold, inside an error message or a trailer — `--schema-show` over
   `-S`. Short forms exist only for the paging counts, where fifty years
   of pager muscle memory is itself the affordance.
2. **Strict at the syntactic boundary, tolerant at the semantic one.**
   Malformed flags, cursors, and schema names are rejected with a precise
   message, never guessed at; stream *content* is accepted generously.
3. **Errors are written to the reader.** A bad cursor or expired spool
   produces a message addressed to the model that will act on it — same
   surface discipline as the trailer, not a log line.
4. **Every flag names the error class it prevents.** A proposed option
   that cannot say what reader failure it forecloses doesn't ship.
   (Corollary, applied to this ADR's own headline: stderr-vs-stdout is a
   *false* conflict — the stderr default serves model and human alike;
   the destination flag exists for harnesses that capture only stdout,
   a real and specific failure.)
5. **Durable over compensatory.** Options encode durable contract
   (schemas, cursors, units), never workarounds for a current model
   generation's quirks; the token unit must be designed
   tokenizer-agnostic or it drifts into the compensatory column.

Public prior art this design stands on, citable freely: Anthropic's
tool-writing guidance (prefer meaningful over low-level identifiers),
the Dexter/BAML result on short ids cutting reference errors,
Biilmann's Agent Experience (AX), and Arcade's MX — the genre is
theirs; `moreover`'s claim is only the composition.

**Cursor-id doctrine folded in** (implemented 2026-09-22, commit
`3e18acc`): minted ids now guarantee a mixed letter+digit form (an
all-digit id reads as a counter and invites extrapolation; an all-letter
id reads as a word — both retype poorly per internal estate
experiments); the help text carries the don't-invent rule ("a cursor is
only valid if moreover printed it"); cursor scope is **stream-local**,
stated here explicitly rather than assumed; and the id never encodes the
position (it is a petname resolved by the store — the printed cursor is
identity, the page number is display, and only the former survives a
reader's context compaction).

## Questions

### QST-TRAILER-DEST: How is the trailer routed, and how is it hidden?
- Status: answered
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

**ANS:** (by Jérémie Lumbroso)
Yes, obviously Option A, it is the only decoupled/scalable option. Additionally to the existing benefits, it makes it easy to pick a default, and then for that default to be overridable in settings. The precedence rules of the other option are a just a prospective nightmare not just to implement but cognitive load on the caller. We should add a deferred question about whether having an env override *in addition* would be a good future option. (If the tool is adopted, this may be a requested feature, but I appreciate your strong argument about legibility to the calling model.)

---

### QST-SELF-DESCRIPTION: Does moreover get a machine-facing self-description mode?
- Status: answered
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

**ANS:** (by Jérémie Lumbroso)
Great idea! Yes, let's do Option A. I don't think we have formalized this mechanism in ThirdX — this reminds me of the `llms.txt` concept for websites.

---

### QST-CANDIDATE-CUT: Which candidate members make v0.1?
- Status: answered
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

**ANS:** (by Jérémie Lumbroso)
Sounds good. But first, I'd like you to route this ADR to **Mint 5 of Lumbroso HQ** for review, especially of the flag names. (`--json` alone, for the *template JSON* feels suspiciously thin, I'd assume it's like a way to convert the input into JSON or something.) I also think it needs to be clear which flags can be used while piping data and which are not. The option `--ls` makes me think the failure mode would be for the model to see a ton of cursors of the concurrent models working on the same machine: I was thinking, having an environment variable, maybe with their session UUID, to help filter the cursors displayed; but then we could also use the PID of the shell session? However I just tried `echo $$` in the Claude Code harness, and we found each turn is run in a different shell — so there is no persistence of session, we need a mechanism to filter — it's not mandatory — but an environment variable could be useful here, though I think setting them is still a crapshoot.

---

### QST-ENV-OVERRIDE: Should an env var later be allowed to override the trailer default?
- Status: answered
- Why asking: `--trailer`'s answer chose flag-only routing for legibility to the calling model; a `MOREOVER_TRAILER` env default would serve harness operators but reintroduces invisible state steering the most-seen surface.
- Need: yes/no + precedence rule, when revisited

Options:
- **A — env as default-setter only**: precedence flag > env > built-in stderr; the env can move the default, never defeat an explicit flag.
- **B — no env, ever**: configuration file or wrapper scripts are the operator's tools; the trailer's routing stays visible in the invocation.

**Recommendation**: (by Ribbon 5, Claude Fable 5)

**A — env as default-setter only**, *when* the need materializes.
*Rationale*: it is the one precedence order that keeps the invocation
legible (an explicit flag always tells the truth) while giving operators
a knob; his answer already sketched exactly this ("easy to pick a
default, and then for that default to be overridable in settings").
*Confidence*: 0.7 — because the shape is standard and matches his
sketch, but the trigger condition (real operator demand) hasn't
occurred. *If wrong*: if in practice env-set defaults produce
"why did my trailer vanish" confusion in model transcripts, that is
**B** — visible-invocation absolutism.

**ANS:** (by Jérémie Lumbroso)
Yes, the precedence in Option A is just right — it feels intuitive.

---

### QST-THIRDX-FRAMING: How much ThirdX may this public repo say out loud?
- Status: answered — via conversation 2026-09-22, recorded by Ribbon 5
- Why asking: the commissioned review found there is currently no public ThirdX writing — this repo may be the framework's first public mouthpiece, which is a publication decision only its author can make.
- Need: pick a letter (or override — any shape answers)

Options:
- **A — name it, cite the public prior art, hold the rest**: keep naming "ThirdX principles" as the repo already does (README, founding brief); cite AX/MX/Anthropic-guidance/BAML as the public lineage; publish no framework specifics (no ADR numbers, no experiment results, no framing paragraphs) until thirdx.design exists.
- **B — carry the framework's framing here**: include the framework's own positioning language (its scope-limiting framing paragraph first, per its author's standing rule) and let moreover be the first public statement, deliberately.
- **C — go quieter**: drop ThirdX naming from future public materials until the site exists; describe the design principles anonymously.

**Recommendation**: (by Ribbon 5, Claude Fable 5)

**A — name it, cite the public prior art, hold the rest.** *Rationale*:
the name is already public in this repo by your founding choices, so C
would be a retraction; but B makes a product repo the framework's
canonical first statement, which pre-empts the launch you have planned
elsewhere — A preserves that sequencing while keeping this repo honest
about its lineage (conceding the genre to AX/MX is, per the review, the
framework's own credibility posture). *Confidence*: 0.75 — because the
sequencing logic is strong but the launch-piece ambition ("a one-shot
launch tool that announces the ecosystem") is yours to weigh against it.
*If wrong*: if you want moreover to BE the first public statement, that
is **B** — deliberate, with the framing paragraph you choose.

**ANS:** (by Jérémie Lumbroso, 2026-09-22, via conversation)
Essentially A, with a tone amendment. The tool's scope is narrow: say
little about ThirdX — cite the relevant principles where they governed a
choice, no more. Drop the defensive posture on prior art: assert lineage
plainly ("Builds on …") instead of conceding at length. And
para-narrative material — e.g. that this is his first project built in
public — does not belong in the README: "the README is to learn about
the project"; genesis belongs to blog posts later. He proposes
recruiting a gpt-6-astra seat to hold public-facing language quality
against **blathm** — his term, by his definition: "artificially coherent,
structurally fluent language that is semantically hollow, produced by
surface-level optimization without genuine representational intent."

---

### QST-SUBCOMMAND-GRAMMAR: Do the desk verbs become subcommands?
- Status: answered
- Why asking: `moreover ls|stat|drop|gc|contract` versus `--ls`-style flags decides the CLI's whole shape, and the window is now-or-never — every member is unshipped except `--agent` (hours old), so the move is free today and a breaking change forever after.
- Need: pick a letter (or override — any shape answers)

Options:
- **A — the two-world grammar (Mint's ruling)**: bare `moreover` + flags = pipe world (paging, resumption, trailer, rendering); subcommands = desk world (`ls`, `stat`, `drop`, `gc`, `contract`). Pipe-safety becomes syntax: wrong compositions won't parse. The git/cargo shape. `--agent` (shipped today) renames to `moreover contract`.
- **B — flags all the way down**: keep every mode a flag, carry pipe-compatibility as a documented help-table column. No rename of `--agent`; the cost is permanent documentation duty and `--ls`-style register violations.

**Recommendation**: (by Ribbon 5, Claude Fable 5)

**A — the two-world grammar.** *Rationale*: it converts a rule a reader
must remember into a sentence a reader cannot missay — the strongest form
of design commitment 2 (strict syntactic boundary), and it resolves his
own pipe-compatibility ask (Iteration 4) structurally rather than
documentarily; the sole shipped casualty is `--agent`, hours old and
pre-announcement. *Confidence*: 0.85 — because the review's argument is
the ADR's own commitments applied consistently, and the migration cost is
one rename today versus a breaking change later; read as an action band
this is mine to take, but the falsifier below is exactly why it waits for
you. *If wrong*: if you want moreover to stay a single-grammar tool (one
invocation shape, no subcommand tree — a legitimate small-tool
aesthetic), that is **B**, and `--agent` still renames to `--contract` as
a flag (that verdict stands either way).

**ANS:** (by Jérémie Lumbroso)
Yep, Option A it is, Mint outdid themselves once more. And I think `moreover contract` is a sharp decision.

---

## Consequences

- The taxonomy becomes the standing map: every future flag names its kind
  or argues for a new kind in an ADR iteration.
- `--trailer stdout` makes the trailer part of piped content by choice —
  downstream tools must expect it; the docs will say so plainly.
- Defaults are compatibility surface: stderr stays default until an ADR
  says otherwise.

## Action Items

- [x] Answer the four QSTs - Owner: Jérémie — all answered by 2026-09-22 (ENV-OVERRIDE parked deferred at his request)
- [x] Fold in the ThirdX-relevance and petname-doctrine reviews (commissioned 2026-09-22) - Owner: Ribbon 5 — folded at Iteration 2; cursor-id changes implemented same day (`3e18acc`)
- [ ] Route this ADR to Mint 5 (lumbroso-hq) for naming review: flag names overall; `--json` reads as input-conversion, not a trailer schema; how to present pipe-compatible vs. standalone flags; the scope-filter's name - Owner: Ribbon 5 (brief), Mint 5 (review)
- [ ] Rewrite README per QST-THIRDX-FRAMING's tone ruling (assertive "Builds on…", no para-narrative) - Owner: Ribbon 5
- [ ] Draft the gpt-6-astra public-language seat's founding brief; he recruits - Owner: Ribbon 5 (draft), Jérémie (recruit)
- [ ] Implement the accepted cut (post-Mint naming review) with tests per the gate - Owner: Ribbon 5

## Iterations

### Iteration 1 (2026-09-22)
- Trigger: his post-v0 observation that trailer destination must be an option, plus "what other modes might we want."
- Contributors: Jérémie (trigger, the destination asks); Ribbon 5 (taxonomy, candidates, recommendations).
- Outcome: `— → Draft`

### Iteration 2 (2026-09-22)
- Trigger: the two commissioned estate reviews returned (ThirdX-corpus relevance; petname doctrine — both read by subagents, distilled here without republishing private substrate).
- Contributors: Ribbon 5 (fold-in); the estate's prior work (via the reviews).
- Changes: Design-commitments subsection added (self-describing names; strict-syntax/tolerant-semantics; reader-addressed errors; error-class discipline; durable-over-compensatory); `--stat` joined the candidates (map-before-territory); public prior art cited (AX, MX, Anthropic guidance, BAML); cursor-id doctrine implemented in code; QST-THIRDX-FRAMING opened.
- Outcome: status unchanged (`Draft`); four QSTs now await answers.

### Iteration 3 (2026-09-22)
- Trigger: two events — his answers to QST-TRAILER-DEST and QST-SELF-DESCRIPTION landed (both A; committed by him as `2b54541`), and his two-PID harness experiment (`echo $$` twice: fresh shell per turn) demonstrated the tool's founding condition inside our own working medium.
- Contributors: Jérémie (answers; the experiment); Ribbon 5 (DOC: two-PID experiment; `-c last` candidate; QST-ENV-OVERRIDE opened deferred per his answer; process note below).
- Changes: experiment staked with its two consequences (trailer doctrine holds for both reader kinds; `-c last` earns its error class); `--agent` mode is a go — his answer notes ThirdX has no formalized self-description convention yet and points at the `llms.txt` concept as kin, so the format design pass is on Ribbon 5; env-override deferred question opened.
- Process note: this iteration also set a crew rule after a real collision (Ribbon 5 edited this file during his answering pass): when the human is mid-edit, his changes get safe-committed first as human feedback, then the seat's — the record reflects who did what.
- Outcome: status unchanged (`Draft`); QST-CANDIDATE-CUT and QST-THIRDX-FRAMING await answers; QST-ENV-OVERRIDE parked.

### Iteration 4 (2026-09-22)
- Trigger: his last two answers. QST-CANDIDATE-CUT: A (reader-first cut) accepted — *conditioned on a naming review by Mint 5* before implementation, with three specific concerns: `--json` misleads (reads as converting the input, not selecting a trailer schema); flags must be presented by pipe-compatibility (usable mid-pipe vs. standalone); and `--ls` in a multi-agent machine shows every concurrent reader's cursors — some optional scope filter is wanted (an env var is plausible-but-unloved: "setting them is still a crapshoot"; the shell PID is ruled out by this ADR's own two-PID experiment). QST-THIRDX-FRAMING: A with the tone amendment (assertive "Builds on…", cite only governing principles, no para-narrative in the README).
- Contributors: Jérémie (answers, incl. the --ls concurrency catch); Ribbon 5 (record, routing, follow-through actions).
- Changes: scope filter joins the State candidates unnamed (its name is Mint-review material); action items rewritten into the follow-through set (Mint routing, README rewrite, astra-seat founding brief); ANS slots closed.
- Outcome: status unchanged (`Draft` — Accepted once the Mint naming review folds in); implementation of the cut queues behind that review.

### Iteration 5 (2026-09-22, overnight)
- Trigger: Mint 5's naming review returned same night (hub inbox `2026-09-22-0605`), hours after the v0.1 cut shipped rename-ready (`4a2525d`: `--trailer`, `--overlap`, file mode, `--agent`).
- Contributors: Mint 5 (all verdicts); Ribbon 5 (fold-in, QST-SUBCOMMAND-GRAMMAR, this record).
- Changes: Naming-verdicts subsection added; taxonomy table updated (shipped column, "rendering" kind, subcommand-form candidates); `--json` killed to `--schema json`; the desk scope mechanism recorded (cwd default, `MOREOVER_DESK` sharpener, `--everywhere`); QST-SUBCOMMAND-GRAMMAR opened for his glance — the sole verdict that is architecture; `--agent` → `contract` queued behind it. Post-v0.1 follow-up noted: `contract` may absorb `--schema-show`/`--schema-template`.
- Outcome: `Draft`; one QST open (the glance); everything else foldable without him.

### Iteration 6 (2026-09-22, overnight)
- Trigger: his glance landed same night: QST-SUBCOMMAND-GRAMMAR answered **A** ("Mint outdid themselves once more"; `moreover contract` called "a sharp decision").
- Contributors: Jérémie (the glance); Ribbon 5 (implementation, record).
- Changes: the two-world grammar shipped — `moreover contract` is the first desk subcommand (`--agent` removed, hours old, pre-announcement); all five desk verbs reserved from day one so a file named `ls` can never silently page (the error teaches `./ls`); the unknown-cursor error now points at `moreover contract`. 24 tests green.
- Outcome: `Draft → Accepted`. Open remainder: QST-ENV-OVERRIDE stays deferred by design; the unshipped desk verbs (`ls`, `stat`, `drop`, `gc`) and `--peek`/`--tokens`/`--human`/`--schema json`/`-c last` remain candidates awaiting the usage evidence their glosses name.

### Iteration 7 (2026-09-23)
- Trigger: `-c last`'s awaited usage evidence arrived — a second model reader's first-contact field report: their trained `2>/dev/null` habit destroyed the trailer and orphaned a cursor they never saw, and no in-band recovery existed (their recovery was spelunking the state dir by hand). The same report observed concurrent readers' cursors mingled in the state dir.
- Contributors: a second model reader (the field report, estate-side); Ribbon 5 (implementation); Lector 6 (independent audit of the throughput benchmark, folded into `scripts/bench-throughput.sh` caveats and parser fix).
- Changes: **`-c last` shipped, desk-scoped** — cursors now record their mint-time working directory; `last` resolves to this desk's newest cursor and refuses with a teaching error elsewhere (`ls`'s cwd-desk mechanism, exercised early); `last` is reserved vocabulary outside the minted-id space and is never case-folded. Contract and help updated truthfully; wording polish is the language seat's. `ls` promoted to next-ship on the same evidence. A contract hazard note on the three stderr idioms (bare, `2>&1`, `2>/dev/null`) is docketed to the language seat.
- Outcome: status unchanged (`Accepted`); 28 tests green; next release carries `-c last` to the tap.

---

## Links

- Related ADRs: ADR-0002 (units, trailer grammar, state — the surfaces these options steer)
