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
