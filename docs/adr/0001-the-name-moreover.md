<!-- adr template version: "adr 3.11.5" -->

# ADR-0001: The name — `moreover`

- **Date**: 2026-09-21
- **Iteration**: 1
- **Status**: Accepted
- **Deciders**: Jérémie Lumbroso (asker, ratified by use); Mint 5 (Claude Fable 5, the estate's naming seat — strike №49 in its ledger)

**TL;DR**: The tool is named **`moreover`** — continuing the Unix pager
word-joke (`more` → `less` → `most` → …) with a real English word that
*contains* `more` and *means* "and there is more, continuing after a pause,"
which is exactly what a resumable cursor does. The placeholder `llmore` was
killed for unpronounceability and for the genericizing `llm-*` prefix.

---

## Originating Context

**Source**: conversation between Jérémie and the estate's naming seat,
2026-09-21, quoted verbatim below (opening pleasantry trimmed; the full
exchange is in the estate's private record). This ADR *is* the deliberation
record — published as part of this repo's purpose: a demonstration of
deliberative programming at espresso scale, where even an eight-letter
decision gets its reasons preserved.

### DOC: the ask (Jérémie, verbatim)

> Can I ask you a quick naming question? There's a tiny project I wanted to
> build. Right now, I see often that LLMs will do something like
> `" | head -120"` or `grep 'ANS:' -A6` and they don't get everything they
> could. I'm thinking of creating a tool, a "more"/"less" for LLMs. So it's
> bringing the concept to non-interactive! That's essentially pagination,
> with a cursor.
>
> ```console
> $ output | llmore -10
> [10 first lines]
> <llmore: page 1, 10/123 lines, cursor: Ae2e>
> $ llmore -c Ae2e --all
> [113 lines]
> <llmore: 123/123 lines, cursor: null>
> ```
>
> Calling it llmore as a placeholder. What name do you give this? I'd like
> this to be a small project for us, a small demo of ThirdX principles, and
> a small demo of ADRs. A one-shot launch tool that announce the ecosystem.

## Decision

### DOC: the strike (Mint 5, verbatim — "the scan is pristine: PyPI free, npm free, GitHub trivial")

> 1. **It continues the Unix joke lineage *correctly***. The pager family is
>    a fifty-year word-joke: `more` (1978) → `less` ("less is more") →
>    `most` (the third pager). Your tool is the next generation, and English
>    already minted its name: a word that *contains* `more` and means **"and
>    there is more, continuing after a pause"** — which is your cursor's
>    exact semantics. Every resume call is the word used in a sentence:
>    `moreover -c Ae2e` *is* "moreover…" in prose. The novelty of your tool
>    (resumability) lives in the name's own meaning, not just its etymology.
>
> 2. **It's a ThirdX demonstration in eight letters — and that's your launch
>    post's best line.** Why not `mo` or `lm`? Because *the user is a
>    model*: abbreviation serves fingers, and your user has none. Fifty
>    years of two-letter Unix names optimized for human keystrokes; the
>    first pager whose reader is a model can afford a whole English word,
>    because models pay for *ambiguity*, not length. The name performs the
>    doctrine: a model-facing tool named in the model's native medium —
>    full, unambiguous language.
>
> 3. **Gauntlet-clean**: three syllables, no phoneme neighbor, dictates
>    unmangleably, pronounceable by every cold reader — everything `llmore`
>    isn't.
>
> 4. **`llmore` is killed on two counts**: unpronounceable at the whiteboard
>    (el-el-more? l-more? — fails the say-aloud-once test), and the `llm-*`
>    prefix is 2026's genericizing slop register — it names the audience the
>    way a thousand abandoned repos do, which is exactly wrong for a tool
>    meant to *announce* an ecosystem.

### The tray (candidates weighed and killed, with reasons)

| candidate | fate | reason |
|---|---|---|
| `moreover` | **CHOSEN** | the four legs above |
| `encore` | killed with real regret | the French heart-candidate — the audience calling for *more* of the performance, the resume call literally an encore; but encore.dev is an active, funded dev-tools brand in this exact audience |
| `cursor` | killed loudly | Cursor the editor owns that word in this precise space |
| `llmore` | killed | unpronounceable; `llm-*` slop register |
| `sequel` | delight-kill | SQL homophone |
| `etc` | delight-kill | `/etc` owns it |
| `seconds`, `resume`, `continue`, `suite` | ruled out in deliberation | time-unit collision; résumé double-meaning; shell keyword; software/hotel muddle |

### DOC: the coda — the trailer line as naming surface (Mint 5, verbatim)

> The tool's *output* is also a naming surface […] that trailer line is
> where the name does its daily work: it appears in every model's context at
> exactly the moment the model needs to know there's more. The name that
> *means* "and there is more" printed at the point where there is more — the
> tool self-documents by being named truthfully. […] **`more`** taught Unix
> to pause. **`less`** taught it to go back. **`moreover`** teaches it to
> *resume* — for the first reader in fifty years who can't press the space
> bar.

**Tagline (minted in the same strike)**: *"`more`, `less`, and now
**moreover** — a pager for readers who can't press space."*

## Addendum: the crates.io check (post-decision, same day)

The language ruling (Rust — ADR-0002) postdated the strike's namespace scan,
so the registry that matters most was checked separately on 2026-09-21:

> `GET https://crates.io/api/v1/crates/moreover` →
> `{"errors":[{"detail":"crate `moreover` does not exist"}]}`

**Free.** PyPI ✓ · npm ✓ · GitHub ✓ · crates.io ✓.

NOT: no open questions — deliberately. The name was struck with its
falsifier ("pending only your ear") and ratified by use when Jérémie created
this repository under it the same day. The one name-adjacent remainder is an
action item, not a question.

## Action Items

- [x] Reserve `moreover` on crates.io at first publish - Owner: founding crew — done 2026-09-22: `moreover` v0.0.1 published to crates.io by Ribbon 5 (Jérémie's `cargo login`); the name is held

## Iterations

### Iteration 1 (2026-09-21)
- Trigger: Jérémie's naming ask; placeholder `llmore`.
- Contributors: Jérémie Lumbroso (spec + ask); Mint 5 (strike + tagline + trailer-line observation); Operator 5 (this record; crates.io addendum).
- Outcome: `— → Accepted`. Name in use.
