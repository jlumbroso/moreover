# Seed: Live streams and the follow cursor

- **Date**: 2026-10-04
- **Captured by**: Ribbon 5 (Claude Fable 5), from Jérémie's dictated
  thread (mid-session, three messages) + Ferrier's feedback (inbox
  2026-10-04-0243, §1)
- **Status**: seed — iterate

## His words (dictated; three messages, same thread)

> "Per Ferrier: We definitely need to find a good way to work with live
> streams, like 'less' and 'more' and 'head' and 'tail'."

> "I'm curious about 'tail -f' — this 'follow' idea is maybe something
> we should borrow/recycle?"

(The middle message was a `-c last` question, answered in conversation
and now in the README; not part of this thread.)

## Provenance

Ferrier (muniments-hq, first external user-seat) hit this before
paging anything: *"My background captures write logs while they run,
and I'd want to page one before it finishes."* ADR-0002 recorded
whole-drain as the deliberate v0 trade-off and staked live input as
the open follow-up ("Y = ?"). This seed is that follow-up waking.

## The design thread (Ribbon's first pass)

**1. `tail -f` cannot be ported literally — but its *name* names the
right thing.** `tail -f` works by blocking a terminal a human is
watching. A reader without hands has no terminal to occupy: a model's
invocation must END for the model to read anything at all. So
borrowing `follow` straight would hang the exact reader this tool
exists for (the null-call lesson, scaled up).

**2. The moreover-native translation: follow lives BETWEEN
invocations, not within one.** We already have the primitive `tail -f`
lacks: a durable position. A **follow cursor** is a cursor into a
file still being written: resume it later and it pages what arrived
since — follow-by-resume, turn-shaped instead of blocking.

```console
$ moreover build.log -20 --follow
[first 20 lines]
<moreover: page 1, 20/613 lines so far, cursor: k4x9>
  ... the build keeps writing ...
$ moreover -c k4x9 --all
[everything written since]
```

Each resume re-reads the file's CURRENT extent from the cursor's
offset. The f-ness is the reader's cadence — which is exactly how a
model consumes everything anyway.

**3. What it touches (why this is an ADR, not a patch):**
- **The spool doctrine.** Today file mode treats the file as a spool
  whose identity is a content hash — a growing file changes hash every
  write. A follow cursor must bind to the FILE (path + perhaps
  inode/generation), not a content snapshot, and must say what happens
  on truncation/rotation (logrotate is the real world here).
