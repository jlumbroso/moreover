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
- Status: unanswered
- Options: **A — follow-by-resume** (above: turn-shaped, no blocking,
  fits the reader; the name `--follow` is recycled with its meaning
  transposed honestly). **B — bounded blocking follow** (`--follow
  --for 30s`: drain-what-arrives-in-a-window; closer to tail -f,
  usable by scripts with timeouts, but reintroduces waiting into a
  tool built on not-waiting). **C — both, A first.**
- **Recommendation**: C with A first — A is the model-native shape and
  needs no new waiting machinery; B has real script uses but can
  arrive later as an assertion flag without breaking A.
- Confidence: medium-high. Falsifier: if Ferrier-class users turn out
  to poll so fast that A's per-invocation cost dominates, B's window
  becomes the efficient primitive and deserves the lead.

### QST-GROWING-TOTALS: What may the trailer say while the stream grows?
- Status: unanswered — **his to rule; trailer grammar is frozen**
- The tension: `{total}` must not lie, and `cursor: null` must not
  mean two things. Candidate shapes for his glance, not a ruling:
  `20/613+ lines`, `so far: 613`, or a distinct follow-form trailer
  the way `--all` already has its own form.
- **Recommendation**: a `+` suffix on total in a follow-form variant
  (smallest honest mark, reads naturally, machine-parsable), with
  `cursor:` never null while the file remains followable.
- Confidence: medium. Falsifier: if downstream parsers written against
  v0 choke on `+`, a separate follow schema (like the demux schema
  direction) is the honest cost.

---

**Model Response Request:**

- [ ] Chunk this into ADRs
- [X] Iterate with me
- [ ] Structure in place
