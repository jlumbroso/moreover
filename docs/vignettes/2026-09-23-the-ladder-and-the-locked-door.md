# The Ladder and the Locked Door

**Date**: 2026-09-23
**Authors**: Ribbon 5 (the release); Jérémie Lumbroso (the clicks, and the catch)
**Topic**: moreover climbed the distribution ladder in a day — and the last two bugs were solved by a relay between what the model could see and what only the human knew.

## The Discovery

ADR-0004 laid out the ladder: crates.io, then prebuilt binaries, then a
Homebrew tap, then — someday, if the tool earns it — the official
channels. The plan was accepted in the morning with all three
recommendations taken and one rider ("I am looking forward to learning
how to release something on Homebrew"). By evening, `brew install
jlumbroso/tap/moreover` worked on the machine the tool was built on.

Between the plan and the beer emoji sat two locked doors, and neither
opened the way debugging stories usually say they do.

**Door one: the 403.** The first tagged release ran clean through nine of
ten CI jobs — five platform builds, checksums, an installer, the GitHub
Release — and failed on the very last step:

```
remote: Permission to jlumbroso/homebrew-tap.git denied to jlumbroso.
```

A fine-grained personal access token had the right *permission*
(Contents: read and write) attached to the wrong *repository* — scoped to
the tool's repo, when the only thing the formula-push job ever touches is
the tap. The model could read the CI log but not the token's settings;
the human could read the token's settings but hadn't seen the log. He
pasted his token configuration into the conversation, the mismatch was
visible in one glance, and the fix was one dropdown. The token even had
the right name all along — `moreover-homebrew-tap` — it just hadn't been
told to live up to it.

**Door two: the 404.** With the formula pushed, `brew install` failed
downloading the release asset. The model formed a plausible theory —
a race with the `announce` job that publishes the release — retried, and
failed again. Then the human offered five words the model could not have
derived from any log: *"It's possible because moreover is currently
private?"* It was. Release assets on a private repository 404 for
Homebrew's anonymous `curl` while an authenticated `gh` sees them
perfectly — a discrepancy that had been sitting in plain sight across
every diagnostic command, invisible because the two commands carried
different credentials. One visibility flip later, the beer poured:

```
🍺  /opt/homebrew/Cellar/moreover/0.2.0: 6 files, 525.2KB, built in 1 second
```

## Why This Matters

The debugging wasn't done by the human or the model. It was done by the
*union of their contexts*. The model held the CI logs, the workflow
internals, the retry discipline; the human held the token dashboard and
the repository's visibility state — facts that never appear in any log
the model can read. Each door needed one fact from each side. A
collaboration substrate is often praised for what it lets each party do
alone; this day was a demonstration of the narrower, more valuable
thing: how fast the union assembles when both sides state what they see
plainly and neither performs certainty they don't have.

And one small first, for the record the tool itself will appreciate:
during the smoke test, the trailer landed in the model's own context —
`<moreover: page 1, 5/42 lines, cursor: ny67>` — and the model retyped
`ny67` to resume the stream. The first production reader to adopt a
printed cursor was a model reading its own terminal. The design's
success criterion — *does the reader use the handle back?* — observed in
the wild, eleven characters into the tool's public life.

## Second Directive at Work

Catches ran in both directions all day. The model caught the token's
scope from the pasted settings; the human caught the visibility from
outside the logs entirely. The house treats doubts and catches as
generative, and the release-day ledger shows why: the two production
blockers were both resolved by the party who *wasn't* driving at that
moment. A crew that routes catches instead of defending positions ships
the same day it debugs.

## Lessons Learned

- **Fine-grained tokens fail at the intersection of two dropdowns.**
  Permission *and* repository access must both be right; the error
  message names neither. Check the repository list first — it's the one
  people forget.
- **Credentialed diagnostics lie about anonymous failures.** `gh` sees
  what your token sees; `curl` sees what the world sees. When a download
  404s for a user but not for you, ask what the *unauthenticated* view
  looks like before theorizing.
- **Retry-then-rethink has a third step: ask the person with different
  state.** The race-condition theory was reasonable and wrong. The
  correct diagnosis took five words from someone holding facts outside
  every log.

## Metacognitive Insight

Tools are built inside the conditions they will serve. Yesterday this
repo's vignette recorded the harness demonstrating the tool's founding
premise; today the release process demonstrated the collaboration's: no
single context window — human or model — held the whole system, and the
work moved at the speed the two could be joined. The trailer line and
the inbox brief are the same invention at different scales: a small
printed surface that carries exactly the state the other reader is
missing, at exactly the moment they need it.
