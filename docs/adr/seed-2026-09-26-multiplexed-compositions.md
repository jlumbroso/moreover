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
- [ ] Iterate with me (keep exploring)
- [ ] Structure in place (organize but don't split)

---

## Thread

### [Model Name] - [Date]

[Model's response goes here]

---

**Model Response Request:**

- [ ] Chunk this into ADRs
- [ ] Iterate with me
- [ ] Structure in place

### [Your Name] - [Date]

[Your response if continuing]

---

**Instructions for Model:** Copy everything from "Model Response Request" through the final `---` to the end of your response.

---

## Derived Into

*(This seed has not yet been chunked into ADRs)*

<!-- When this seed is processed, the AI should update this section with:
- ADR-NNNN: Brief title
- ADR-NNNN: Brief title

If anything in the seed remains unprocessed, make a note of what has not been processed yet at the end of this section.
-->