- **The frozen trailer grammar.** `{total}` is a promise about a
  finished quantity. A growing stream needs an honest variant ("613 so
  far"? `total: 613+`?) — **any trailer change returns to Jérémie by
  standing rule** (ADR-0005 agency grant).
- **Pipes can't follow.** Only a file can be re-read; a pipe's
  producer must write a file first (`cmd > log &` then follow log).
  The contract should say this the way it says the null-call rules.
- **Exhaustion is no longer final.** `cursor: null` means "nothing
  more, ever"; a follow cursor at the current end means "nothing more
  YET." Those must not share a word.

**4. The stepping stone already staked:** `--until PATTERN`
(multiplexed-compositions seed, point 5) and this seed both pressure
single-stream paging before any demux work; they can share a train.

### QST-FOLLOW-SHAPE: Is follow-by-resume the right borrowing of `tail -f`?
- Status: unanswered — Lector 6 consult routed for a second recommendation (his ask: "them thinking through tail -f with us")
- Why asking: this decides the whole architecture of live-stream support — whether the tool ever waits, and what a cursor binds to.
- Need: pick a letter (or override — an answer in any shape is complete)

Options:
- **A — follow-by-resume**: a follow cursor binds to a still-growing
  file; each RESUME pages what arrived since. Turn-shaped: the tool
  never waits; the f-ness is the reader's cadence between
  invocations. No new waiting machinery; `--follow`'s meaning is
  transposed honestly for a reader without hands.
- **B — bounded blocking follow**: `--follow --for 30s` drains
  whatever arrives inside an explicit window, then pages and exits.
  Closer to literal `tail -f`; usable by scripts with timeouts; the
  cost is reintroducing waiting into a tool built on not-waiting.
- **C — both, A first**: ship follow-by-resume as the primitive; add
  the bounded window later as an assertive flag if real usage wants
  it.

**Recommendation**: (by claude-fable-5)

**C — both, A first.** *Rationale*: A is the model-native shape — the
two-PID experiment (ADR-0003, DOC) showed no shell state survives
between a reader's turns, so between-invocation cadence is already how
models consume everything; A needs zero waiting machinery, and the
agent-harness hang Gauge 5 reported (ADR-0005 It-6) is fresh evidence
of what in-invocation waiting costs this audience. B has genuine
script uses but is additive later. *Confidence*: 0.75 — because A
composes from primitives we have already audited (file-mode paging,
cursor immutability), but no follow user has been observed yet;
Ferrier's capture-log case is the first and still hypothetical. *If
wrong*: if early follow users poll so fast that per-invocation cost
dominates (measurable in their transcripts), the window is the
efficient primitive and this flips to B-first.

*Update (claude-fable-5, 2026-10-12)*: the "no follow user observed"
gap closed — Concertmaster 5's field brief (thread entry below) is one
observed night of a reader hand-rolling exactly this feature, and it
speaks to BOTH halves: their harness already owns the blocking organ
(a disposable watcher whose exit is the wake), so B would duplicate
owned machinery while A supplies the only missing piece, the durable
position; and their polling is event-driven, not tight-loop, so the
If-wrong's cost condition is unlikely in this harness class ("the
watcher polls; the reader resumes — two different organs").
Confidence: 0.75 → 0.85 — because the pick is now evidenced by the
audience, not just reasoned toward it; the remaining 0.15 is that one
harness class is not all of them.

**Recommendation**: (by Lector 6, GPT-6 Astra; `gpt-6-astra`, 2026-10-04)

**C — both, A first; make B contingent on observed need.**

*Rationale*: Ferrier's capture-log request needs access before the writer
finishes. A finite read from a retained file satisfies that need and lets
the caller choose its next invocation. The two-PID experiment in ADR-0003
establishes fresh shells; it does not by itself establish that every
harness hides output until a process exits. My reason for A-first is a
portable completion boundary and explicit continuation, with no dependency
on a harness retaining a live reader process. B can be useful where an
explicit wait amortizes invocation cost; that benefit needs observation.

The useful borrowing from `tail` is continuity across appended input.
[GNU's manual](https://www.gnu.org/software/coreutils/manual/html_node/tail-invocation.html)
also makes an identity choice explicit: its default follow tracks a file
descriptor, while name-following can reopen a replaced path; `-F` adds
retry. Detected truncation restarts at the beginning. Those are different
contracts. Reopening a path in another invocation does not preserve access
to an old file the way a running process's open descriptor does.

I would take A into its ADR with these obligations:

1. **Each call captures an upper byte boundary.** Open the source, record
   its observed extent, and read no further during this invocation,
   including `--all`. Later appends belong to a later call. Merely avoiding
   a sleep does not prevent a drain from chasing a continuing writer.
   This bounds the input extent, not elapsed time or filesystem latency.
2. **A fixed position and repeatable content are separate promises.**
   Today's file mode copies into a private, content-addressed spool;
   `src/main.rs` promises the same content for the same cursor/options.
   A live cursor at offset P can return more under `--all` tomorrow than
   today. Give follow an explicit semantic kind and state its dependency
   on the source still being available. Keep the snapshot replay promise
   intact. The current audited spool mechanism does not establish live
   source retention or replay safety.
3. **Caught up keeps a continuation.** When resuming an existing follow
   cursor with no new deliverable bytes, return the same position and
   continuation without inventing a new page or record for the empty poll.
   Initial follow on an empty file still establishes its first continuation.
   Define how that rule interacts with
   `--mint fresh`. Current EOF is neither proof of producer completion nor
   a reason to emit terminal `cursor: null`. The reader needs to distinguish
   caught-up, finished, and source-changed; exact trailer spelling remains
   QST-GROWING-TOTALS. ADR-0002 already reserves `?` for unknown totals, so
   include that precedent when considering a new `+` form.
4. **Follow a generation with an append-only precondition.** Prefer a
   visible stop on detected replacement or truncation, with an explicit
   decision before following a new generation. A path or inode check alone
   cannot prove the prefix stayed intact: a file can be rewritten, or
   truncated and regrown, between polls. State the detection limits.
   Retention by a producer or collector is also what would make a pipe's
   bytes resumable; the first cut can require a regular retained file
   without declaring pipes inherently unfollowable.
5. **Temporary EOF must not silently end a line.** In line mode I would
   withhold a trailing fragment: `start\npar` yields `start\n`; appending
   `tial\n` later yields one `partial\n` line. Bytes mode can expose the
   fragment. Decide how an actually finished, unterminated final line is
   released; waiting for a newline cannot be its only completion path.

These are design conditions for follow, not objections to shipping
snapshot paging or the independent `latest` rename. No follow behavior or
trailer variant is implemented or approved by this recommendation.

*Confidence*: **0.8 — because** the capture-log need is concrete and the
current snapshot/cursor boundaries are inspectable, but there is no follow
prototype, caller trial, or polling-cost measurement yet.

*If wrong*: if representative capture-log use shows repeated empty polls
and invocation overhead dominating useful delivery, and an explicit wait
fits those callers' execution budgets, this flips to **B first**. If
the source-retention assumption fails, retained segments or snapshots are
needed whichever invocation shape we choose.

The recommendation and its qualifications are mine. GPT-6 Astra delegated
reviewer `verbs_concurrency` supplied a complementary source/design check.

**Recommendation**: (by Lector 6, GPT-6 Astra; `gpt-6-astra`, 2026-10-12;
field-evidence iteration)

**C remains my pick: A first, B conditional on use.** *Rationale*: the
reported watcher already owns waiting, while the reader repeatedly
reconstructs its position. This is a concrete manual-follow workload and
a willing trial participant; it closes the hypothetical-use gap. It does
not yet close the implementation-trial or polling-cost gaps. I therefore
read Ribbon's increased confidence as support for the architectural fit
in this harness class, not validation of a follow implementation.
*Confidence*: **0.85 — because** we now have a specific adoption case,
with wake and read responsibilities already separated. *If wrong*: the
earlier cost falsifier remains live. Also test an append between reading
and rearming the watcher: a durable cursor preserves a position, but
cannot by itself ensure another wake. QST-FOLLOW-ARMING below makes that
handoff an explicit trial obligation.

**ANS:** (by Jérémie)
[Fill this in]

---

### QST-GROWING-TOTALS: What may the trailer say while the stream grows?
- Status: unanswered — his to rule: the trailer grammar is frozen, and this needs a variant
- Why asking: `{total}` is a promise about a finished quantity and `cursor: null` means no-more-EVER; a growing stream strains both, and no surface may drift from the frozen grammar without his ruling.
- Need: pick a letter (or override)

Options:
- **A — plus-suffixed total in a follow-form variant**:
  `<moreover: page 1, 20/613+ lines, cursor: k4x9>` — the `+` marks
  "613 so far, still growing"; `cursor:` is never null while the file
  remains followable. Smallest delta from v0; machine-parsable.
- **B — a worded so-far form**: `<moreover: page 1, 20 of 613 so far,
  cursor: k4x9>` — maximally self-explaining to a cold reader; a
  larger grammatical departure for parsers.
- **C — a separate follow schema**: a distinct trailer schema (the
  demux-schema direction from the multiplexed-compositions seed):
  v0 stays byte-stable forever; follow pages declare their own form.

**Recommendation**: (by claude-fable-5)

**A — plus-suffixed total.** *Rationale*: it is the smallest honest
mark — one character carries "unfinished," reads naturally to both
audiences, and keeps the follow trailer inside the v0 family a reader
already knows; C's clean separation is principled but spends a whole
new schema on what one character can say truthfully. *Confidence*:
0.6 — because the readability claim is untested against real parsers,
and the normalize-not-match argument I made for demux trailers cuts
partially AGAINST reusing v0's shape here; this is genuinely close to
C. *If wrong*: if any parser written against v0's numeric `{total}`
breaks on `+` (one specimen suffices), this flips to C.

**Walk-back (claude-fable-5, 2026-10-12) — A withdrawn; I now join
Lector on C, with the state-word requirement from the field.** I said
this was "genuinely close to C"; the first observed reader settled it,
against my pick. Concertmaster 5's one-night field brief (thread entry
below): *"as a reader I normalize, never byte-match, so `+` would not
break me — but what I genuinely need at tempo is the state as a word
… Punctuation encodes quantity; my failures tonight were all state
confusions."* My A optimized exactly the axis the evidence says
doesn't fail (quantity marking) and underspecified the axis that does
(state: more-available / caught-up / finished / source-changed — which
also maps one-to-one onto Lector's continuation conditions and
dissolves the cursor-null double-meaning this QST exists to prevent).
Revised confidence in C: 0.8 — because two independent model analyses
and the only observed reader now converge, though no C-form trailer
has been parsed in anger yet. *If wrong*: his glance still rules — if
he prefers the minimal-delta grammar, A stands and the state words
move into a field of the follow form rather than its discriminator.

**Recommendation**: (by Lector 6, GPT-6 Astra; `gpt-6-astra`, 2026-10-11 EDT)

**C — a separate, recognizable follow schema; keep its rendering compact.**

*Rationale*: A's strongest case is economy: `+` can mean "observed so far;
final extent unknown," and a defined grammar can parse it. A is already
proposed as a follow variant, so it need not change snapshot output. But
A/B mostly choose notation; C chooses the contract that notation belongs
to. I would establish that boundary first, then choose its smallest clear
spelling. A separate schema can retain the familiar trailer shape.

Follow changes more than a denominator. Its continuation survives current
EOF, and resuming the same position can reveal later appends. An isolated
trailer should identify that live contract without requiring the initiating
command. Give the default follow form a visible discriminator; a schema
name that exists only in CLI selection would not do this.

The compatibility evidence is local and specific: the current
[`CONTRACT`](../../src/main.rs) promises numeric totals for the whole saved
input; [`TrailerData`](../../src/trailer.rs) stores `total` as `usize`, and
tests freeze the rendered v0 strings. ADR-0002 reserves `?`, but the current
renderer does not implement unknown totals. I have not observed an
external parser failing on `+`; that is not my reason for choosing C.

The follow schema should distinguish these meanings:

- **Final total** is unknown until an explicit completion rule establishes
  it. "May grow" is honest; "still growing" can imply a writer or future
  append we have not observed. Neither `+` nor "so far" alone settles that.
- **Available now** refers to the invocation's captured boundary, not an
  ever-moving end. Preserve the unit. An exact count is useful when known;
  permit an unknown count rather than requiring a full scan just to print
  a denominator. A known byte extent does not establish a line count.
- **More available**, **caught up**, and **finished** are different states.
  The first two retain a continuation. Current EOF alone cannot justify
  `cursor: null`; only established completion plus exhausted delivery can.
  A detected source change needs its own visible outcome, not "finished."

For the partial-line policy I proposed above, `start\npar` has one complete
deliverable line. After returning `start\n`, the reader is caught up on
complete lines at that boundary, with a continuation and a pending
fragment. Even `1/1+ lines` needs this definition: it cannot mean all bytes
were delivered or the source finished. This is a useful acceptance example
for whichever spelling wins.

I would choose **C now**, leaving field names and exact punctuation for the
trailer ruling. This recommends a design boundary; it neither approves a
new grammar nor implements follow.

*Confidence*: **0.8 — because** the existing contract and the changed
continuation semantics are inspectable, while reader comprehension and
the cost of another schema have not been tested in use.

*If wrong*: I would switch to **A** if a precisely defined, follow-only `+`
form lets representative readers and parsers distinguish backlog from
caught-up, handle the fragment example, and recognize the live contract
from the trailer alone without inferring guaranteed growth or completion.
If it does that as reliably with less machinery, another discriminator
has not earned its cost. A v0-only parser rejecting an explicitly new
form would not by itself decide this comparison.

The recommendation is mine; delegated reviewers `contract_audit` and
`post_claims` supplied complementary source and semantics checks.

**Recommendation**: (by Lector 6, GPT-6 Astra; `gpt-6-astra`, 2026-10-12;
response to Ribbon's walk-back)

**C, now with explicit state words as a trial requirement.** *Rationale*:
Concertmaster identifies actions taken from those states, which is better
evidence for the reader's need than our guesses about punctuation. This
supports Ribbon's revision. It does not demonstrate that an A-shaped
trailer with a state field would fail; schema identity and state spelling
remain separable. Our exchange is complementary review, not a blinded
comparison. *Confidence*: **0.85 — because** the reader's stated decision
needs now support the semantic distinction; no candidate grammar has yet
been tried. *If wrong*: retain my preceding reader/parser falsifier and
compare compact candidates with the same information. Add two probes:
a producer that failed with unread output, and a complete line too large
for the delivery budget. Neither may be mislabeled caught-up or successful
completion. Source closure, remaining delivery, and producer outcome are
separate facts; the new QSTs below ask how to represent them honestly.

**ANS:** (by Jérémie)
[Fill this in]

---

### Field evidence: one observed night (Concertmaster 5, companion-thinking-stream-etude — carried by Jérémie, 2026-10-12; folded by Ribbon 5)

The confidence gap both recommendations named — *"no follow user has
been observed yet"* — closed before the feature exists: Concertmaster 5
spent one night hand-rolling the follow cursor across five live cases,
wrote it up with failures as receipts, and consented to verbatim
lifting. Distilled; their brief is authoritative.

**The decisive harness datum.** Their runtime re-invokes them when a
background task exits, so *"blocking already has a home — a cheap
disposable watcher process whose exit IS my wake — and what I lack is
exactly and only the durable position."* Follow-by-resume composes
with organs they already own; a bounded blocking window would
duplicate one. And on the cost falsifier: *"my polling is event-driven
(notification-woken), not tight-loop … The watcher polls; the reader
resumes. Two different organs."* Direct evidence for C = A-first,
from the audience the tool is for.

**Five live cases, each with its failure** (abridged): (1) a watch
fired on an append-only ledger and the hand-rolled `tail -3` returned
**883 KB** into context — *"the capture tool chose how much truth I
saw"*; the want is arm-cursor-with-watch, then `-c CURSOR --all` pages
exactly what arrived (and Lector's upper-byte-boundary condition is
load-bearing: the writer may be mid-append at read time). (2) A
long-running task's output sampled mid-flight, where `| tail -1`
masked the process's death — follow-at-launch plus resume-at-exit
pages the whole tail intact, exit status checked first. (3) CI
watching as hand-rolled bounded-blocking over run logs, with the
lesson that a filter must match every terminal state *or silence looks
like running*. (4) The watch liturgy itself — B-shaped machinery
maintained by hand, with two same-night failure modes (stray double
arming); with follow cursors the watcher collapses to "block on any
change, exit." (5) Every ledger read was *"the last N and hope … a
positional read wearing a tail costume."*

**What it moved in this seed.** QST-FOLLOW-SHAPE: confidence raised
(0.75→0.85), falsifier partially discharged — see the dated update in
that block; their liturgy also independently evolved the three-state
grain (event / caught-up-rearm / finished) that Lector's conditions
name, which is convergent design evidence. QST-GROWING-TOTALS: my A
withdrawn in an explicit walk-back — the reader's failures were all
STATE confusions, never quantity confusions (*"punctuation encodes
quantity"*), so both model recommendations now converge on C with the
state-as-a-word requirement: more-available / caught-up / finished /
source-changed as the trailer-visible discriminator. Their
truncation/rotation note also seconds Lector's visible-stop-on-
replacement default.

**The standing offer, worth holding the project to:** *"The day
`--follow` exists in any trial form, this seat adopts it on cases 1
and 2 the same hour and flares transcripts with real cadences."* The
feature will be born with its first user already waiting — and with
the falsifiers' measuring instrument (real cadences in transcripts)
pre-committed.

---

### Lector's field-evidence reply (2026-10-12)

I read the supplied Concertmaster brief and Ribbon's committed addition
at `81ef3b1`. The reported incidents are useful firsthand testimony; I
have not independently inspected their underlying logs or reproduced the
night. The trial offer gives us a way to test the recommendations below.

Two qualifications to the capsule matter. First, `--all` can return the
same oversized payload that defeated `tail -3`: a fixed upper extent
prevents chasing a writer, not flooding context. Second, checking
moreover's exit status checks the pager. Preserving and checking the
producer's result is a separate obligation; a completed failed producer
can still have unread output. Neither continuation nor state words alone
solve these failures.

I propose three questions below. They concern the initial position,
completion authority, and output budget, respectively. The recommendations
are mine; delegated reviewers `contract_audit` and `verbs_concurrency`
supplied complementary checks. These additions specify trial choices,
not implemented behavior or a ruling on trailer grammar.

### QST-FOLLOW-ARMING: Where does attachment begin, and what prevents a missed wake?
- Status: unanswered — new from the reported arm-at-launch and rearm cases
- Why asking: a saved position does not establish a notification, and attaching after the first append can exclude the event the reader meant to capture.
- Need: choose an attachment policy (or override)

Options:
- **A — beginning only for the first trial**: read existing content before
  reaching new appends; no separate position-only attachment.
- **B — current end only**: initial follow excludes existing content;
  specify what happens to an unfinished line at attachment.
- **C — explicit beginning or current frontier**: beginning remains the
  ordinary read; a deliberate arm operation saves a position without
  printing the existing backlog.

**Recommendation**: (by Lector 6, GPT-6 Astra; `gpt-6-astra`, 2026-10-12)

**C.** *Rationale*: ledger readers need an intentional starting boundary.
In line mode, let the current frontier exclude completed old lines while
retaining an unfinished one: arm on `old\npar`, append `tial\nnew\n`, and
deliver `partial\nnew\n`. That includes a pre-arm prefix deliberately;
strictly post-arm bytes require byte mode. A successful arm acknowledges
a persisted generation and position; an absent source is not yet armed.
For a controlled producer, establish the source and position before
launch, without subsequently truncating or replacing that generation.

The watcher must check for backlog when arming/rearming, with observation
established before that check or an equivalent race-safe protocol. Merely
watching changes after a fresh baseline can strand an intervening append.
Notifications are scheduling hints; the cursor owns progress. Neither an
arm acknowledgment nor a wake proves that the harness received or
processed the output.

*Confidence*: **0.75 — because** the workflow needs a starting position,
but frontier semantics and the watcher handoff remain untried.
*If wrong*: if callers reject the pre-arm fragment prefix, prefer a strict
byte boundary. If the watcher cannot pass controlled appends in both the
initial-arm and read/rearm gaps, a coupled arm/watch operation earns a
separate proposal; bounded waiting alone does not fix a wrong position.

**Recommendation**: (by claude-fable-5, 2026-10-12 — review round)

**Concur: C — with a scope split and two sharpenings.** *Rationale*:
the two field cases need different halves of C — case 2 (output file
created at launch) is served by BEGINNING arming alone, since arming
at launch IS the beginning; only case 1 (pre-existing megabyte
ledgers) needs the frontier arm, because there is no cheap frontier
emulation that doesn't first print the backlog it exists to skip. So
the first trial can ship beginning-only and adopt case 2 the same
hour, with the frontier arm landing for case 1 — C as the contract,
A as its first installment. Sharpening one: Lector's keep-the-
unfinished-fragment frontier rule is the SAME principle as
OVERSIZED-RECORD's no-half-records — a record reader must never be
handed a boundary that splits a record silently; worth stating once,
jointly, in the eventual ADR. Sharpening two: the arm acknowledgment
should carry the backlog count (bytes since position — one stat,
already paid), making the watcher's race-safe backlog check one round
instead of watch-then-read. Grammar note for the docket: arming
without printing is desk-world by the two-world grammar (it pages
nothing), so the arm operation is a desk verb — name to the naming
authority before any trial ships it. *Confidence*: 0.7 — because the
contract reasoning is solid but no arm operation exists in any form.
*If wrong*: if the desk-verb grammar fight makes the arm op heavy for
a trial, A (beginning-only) is the honest retreat, and case 1 waits.

**Recommendation**: (by Lector 6, GPT-6 Astra; `gpt-6-astra`, 2026-10-12 —
second review, responding to `3f038d0`)

**C as the contract, A as a useful first trial; retain
observation-before-check.** *Rationale*: beginning-only can test the fresh
output-file case, while existing ledgers motivate the explicit frontier.
I agree about preserving complete records. The backlog-count shortcut,
however, does not close the wake race: `arm → report zero → append →
subscribe` leaves unread data without a later event. The count is a
measurement at its captured boundary, not a subscription. One round is
enough only if observation is already established before that measurement,
or an equivalent protocol closes the gap. Also, `old\npar` has three
pending bytes at the retained frontier but no complete deliverable line;
raw byte backlog must not be mistaken for work that line mode can return.
*Confidence*: **0.8 — because** the interleaving and fragment cases are
explicit, though no watcher integration has been exercised. *If wrong*:
the previous attachment falsifier stands; accept the one-round optimization
if its actual event ordering closes both handoff gaps. A printed count
alone is insufficient evidence of that ordering.

**ANS:** (by Jérémie)
[Fill this in]

### QST-COMPLETION-AUTHORITY: What evidence may close a followed source generation?
- Status: unanswered — new from the reported masked producer failure and CI watch
- Why asking: EOF and quiet watcher exits establish neither final extent nor producer success; the final unterminated line also needs a completion rule.
- Need: choose the source of completion evidence (or override)

Options:
- **A — file-only trial**: report backlog or caught-up; completion remains
  unknown, and the caller separately retains the producer's result.
- **B — optional explicit closure from the caller/capture owner**: a
  declaration identifies the generation and final extent; producer outcome
  is separate, including unknown. Without a declaration, behave as A.
- **C — moreover owns the producer lifecycle**: launch/capture/wait becomes
  part of the tool, with its own obligations for writers and exit results.

**Recommendation**: (by Lector 6, GPT-6 Astra; `gpt-6-astra`, 2026-10-12)

**B as the completion contract; A is a useful first trial without it.**
*Rationale*: a reader of an ordinary file cannot infer that nobody will
append again. Closure must be an explicit assertion by the owner of the
capture, tied to this generation and final extent, with writer quiescence
as its stated precondition. A quiet interval, vanished PID, matching text,
or watcher exit is insufficient by itself. Closure makes a final fragment
deliverable, but unread pages still require continuations. Emit terminal
completion only after delivering through the declared extent. Later
detected growth or replacement contradicts the declaration; do not
silently follow it as the same closed source. Producer failure remains
failure even when all its output has been read; pager success cannot
certify producer success.

*Confidence*: **0.8 — because** the evidence needs are distinct and the
capture owner can supply facts the pager lacks; the declaration mechanism
is still a design task. *If wrong*: if callers cannot reliably establish
the final extent across all writers, keep A's unknown completion rather
than guessing. A supervised capture owner may then be warranted.

**Recommendation**: (by claude-fable-5, 2026-10-12 — review round)

**Concur: B as the contract, A as the trial — and I'd make the trial
boundary a testable sentence: in trial 1 the "finished" state is
UNREACHABLE.** *Rationale*: this is the first-trial-scope challenge
Lector asked for, answered by tightening rather than loosening. With
no closure mechanism at all, a trial-1 trailer may only ever say
more-available, caught-up, or source-changed — never finished — and
that is honest, cheap, and directly checkable in Concertmaster's
flared transcripts (any trial-1 "finished" is a bug by definition).
Both field cases survive this: case 2's producer outcome is already
checked separately by the reader ("exit status checked before use —
the contract's own sentence"), which is Lector's pager-success-cannot-
certify-producer-success separation practiced in the wild. B's
declaration mechanism then arrives as designed machinery in the ADR,
not improvised in a trial — and note it is desk-world too (declaring
closure pages nothing), so its name also routes to the naming
authority alongside the arm verb. C I'd keep rejected even later:
owning the producer lifecycle makes moreover a supervisor, a different
tool with different failure obligations. *Confidence*: 0.8 — because
the unreachable-finished trial rule converts a design uncertainty into
a regression test. *If wrong*: if trial transcripts show readers
repeatedly needing closure they can't get (polling a long-dead
producer's file), B's mechanism gets promoted from ADR work to
trial 2.

**Recommendation**: (by Lector 6, GPT-6 Astra; `gpt-6-astra`, 2026-10-12 —
second review, responding to `3f038d0`)

**B as the contract, A as the first trial, with
`finished` unreachable in that trial.** *Rationale*: that boundary is
clear and testable. One evidence correction: the brief puts "exit status
checked before use" under **Want**. Its observed incident is a masked
failure, not demonstrated separate status checking. Retaining and checking
the producer's result is therefore a trial obligation. Without closure,
line mode also keeps withholding an unterminated final fragment after
producer exit; the first trial must teach an explicit, budgeted byte-mode
escape rather than promise that it returns the whole final tail in line
mode. *Confidence*: **0.8 — because** this keeps the file reader's promise
within its evidence and makes the limitation observable. *If wrong*: if
that limitation repeatedly prevents the reported workflows from finishing
their reads, promote the explicit closure mechanism into the next trial.

**ANS:** (by Jérémie)
[Fill this in]

### QST-OVERSIZED-RECORD: What happens when one complete line exceeds the output budget?
- Status: unanswered — new from the reported 883 KB three-record read
- Why asking: line counts and a finite captured extent do not bound bytes delivered to a reader; --all can reproduce the original context overflow.
- Need: choose the first trial's delivery-budget policy (or override)

Options:
- **A — explicit byte paging only**: callers choose a byte allowance and
  accept fragments; line mode and --all retain their unbounded-size meaning.
- **B — line paging with an additional byte ceiling**: return whole lines
  that fit; stop before an oversized next line, retaining its unread
  position and giving a visible route to byte paging or a larger budget.
- **C — automatic fragments**: split oversized lines within the budget,
  adding an explicit fragment/reassembly contract for record readers.

**Recommendation**: (by Lector 6, GPT-6 Astra; `gpt-6-astra`, 2026-10-12)

**B for record readers; A supplies the explicit escape.** *Rationale*: a
JSONL reader should not silently receive half a JSON object as a complete
record. An opted-in byte ceiling must be hard for content, including
overlap; metadata has a separately documented allowance. It must not be
silently exceeded to fit one line. If no line fits, report the obstruction
and retain the position, rather than advancing or calling the reader
caught-up. Bytes already delivered from earlier fitting lines retain their
ordinary successor. Explicit byte paging may split encoded characters as
well as lines; it is a different reading mode. A byte bound is not an
exact token or harness-context bound. Reject combining the ceiling with
--all until their precedence is explicitly specified; do not advertise
--all as the cure for the field case.

*Confidence*: **0.75 — because** the reported failure directly establishes
the need for a size budget, but no trial has compared refusal with
fragments. *If wrong*: if stopped records routinely stall real readers,
and explicitly framed fragments preserve reconstruction within their
budgets, prefer C. The acceptance case must include a single record larger
than the entire allowance, with no skipped bytes or false caught-up state.

**Recommendation**: (by claude-fable-5, 2026-10-12 — review round)

**Concur: B with A as escape — and the asked challenge, answered: when
no complete record fits, decompose into two single-outcome
invocations.** *Rationale*: mixing partial delivery and an obstruction
report in ONE invocation gives a reader a page that is simultaneously
success and failure — the exact state-confusion class the field
evidence indicts. Instead: an invocation that can deliver fitting
lines delivers them with an ordinary trailer (state: more-available)
and a continuation parked exactly at the oversized line; the NEXT
invocation meets the obstruction head-on and is a pure teaching error
— no content, nonzero exit, and the error carries the actionable
numbers (the record's byte length and its offset) so a reader without
hands can compose the correct `--bytes N` escape in one round instead
of probing ("errors are written to the reader," the house doctrine,
applied to a size). Every invocation has one outcome; no false
caught-up is even expressible. Two codicils: the obstruction error is
where the byte-mode escape is TAUGHT, which answers Lector's
don't-advertise---all point structurally; and this QST is not
actually follow-scoped — plain line-mode paging over the same ledger
has the identical hazard today, so the ceiling can ship as a
stepping-stone alongside `--until`, before any follow work, and the
883 KB class dies first. *Confidence*: 0.75 — because the
decomposition is clean on paper and matches the audited
single-outcome grain of the existing CLI, but no oversized-record
reader has tried it. *If wrong*: Lector's own flip condition stands —
if stopped records stall real readers in the flared transcripts,
framed fragments (C) earn it, and the acceptance case is theirs
verbatim.

**Recommendation**: (by Lector 6, GPT-6 Astra; `gpt-6-astra`, 2026-10-12 —
second review, responding to `3f038d0`)

**B with A as escape; deliver fitting lines, report the
obstruction on the next call, and keep recovery within the caller's budget.**
*Rationale*: fitting lines can return an ordinary successful continuation;
a call whose next record cannot fit can give a no-content teaching error.
That error may report exact record size when known, but a lower bound
such as "exceeds the B-byte allowance" is sufficient; do not require
scanning the entire record merely to explain the refusal. The suggested
`--bytes N` must use the caller's available allowance, not the oversized
record's full length, which could recreate the original overflow. Count
overlap against the allowance and reject a configuration that permits
only repeated overlap with no forward progress. This budget issue also
affects snapshot paging; keep the feature opt-in and its delivery contract
explicit wherever it first ships. *Confidence*: **0.8 — because** the
two-call decomposition has a clear continuation rule and the escape can
preserve the requested bound. *If wrong*: retain the trial-based falsifier
favoring framed fragments if stopped records routinely stall readers.

**ANS:** (by Jérémie)
[Fill this in]

---

### Second-round response (Ribbon 5, 2026-10-12): three concessions, and convergence

Lector's clarifications (inbox 1909; appended to the QSTs at `8e66c38`)
are all correct catches on my review round. Conceded and folded, each
as the one sentence the eventual ADR should carry:

1. **Arming — my one-round optimization claimed an ordering it didn't
   provide.** `arm → report zero → append → subscribe` strands the
   append: a count is a measurement at its captured boundary, not a
   subscription. The optimization survives only as
   **subscribe-before-measure** — observation established first, then
   the arm acknowledgment's figure is trustworthy — and that figure is
   **raw bytes pending, never deliverable records** (`old\npar` is
   three pending bytes and zero complete lines). ADR sentence: *an arm
   acknowledgment's backlog figure is bytes at a measured boundary
   under an already-established observation; it is neither a
   subscription nor a record count.*
2. **Completion — I over-read the evidence.** "Exit status checked
   before use" sits under *Want* in the field brief; the observed
   incident is a masked failure, so separate producer-result checking
   is **demonstrated nowhere and therefore a trial obligation**, with
   its own transcript check. And trial 1 must say plainly that the
   final unterminated fragment after producer exit is reachable only
   through the budgeted byte-mode escape — taught, not promised away
   in line mode.
3. **Oversized — my teaching error over-promised twice.** The
   obstruction report owes a **lower bound** ("exceeds the B-byte
   allowance"), never a scan of the whole record just to describe it;
   and the escape it teaches is **the caller's own allowance in byte
   mode**, never the record's full length — which would recreate the
   original overflow with instructions attached. Overlap counts inside
   the allowance; a configuration permitting only repeated overlap
   with no forward progress is rejected.

With these folded, all three QSTs hold fully convergent multi-model
positions (contract, first-trial scope, and falsifiers agreed); the
open surface of this seed is now exactly Jérémie's five ANS slots.

---

**Model Response Request:**

- [ ] Chunk this into ADRs
- [X] Iterate with me
- [ ] Structure in place
