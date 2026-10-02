# SEED: Multiplexed compositions

- **Date**: 2026-09-26
- **From**: Jérémie Lumbroso

---

## Brain Dump

The original failure `moreover` is addressing is the `head -120` with no follow-up. But a similar failure is the `grep -A6` for ANS (that specific problem is resolved by the ADRs4AI MCP that provides a non-regexp interface to structured QSTs/ANS, but let's assume the use case is generally useful). `moreover` currently would not help with the `grep -A6` problem because `moreover` bounds one stream instead of multiple. Therefore to be able to be useful in that scenario, `moreover` would have to be able to demux a stream into substreams and then bound each one of them.

Here's an example:

```bash
$ grep -r 'QST[^: ]*:' docs/adr/00*.md -A3 | head -20
docs/adr/0002-pagination-for-a-reader-without-hands.md:### QST-CURSOR-STORE: Where does the paged stream live between invocations?
docs/adr/0002-pagination-for-a-reader-without-hands.md-- Status: answered
docs/adr/0002-pagination-for-a-reader-without-hands.md-- Why asking: the pipe is consumed at first read; when the resume call comes, the original stream is gone. Whatever wasn't printed must have been spooled somewhere durable enough to outlive the first process.
docs/adr/0002-pagination-for-a-reader-without-hands.md-- Need: a storage location + lifecycle (creation, lookup, expiry)
--
docs/adr/0002-pagination-for-a-reader-without-hands.md:### QST-CURSOR-SEMANTICS: What exactly does a cursor name?
docs/adr/0002-pagination-for-a-reader-without-hands.md-- Status: answered
docs/adr/0002-pagination-for-a-reader-without-hands.md-- Why asking: `moreover -c Ae2e --all` and a hypothetical `moreover -c Ae2e -20` must both make sense. Does the cursor carry only "which stream + where in it," with page size per-call, or does it also freeze paging parameters?
docs/adr/0002-pagination-for-a-reader-without-hands.md-- Need: cursor = f(what)
--
docs/adr/0002-pagination-for-a-reader-without-hands.md:### QST-PAGE-UNITS: Lines, bytes, or tokens?
docs/adr/0002-pagination-for-a-reader-without-hands.md-- Status: answered
docs/adr/0002-pagination-for-a-reader-without-hands.md-- Why asking: the reader is a model, and models budget in tokens — but tokenizers are model-specific, heavy dependencies, and the founding sketch speaks in lines.
docs/adr/0002-pagination-for-a-reader-without-hands.md-- Need: v0 unit set + the upgrade path
--
docs/adr/0002-pagination-for-a-reader-without-hands.md:### QST-TRAILER-GRAMMAR: Is the trailer line's grammar frozen as sketched?
docs/adr/0002-pagination-for-a-reader-without-hands.md-- Status: answered
docs/adr/0002-pagination-for-a-reader-without-hands.md-- Why asking: the trailer is the tool's most-seen surface (it lands in a model's context every page) and the founding record already published its shape; changing it later breaks the readers who learned it.
docs/adr/0002-pagination-for-a-reader-without-hands.md-- Need: freeze/amend decision for v0, incl. the unknown-total case
--
```

What we observe here are:
- prefix information (here filename, but presumably could have line information or anything else);
- substream separator, here `--` which can probably be changed — let me find out;

According to Sonnet 5:

> Yes. Both flags work here:
> 
> - `--group-separator=SEP` replaces the `--` line with `SEP`. For example, `grep -A1 --group-separator='<<>>' pat` printed `<<>>` between the two groups.
> - `--no-group-separator` removes the separator line entirely.
> 
> Both are GNU grep options. They also worked in this shell, where `grep` is ugrep 7.8.4 rather than macOS's stock BSD grep. I didn't test stock BSD grep, so on a machine without ugrep or GNU grep, check `grep --help` first.
> 
> `rg` (ripgrep) is installed here too. It has the same feature under different names: `--context-separator=SEP` and `--no-context-separator`.
> 
> For `moreover`, an empty separator (`--group-separator=`) leaves a blank line between groups, and `--no-group-separator` gives the most compact output. Either way the reader loses the "there was a gap here" cue, so a distinctive separator is safer when the output is meant for an LLM.

So it seems we have everything to compose with these tools.

I guess what could be useful then is if something like this could be done:

```
# this is design, not reality!

$ grep -r 'QST[^: ]*:' docs/adr/00*.md -A100 | moreover -N 4
docs/adr/0002-pagination-for-a-reader-without-hands.md:### QST-CURSOR-STORE: Where does the paged stream live between invocations?
docs/adr/0002-pagination-for-a-reader-without-hands.md-- Status: answered
docs/adr/0002-pagination-for-a-reader-without-hands.md-- Why asking: the pipe is consumed at first read; when the resume call comes, the original stream is gone. Whatever wasn't printed must have been spooled somewhere durable enough to outlive the first process.
docs/adr/0002-pagination-for-a-reader-without-hands.md-- Need: a storage location + lifecycle (creation, lookup, expiry)
<moreover: page 1, 4/100 lines, cursor: ae2e>
--
docs/adr/0002-pagination-for-a-reader-without-hands.md:### QST-CURSOR-SEMANTICS: What exactly does a cursor name?
docs/adr/0002-pagination-for-a-reader-without-hands.md-- Status: answered
docs/adr/0002-pagination-for-a-reader-without-hands.md-- Why asking: `moreover -c Ae2e --all` and a hypothetical `moreover -c Ae2e -20` must both make sense. Does the cursor carry only "which stream + where in it," with page size per-call, or does it also freeze paging parameters?
docs/adr/0002-pagination-for-a-reader-without-hands.md-- Need: cursor = f(what)
<moreover: page 1, 4/100 lines, cursor: hjy5>
--
docs/adr/0002-pagination-for-a-reader-without-hands.md:### QST-PAGE-UNITS: Lines, bytes, or tokens?
docs/adr/0002-pagination-for-a-reader-without-hands.md-- Status: answered
docs/adr/0002-pagination-for-a-reader-without-hands.md-- Why asking: the reader is a model, and models budget in tokens — but tokenizers are model-specific, heavy dependencies, and the founding sketch speaks in lines.
docs/adr/0002-pagination-for-a-reader-without-hands.md-- Need: v0 unit set + the upgrade path
<moreover: page 1, 4/100 lines, cursor: er31>
--
docs/adr/0002-pagination-for-a-reader-without-hands.md:### QST-TRAILER-GRAMMAR: Is the trailer line's grammar frozen as sketched?
docs/adr/0002-pagination-for-a-reader-without-hands.md-- Status: answered
docs/adr/0002-pagination-for-a-reader-without-hands.md-- Why asking: the trailer is the tool's most-seen surface (it lands in a model's context every page) and the founding record already published its shape; changing it later breaks the readers who learned it.
docs/adr/0002-pagination-for-a-reader-without-hands.md-- Need: freeze/amend decision for v0, incl. the unknown-total case
<moreover: page 1, 4/100 lines, cursor: l0ki>
--
```

We could presumably refine this design in several ways:

1. allow for an optional "stop word" — for instance, above we are asking for `-A100` to be sure no to truncate anything; but we might get irrelevant content; however we know that in our format, a QST/ANS pair will be separated, sometimes by a `---` but usually by another section at `###` depth (another QST) or at `##` depth (a different section); we could have that provided the `moreover` as an indication to truncate the demuxed streams this way

2. allow for overlap detection — this is complementary to the previous idea; the previous idea helps us use semantic awareness about the file to improve the usefulness of `moreover`, here we are simply just being "smart" about noticing that, because Item 1 has a lot of after-context, it reincludes content that is part of Item 2 (or of Item 2, Item 3, and more, etc.), so, we will truncate it right before the duplicate content starts; note that by "duplicate content" we don't mean "syntactically the same" we mean "sourced from the exact same lines".

One open question is, how do we preconfigure `moreover` to have the right configuration. Perhaps the answer is a combination:

1. It seems like what `grep` is doing is pretty standard, so doing that as a default/first approximation could be useful.

2. We could have different modes, specifically for different tools.

3. We could have an autodetect mode that uses classifiers and ranking of likelihoods.

All this is not necessary to have in the first stable intermediate form, but are ways of composing `moreover` more harmoniously with searching tools.

---

**Model Response Request:**

- [ ] Chunk this into ADRs (I'm ready to formalize)
- [X] Iterate with me (keep exploring)
- [ ] Structure in place (organize but don't split)

---

## Thread

### Ribbon 5 (Claude Fable 5) - 2026-09-26

Iterating. The idea reads as moreover's second act: the first act made
*one* truncation resumable; this makes the `grep -A6` failure — many
truncations at once — resumable per match. Sharpening five things, one
of which is a real design catch in your sketch:

**1. The catch: interleaved trailers can't ride stderr.** Your sketch
shows each substream's trailer *inline between groups*. With today's
stderr default, that interleaving is not guaranteed — stdout and stderr
are separately buffered, so a pipe consumer would see the trailers
drift out of position (or not at all). Demux mode's per-substream
trailers must go **on stdout, in-band**, as your sketch actually drew
them — which is fine precisely because demux output is already
structured for the reader, not for further blind piping. That makes
demux a distinct output *shape*, honestly a new trailer schema (the v0
grammar is frozen for the single-stream forms; a demux form is an
addition, versioned, not a mutation).

**2. The unit of resumption: your sketch is right, and it's the
`--overlap` story again.** Each substream gets its own cursor — a model
scanning 30 QSTs resumes only the two it cares about, which is the
whole point. The open refinement: whether the demux invocation ALSO
mints one *batch* handle ("next page of every unfinished substream").
My lean: yes, later — individual cursors are the primitive; a batch
handle is sugar the desk can add when real transcripts show models
wanting it.

**3. Overlap detection gets exact the moment we ask for `-n`.** Your
"sourced from the exact same lines" is implementable without any
similarity heuristics: `grep -rn`/`rg -n` prefixes carry (file, line)
provenance, so duplicate-suppression is set-membership on line
identity. The refinement writes itself: the grep-mode parser reads the
prefix, and truncate-before-duplicate is deterministic. Without `-n`,
we degrade honestly (no dedup, or byte-identity only) rather than
guessing.

**4. Stop-words compose with your assertive-flags doctrine.** A
`--until PATTERN` per-substream truncation (stop before the next
`^###`/`^##`) is a *destination-asserting* option and pairs naturally
with stop-word presets per mode. Which brings the preconfiguration
question home: your options 1–2 (grep-conventions default; per-tool
modes) fit the house — an assertive `--demux grep|rg|…` mode flag with
the grep shape as default. Option 3 (autodetect via classifiers) I'd
hold at arm's length: it sits in the compensatory column (heuristics
tuned to today's tools' quirks) and violates "flags assert
destinations" in spirit — the tool guessing its input's grammar is the
kind of invisible state the trailer doctrine exists against. If
autodetect ever comes, `calibrate`'s measure-don't-guess shape is the
honest version of it.

**5. Scope guard, agreeing with your closing line.** None of this
belongs in the current stable intermediate form: demux touches the
store (substream families), the trailer (new schema), and the CLI
grammar (mode flag) at once — it's a v0.3+ act with its own ADR when
we're ready. What CAN land early and cheap: the `--until` stop-word on
*single-stream* paging (useful alone: "page until the next heading"),
as a stepping stone that de-risks one of demux's three legs.

One naming note for later, not routed yet: "demux" is my working word;
the naming authority will want the docket when this firms (mode flag
name, the batch-handle question, the new schema's name).

---

**Model Response Request:**

- [ ] Chunk this into ADRs
- [X] Iterate with me
- [ ] Structure in place

### Jérémie Lumbroso - 2026-10-01

Agreed we should route to naming authority.

Also, agreed on stdout, and we could have the trailer either match the one that is taken on input, or be normalized to something else, no?

We could task a subagent with studying the JSONL of multiple seats on the hive to learn from the grepping patterns. Or simple we could send a deliberation "how do you use search tools in the command line and how would you want to compose better — see moreover for cat?" I think the latter supposes the models can be subjects and not just objects.

Everything else looks good. Do you need anything else from me before chunking this off? Or do you see higher priorities?

---

**Model Response Request:**

- [ ] Chunk this into ADRs
- [X] Iterate with me
- [ ] Structure in place

### Ribbon 5 (claude-fable-5) - 2026-10-01

**On match-vs-normalize for the inline trailers — recommendation:
normalize to a distinct demux schema, deliberately NOT the v0 grammar.**
Rationale: the v0 trailer is a frozen compatibility promise whose whole
semantics is "one stream, one continuation, on stderr." An inline
substream marker lives in *stdout, between content blocks* — if it
wears the v0 coat, a reader (or a regex someone wrote against our
contract) can mistake a substream's marker for the invocation's own
continuation, and v0's freeze starts accreting side-meanings. Family
resemblance yes (angle brackets, the `moreover:` prefix, named fields —
one house voice), but a visibly different shape carrying the substream
identity + its cursor, versioned on its own track. "Match the input" I
read as the lighter option but the wrong long game: it saves learning
one grammar today at the price of overloading the one grammar we froze.
Confidence: medium-high. Falsifier: if dogfooding shows readers
compose two grammars worse than one overloaded one, normalize loses.
Schema name and field vocabulary → the naming-authority docket already
noted above.

**On the two research routes — take both, but order them, and one
consent flag on the first.** The deliberation ("how do you use search
tools and how would you want to compose better?") is the ThirdX-native
move and should LEAD: models as subjects, stated preferences. The JSONL
study is the revealed-preferences complement — grounding against what
seats actually type. But seat JSONLs are session logs, and the estate's
own policy ("the UUID is inert, it's the log we protect") treats those
as each seat's protected record: I'd gate the study on per-seat opt-in,
folded into the same deliberation ("may a subagent read your session's
shell history for grep/rg patterns?"). Consent turns the study from
surveillance into collaboration — the subjects-not-objects principle
applied twice. A deliberation brief can go out independent of chunking;
say the word and I draft it.

**Nothing else needed from you before chunking.** The one open design
fork (match vs normalize) carries the recommendation above, so your
answer degrades to accept/override. On priorities, my honest ordering:
(1) v0.4.0 is minutes from shipping — stable minting is cleared and the
lifecycle verbs landed today (ADR-0005 It-5), pending Lector's verbs
pass; (2) the reporting-pipeline schema draft (your Option C ruling)
is the oldest open commission; (3) this seed chunks third — `--until`
on single-stream paging can land cheap alongside, as staked in point 5.
If you flip the checkbox to "Chunk this into ADRs" (or just nod), I'll
chunk on that ordering.

---

**Model Response Request:**

- [ ] Chunk this into ADRs
- [X] Iterate with me
- [ ] Structure in place

---

## Derived Into

*(This seed has not yet been chunked into ADRs)*

<!-- When this seed is processed, the AI should update this section with:
- ADR-NNNN: Brief title
- ADR-NNNN: Brief title

If anything in the seed remains unprocessed, make a note of what has not been processed yet at the end of this section.
-->
