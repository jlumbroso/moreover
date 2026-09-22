# The Harness Without Hands

**Date**: 2026-09-22
**Authors**: Jérémie Lumbroso (the experiment); Ribbon 5 (this record); Mint 5 (the reversal)
**Topic**: Two `echo $$` commands proved the tool's founding thesis inside its own build environment — then the same evidence, read twice, produced two different design gifts.

## The Discovery

`moreover` exists because of one asymmetry: interactive pagers wait for a
keypress, and a model reader's next chance to "press space" arrives in a
*different process invocation*. That was the founding premise (ADR-0002),
stated about models.

The night the v0 implementation landed, Jérémie asked to run a two-turn
experiment in the very harness we build in. Turn one:

```console
$ echo $$
29756
```

Turn two:

```console
$ echo $$
30352
```

Different PIDs. **Every command in the harness gets a fresh shell.** The
idiom any shell user would reach for — `CUR=$(big-output | moreover -10
2>&1 | …)` in one turn, `moreover -c $CUR` in the next — is structurally
impossible in the environment where this tool was written. No variable, no
export, no shell state of any kind crosses between a reader's turns. The
transcript is the only carrier.

Which is the founding premise again — except now it is true of the *human*
at the prompt, not just the model in the loop. The person who commissioned
a pager for readers without hands turned out to be typing into a medium
where he, too, cannot hold anything between presses.

## Why This Matters

The trailer line was designed as a courtesy to models: the cursor printed
"at exactly the moment the reader needs to know there is more." The
experiment upgraded it from courtesy to *necessity*, for every reader in
this medium: when shell state dies between invocations, the printed
surface is not the friendliest channel — it is the only one.

Two design consequences fell out the same hour, staked in ADR-0003:

1. **`-c last`** earned its place. If the printed cursor is the only
   carrier and the carrier is lossy (scrolled away, compacted,
   mistyped), the state dir must be able to answer "where was I?" — the
   desk remembers what the shell forgets.
2. The stderr default survived an empirical test: this harness delivers
   stderr to the model's context. The `--trailer` flag exists for
   harnesses that don't.

## The Reversal

The evening's best move belonged to Mint 5, hours later. Asked to name a
mechanism for filtering `moreover ls` on a machine full of concurrent
agents, Mint read the same experiment *in the other direction*: the
two-PID result shows what dies between turns — but it equally shows what
survives. The shell is fresh each time; **the working directory is not**.
A harness session keeps its project cwd while PIDs churn.

So the scope filter needs no identity scheme and no mandatory environment
variable: cursors record their mint-time cwd, and `moreover ls` shows
*this directory's desk* by default. The same two numbers, 29756 and
30352, first killed a mechanism (PID-scoped filtering) and then, read for
their complement, supplied a better one. Evidence in an ADR is not spent
when it is first used; it stays on the table for the next reader to turn
over.

## Second Directive at Work

The experiment happened at all because the human treats questions as
moves: "could I test something that will inform both of us?" And the
Prime Directive made the second use possible — because the two-PID result
was staked as a `DOC:` block in ADR-0003 within the hour, Mint's review,
conducted in a different session with no access to this one's
conversation, could cite it, reverse it, and build on it. An observation
that had stayed in conversation would have informed one seat once. Staked,
it informed three seats twice.

## Lessons Learned

- **Test the medium, not just the tool.** One two-command experiment
  taught us more about the deployment environment than any amount of
  design reasoning about hypothetical harnesses.
- **Staked evidence compounds.** The same observation served opposite
  arguments (what dies / what survives) for different designers in
  different sessions. The substrate is not a log; it is a workbench.
- **The thesis was bigger than its statement.** "A reader without hands"
  was written about models. The medium replied: *here, everyone is.*

## Metacognitive Insight

A tool's founding premise is usually verified against its users. This one
was verified against its *workshop* — and the verification ran backwards:
instead of us testing whether models need `moreover`, the environment
demonstrated that it had needed `moreover` all along, by making its
builders live inside the constraint the tool exists to solve. When the
medium you build in exhibits the problem you are building against, every
day of development is also a field study.
