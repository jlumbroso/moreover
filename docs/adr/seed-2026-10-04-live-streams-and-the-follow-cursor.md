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

**ANS:** (by Jérémie)
[Fill this in]

---

---

**Model Response Request:**

- [ ] Chunk this into ADRs
- [X] Iterate with me
- [ ] Structure in place
