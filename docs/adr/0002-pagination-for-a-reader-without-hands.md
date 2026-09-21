<!-- adr template version: "adr 3.11.5" -->

# ADR-0002: Pagination for a reader without hands — the design space

- **Date**: 2026-09-21
- **Iteration**: 1
- **Status**: Draft
- **Deciders**: Jérémie Lumbroso (rulings quoted below); the founding crew (recommendations to be answered)

**TL;DR**: `moreover` paginates arbitrary streams for readers that cannot
press a key: page out, trailer line with a cursor, resume by cursor in a
later invocation. Implementation is **Rust** (his ruling). Four design
questions are open below, each with a recommendation, so answering degrades
to accept/override.

---

## Originating Context

The one design question, as the naming deliberation put it: **how does a
reader without hands turn the page?** Interactive pagers hold the stream and
wait for a keypress; `moreover`'s reader gets its next chance to "press
space" in a *different process invocation*, possibly minutes later. So the
pause must become a **cursor**, and the held stream must survive between
invocations.

### DOC: the spec sketch (Jérémie, verbatim, 2026-09-21)

> ```console
> $ output | llmore -10
> [10 first lines]
> <llmore: page 1, 10/123 lines, cursor: Ae2e>
> $ llmore -c Ae2e --all
> [113 lines]
> <llmore: 123/123 lines, cursor: null>
> ```
>
> That's essentially pagination, with a cursor.

*(The tool has since been named `moreover` — ADR-0001 — so the trailer reads
`<moreover: …>`.)*

### DOC: the language ruling (Jérémie, verbatim, 2026-09-21)

> For io reasons, I think the tool should be in Rust.

### DOC: the trailer-line doctrine (from the naming strike — why the trailer is load-bearing)

> that trailer line is where the name does its daily work: it appears in
> every model's context at exactly the moment the model needs to know
> there's more.

## Questions

### QST-CURSOR-STORE: Where does the paged stream live between invocations?
- Status: unanswered
- Why asking: the pipe is consumed at first read; when the resume call comes, the original stream is gone. Whatever wasn't printed must have been spooled somewhere durable enough to outlive the first process.
- Need: a storage location + lifecycle (creation, lookup, expiry)

**Recommendation**: (by Operator 5, Claude Fable 5)
**A — spool files under `$XDG_STATE_HOME/moreover/` (fallback
`~/.local/state/moreover/`), one file per stream; the cursor id is a short
prefix of the spool's content hash.** *Rationale*: XDG *state* (not *cache*)
semantics match — caches may be evicted while a cursor is outstanding;
spool-per-stream keeps resumption O(seek); no daemon, trivially inspectable
with ordinary tools. *Confidence*: medium-high — the alternatives fail
structurally (re-reading the pipe is impossible; a daemon is heavy for a
tool this size). *If wrong*: if cursors ever need to work across machines or
users, a content-addressed spool with an explicit export subcommand replaces
the directory layout — the cursor grammar survives unchanged.

**ANS:** (by )
[Fill this in]   <!-- literal placeholder — parser-significant, do not paraphrase -->

---

### QST-CURSOR-SEMANTICS: What exactly does a cursor name?
- Status: unanswered
- Why asking: `moreover -c Ae2e --all` and a hypothetical `moreover -c Ae2e -20` must both make sense. Does the cursor carry only "which stream + where in it," with page size per-call, or does it also freeze paging parameters?
- Need: cursor = f(what)

**Recommendation**: (by Operator 5, Claude Fable 5)
**Stream identity + offset only; page size stays per-call.** *Rationale*:
the founding sketch shows exactly this shape — one cursor consumed by both
`--all` and a sized call; freezing parameters into the cursor would make the
sketch's second line impossible. *Confidence*: high — the sketch is the
spec. *If wrong*: optional fields can be added to the cursor's stored side
without changing its printed form; nothing breaks.

**ANS:** (by )
[Fill this in]   <!-- literal placeholder — parser-significant, do not paraphrase -->

---

### QST-PAGE-UNITS: Lines, bytes, or tokens?
- Status: unanswered
- Why asking: the reader is a model, and models budget in tokens — but tokenizers are model-specific, heavy dependencies, and the founding sketch speaks in lines.
- Need: v0 unit set + the upgrade path

**Recommendation**: (by Operator 5, Claude Fable 5)
**Lines by default, bytes via flag, in v0; tokens deferred to an opt-in
`--tokens` with a pluggable tokenizer.** *Rationale*: lines match the sketch
and fifty years of pager intuition; bytes are free to support; a bundled
tokenizer would tie a tiny tool to one model family's arithmetic.
*Confidence*: medium. *If wrong*: if real transcripts show models repeatedly
burned by mid-structure truncation that token-aware pages would have
avoided, promote tokens into v1 and say so in this ADR's iterations.

**ANS:** (by )
[Fill this in]   <!-- literal placeholder — parser-significant, do not paraphrase -->

---

### QST-TRAILER-GRAMMAR: Is the trailer line's grammar frozen as sketched?
- Status: unanswered
- Why asking: the trailer is the tool's most-seen surface (it lands in a model's context every page) and the founding record already published its shape; changing it later breaks the readers who learned it.
- Need: freeze/amend decision for v0, incl. the unknown-total case

**Recommendation**: (by Operator 5, Claude Fable 5)
**Freeze the sketch verbatim as v0** — `<moreover: page N, X/Y lines,
cursor: C>` — **with `Y = ?` when the total is unknown** (live streams) and
`cursor: null` at exhaustion. *Rationale*: it is already the published
founding grammar; models parse stable grammars; the one case the sketch
leaves open (unfinished input) needs exactly one symbol. *Confidence*: high
for v0. *If wrong*: introduce a versioned prefix (`<moreover/2: …>`) only on
a breaking need — never silently.

**ANS:** (by )
[Fill this in]   <!-- literal placeholder — parser-significant, do not paraphrase -->

---

## Consequences

- The cursor store (QST-CURSOR-STORE) is the only stateful component; every
  other choice is grammar. Settle it first and the implementation is mostly
  `io::copy` with a bookmark — which is why Rust's answer to "io reasons" is
  the whole runtime story.
- The trailer grammar, once frozen, is a compatibility promise to
  every model that reads it.

## Action Items

- [ ] Answer the four QSTs (accept or override the recommendations) - Owner: Jérémie + founding crew
- [ ] First implementation after (or judgment-first with defaults, recording deviations here) - Owner: founding crew
- [ ] `cargo build` green + first regression test per the Third Directive - Owner: founding crew

## Iterations

### Iteration 1 (2026-09-21)
- Trigger: founding. Spec sketch + Rust ruling staked verbatim; four questions opened with recommendations.
- Contributors: Jérémie Lumbroso (spec, ruling); Operator 5 (record, recommendations).
- Outcome: `— → Draft`
