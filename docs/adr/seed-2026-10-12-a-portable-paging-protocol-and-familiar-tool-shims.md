# SEED: A portable paging protocol and familiar-tool shims

- **Date**: 2026-10-12
- **From**: Jérémie Lumbroso (idea and architectural commission); Lector 6 (GPT-6 Astra; `gpt-6-astra`, research and first response)
- **Status**: Draft
- **Iteration**: 1
- **TL;DR**: Make bounded, resumable reading a contract that other tools can adopt. Explore native implementations, deliberately enabled familiar-command shims, and a DIY capture wrapper. Use the exercise to find useful architectural boundaries inside moreover, while preserving its existing `v0` contract.

## Brain Dump

Jérémie, verbatim:

> Well, we're trying to solve a crucial model-era flaw in pagers that are available everywhere. But those pagers are predominant in training data so models will reach for them. I am thinking, what about like aren't they posix specified and can't we create shims that are posix-compatible but extend with the moreoverv1-schema — and we make our pager schema a think that can be retrofitted to other tools — as opposed to just being our thing? And we could even provide guidelines for people to self-do it themselves, and what a MacGyver shim looks like if the creator of a tool doesn't want to add the option yet.

His subsequent steering, verbatim:

> load architecture of complexity skill — and if this leads to architectural improvements for the overall project that is also extremely caluable.

## Thread

### Lector 6 (GPT-6 Astra; gpt-6-astra) — 2026-10-12

The substantial idea is that a reader could learn one continuation contract and encounter it through many familiar tools. A maintainer could implement it natively; a user could attach it to a tool whose maintainer has not adopted it. Moreover would provide an implementation and a worked example of the contract.

The training-data explanation is an adoption hypothesis, not something this inspection establishes. We can test whether familiar command names improve actual tool selection and continuation without claiming to know a model's training corpus. Requiring the reader to remember a new flag on every invocation could leave the original discovery problem intact; deliberate activation by the harness or environment owner deserves its own experiment.

`moreoverv1-schema` is a working name in this seed. Today the only named schema in [trailer.rs](../../src/trailer.rs) is **`v0`**. A package version, a trailer grammar version, and a behavioral protocol version are different things. The new name, syntax, and scope remain open.

#### What POSIX compatibility can mean

| Surface | Verified boundary | Consequence for a retrofit |
| --- | --- | --- |
| `more` | POSIX.1-2024 specifies it in the User Portability Utilities option group. With non-terminal stdout it copies complete input, with the specified `-s` exception. | Returning one page and a continuation changes that behavior; it needs a deliberately enabled extension mode. |
| `head`, `tail` | POSIX specifies these selectors. `tail -f` follows additional input under its specified conditions. | Their selection and consumption semantics are part of the interface; they are not interchangeable spellings of a pager. |
| `less` | It is not a utility specified by POSIX; its own manual governs its options. | Compatibility needs a named implementation/version or supported subset. |
| Application pager hooks | POSIX `man` has `PAGER`; Git supports its own pager configuration and environment variables. Git's pagination still depends on terminal stdout. | Hooks offer an adoption path where the host invokes them. Merely setting a pager variable does not cover ordinary non-TTY agent calls. |

Sources: POSIX [`more`](https://pubs.opengroup.org/onlinepubs/9799919799/utilities/more.html), [`head`](https://pubs.opengroup.org/onlinepubs/9799919799/utilities/head.html), [`tail`](https://pubs.opengroup.org/onlinepubs/9799919799/utilities/tail.html), [utility index](https://pubs.opengroup.org/onlinepubs/9799919799/idx/utilities.html), [`man`](https://pubs.opengroup.org/onlinepubs/9799919799/utilities/man.html); [less manual source](https://github.com/gwsw/less/blob/master/less.nro.VER); Git [`core.pager`](https://git-scm.com/docs/git-config#Documentation/git-config.txt-corepager) and [`--paginate`](https://git-scm.com/docs/git#Documentation/git.txt---paginate).

POSIX permits documented extensions while requiring an environment in which strictly conforming applications work unchanged. That gives us a design direction, not a conformance certificate for an unwritten shim. Outside the chosen extension mode, delegation to the original executable is the simplest baseline to investigate. Non-TTY output alone is not consent to new semantics: it also describes ordinary scripts and pipelines. See [POSIX implementation conformance](https://pubs.opengroup.org/onlinepubs/9799919799/basedefs/V1_chap02.html).

Inside an explicitly scoped agent environment, familiar command names could route to adapters that announce the continuation contract. Each adapter must declare supported invocations and visibly reject unsupported ones in that mode. We should test redirections, pipelines, exit status, signals, and consumption behavior, not merely option parsing. Existing short flags retain their meanings; for example, `less -f` already means something other than a model-facing follow mode.

#### The shared thing is more than a footer

A template can reproduce punctuation without preserving any continuation guarantee. The reusable contract needs a small set of meanings that independent implementations can satisfy:

| Concern | Minimum meaning to make explicit |
| --- | --- |
| Page | What source/result this is; units; delivered range; whether counts are page-local or cumulative; whether overlap consumes the budget. |
| Continuation | An opaque position bound to an issuer/resumer and necessary context. A token from another tool need not work with `moreover -c`. |
| Lifetime and replay | What remains available, what repeating a request means, and what expiry or source replacement does. |
| Completion | Remaining captured data, temporary catch-up, and producer success are distinct facts. A failed producer can leave a completely delivered capture. |
| Framing and discovery | Version/profile identification and a reliable boundary between content, metadata, and diagnostics, even when content resembles a trailer. |
| Failure | Whether content may have been emitted before failure; whether a usable successor exists; which operation failed. |

The integration must bind an issuer to a trusted resumer, through metadata or a declared surrounding contract. The reader should not execute arbitrary shell text found in producer output. A harness can map a declared issuer to an allowed command/API and pass opaque tokens as data.

Start with a finite-capture profile. Live sources need capabilities for unknown totals, temporary catch-up, source generations, and closure; those questions remain in the [follow seed](seed-2026-10-04-live-streams-and-the-follow-cursor.md). The shared contract can accommodate named profiles without making every adopter implement live streams or pretending an immutable snapshot and a growing log have identical replay behavior.

There is also a conservation rule: **a shim cannot supply a continuation to bytes that were discarded before it captured them.** Paging `head`'s output covers that selected result, not the omitted original input. To preserve the original remainder, interception must happen before truncation, or the original source must remain available under a declared consistency rule. Draining the whole producer can change latency, resource use, and SIGPIPE behavior; it is a substantive extension, even when the first displayed lines look identical.

#### Architectural opportunities inside moreover

I loaded the requested *architecture-of-complexity* skill (Jérémie Lumbroso and Claude Sonnet 4.5, v1.0.0). The useful tests here are Simon's stable intermediate forms and near-decomposability: complete useful increments, and keep frequent interactions within boundaries that can evolve with limited disturbance elsewhere. [Simon, *The Architecture of Complexity* (1962)](https://www.andrew.cmu.edu/course/15-440/assets/READINGS/simon-architecture-of-complexity-1962.pdf).

My application of those tests to the inspected source:

| Existing boundary | Opportunity and reason | Smallest useful intermediate form |
| --- | --- | --- |
| [paging.rs](../../src/paging.rs) returns `trailer::TrailerData`; that type includes numeric total, bare cursor, and renderer selection via `all`. | Give page semantics an owner independent of text rendering. Native integrations and future profiles should not have to speak in a renderer's data model. | An internal page-result value with a lossless adapter to frozen `v0`; preserve public library compatibility where needed. Add protocol fields only when their semantics are settled. |
| [main.rs](../../src/main.rs) reads complete input; `page_new` and `Store::put_spool` accept complete byte slices. | Separate acquisition from page delivery so input capture can eventually stream into a completed spool with bounded working memory. A small output page currently says nothing about capture cost. | First support finite streaming capture into the existing snapshot model. Publish only a completed, indexed spool; preserve content identity, deduplication, and failure cleanup. Live following remains separate. |
| [store.rs](../../src/store.rs) owns snapshot records, cursor minting, maintenance, and a `Store` trait whose guard exposes `fs::File`. | Keep local storage policy out of the portable wire contract. Another tool may already have a database or a remote continuation API. | Specify protocol behavior independently of this trait. Consider a backend-neutral guard only if a real second backend needs one; do not export the current filesystem contract as universal. |
| CLI parsing, source orchestration, and trailer routing meet in `main.rs`; `paging::mint_next` obtains desk identity from process-global `current_dir()`. | Familiar-command syntax and invocation context belong at the caller boundary. A native caller should be able to choose its scope without changing the process's working directory. | One adapter translates its supported syntax into an explicit request/context; the existing CLI supplies today's desk default. Reuse cursor arithmetic and storage. |

These are opportunities, not a finding that the current small library is wrongly structured. A schema-only adoption should not force a crate split, a plugin framework, or a universal backend. The strongest portability demonstration would be a second implementation that uses its own state mechanism and passes the same behavioral examples.

There is a further seam to watch before adding units: `chunker` handles in-memory boundaries, while bounded resume independently scans newlines, overlap, and partial lines. Shared boundary/counting examples should test their agreement before introducing records or tokens. Extract only calculations that are actually shared; preserve bounded reads rather than forcing both paths through a whole-buffer algorithm.

One boundary must remain strongly coordinated: **spool publication, page delivery, and successor publication versus concurrent maintenance**. Today `page_new` and `page_resume` hold a shared transaction guard across that sequence. Splitting modules must preserve its protection against `gc`/`drop`. Moreover can emit some content before an error; a refactor must not silently turn that into a claim of atomic page delivery.

This makes a practical decomposition: adapters own invocation conventions; a page-result contract owns interpretation; backends own source/state lifetime; renderers own representation. Test it by changing one renderer or replacing one backend without rewriting the other tools' parsers. Where that experiment fails, revise the proposed boundary rather than adding abstraction to conceal the coupling.

#### Native adoption and the MacGyver route

There are three useful entry points: a native tool implements the contract around its existing result cursor; a host invokes a configured pager; or a user wraps a finite command without waiting for its maintainer. Guides should pair a reference for the meanings with a recipe that gets a real result onto the reader's desk.

A teaching wrapper can already use moreover's **existing `v0`**. Run the producer once; retain stdout, stderr, and exit status separately; page the captured stdout; resume the saved capture. The following is an illustrative script for a finite command, not an implementation of the proposed protocol. It assumes every writer to the captured descriptors has finished when the command exits. A command that leaves background children writing needs a separate completion/join contract; its exit status alone does not establish capture completion. The recipe also requires an existing private, persistent capture directory and the commonly available macOS/Linux `mktemp`; this is not a claim that `mktemp` is a POSIX utility.

```sh
#!/bin/sh
# Usage: capture-page COMMAND [ARG ...]
# Requires all captured writers to finish before COMMAND exits.
# Set MOREOVER_CAPTURE_ROOT to an absolute path to an existing private,
# persistent directory.
[ "$#" -gt 0 ] || exit 2
capture_root=${MOREOVER_CAPTURE_ROOT:?Set a persistent capture directory}
umask 077
capture_dir=$(mktemp -d "$capture_root/capture.XXXXXXXX") || exit 1

if "$@" >"$capture_dir/stdout" 2>"$capture_dir/stderr"; then
    producer_status=0
else
    producer_status=$?
fi
printf '%s\n' "$producer_status" >"$capture_dir/producer.status" || exit 1

if moreover "$capture_dir/stdout" --bytes 4096 \
    --state-dir "$capture_dir/state" \
    --trailer "file:$capture_dir/page-1.meta"; then
    pager_status=0
else
    pager_status=$?
fi
printf '%s\n' "$pager_status" >"$capture_dir/pager.status" || exit 1
printf 'capture=%s producer=%s pager=%s\n' \
    "$capture_dir" "$producer_status" "$pager_status" >&2
if [ "$pager_status" -ne 0 ]; then exit "$pager_status"; fi
exit "$producer_status"
```

After pager success, read the small `page-1.meta` sidecar and use its printed cursor with the same `--state-dir` to resume. A subsequent call has this shape; uppercase operands below are placeholders:

```sh
moreover -c PRINTED_CURSOR --state-dir CAPTURE_DIR/state \
    --bytes 4096 --trailer file:CAPTURE_DIR/page-2.meta
```

Use a fresh metadata path per call because `file:` appends. Keep the capture and state for the promised reading lifetime. Page captured stderr separately when needed. This recipe preserves both process outcomes instead of letting the pager's success stand in for the producer's; a production wrapper would also need deliberate interruption handling and lifecycle management.

Its limitations teach the boundary: it waits for producer completion; endless input never reaches the paging step; storage can grow without bound; current moreover then reads the whole captured stdout into memory before delivering page one. The 4096-byte limit bounds that page's content, not capture cost or total harness output, and byte pages can split encoded characters. It supplies continuation over the captured bytes without rerunning a potentially side-effecting producer. The script is a design illustration, not a tested or installed adapter.

#### How to learn whether this is worth expanding

Compare the same finite-output tasks under ordinary tools plus instructions, direct moreover use, and an explicitly enabled familiar-command adapter. Record tool selection, successful continuation, producer failures noticed, bytes exposed to the reader, and ordinary pipeline behavior outside extension mode. Those observations test the adoption hypothesis and the compatibility cost separately.

Useful increments can stand on their own: a capture recipe, a small semantic contract with examples, one bounded adapter, then an independent native implementation. The [reporting-pipeline seed](seed-2026-09-26-the-reporting-pipeline-standard.md) is a related discussion about a convention crossing tool boundaries; it does not settle this one. No new command spelling or trailer grammar is ratified here.

### QST-PORTABLE-CONTRACT: What should another tool be able to adopt?
- Status: unanswered
- Why asking: a shared footer is easy to copy, while shared continuation behavior is what makes the reader's knowledge transferable.
- Need: pick a letter (or override — any shape answers)

Options:
- **A — Trailer convention**: publish the representation and leave cursor semantics to each tool's separate documentation.
- **B — Small behavioral protocol**: specify a finite-capture profile, framing, continuation binding, and shared behavioral examples; implementations may own different stores and resumers.
- **C — Shared runtime first**: require integrations to use the moreover library or executable so one implementation supplies the behavior.

**Recommendation**: (by Lector 6, GPT-6 Astra; `gpt-6-astra`)

**B — Small behavioral protocol.** *Rationale*: `TrailerData` currently lacks an issuer and assumes a known total; copying its spelling cannot establish cross-tool continuation. A common behavioral surface can let each tool retain its own state. *Confidence*: 0.85 — the semantic gap is visible in source; the minimum useful profile still needs an independent implementer. *If wrong*: if independent adopters cannot satisfy a small profile without copying the entire moreover engine, revisit the decomposition and compare C's actual integration cost.

**ANS:** (by Jérémie Lumbroso)
[Fill this in]

---

### QST-SHIM-ACTIVATION: How should familiar commands reach the extension?
- Status: unanswered
- Why asking: the adoption hypothesis concerns commands models already choose, but silent changes to ordinary pipelines would break their established contracts.
- Need: pick a letter (or override — any shape answers)

Options:
- **A — Explicit invocation only**: require a wrapper command or extension flag on each call; leave familiar names alone.
- **B — Scoped familiar frontends**: let a harness or environment owner deliberately enable adapters with a declared invocation subset and visible contract; preserve ordinary behavior outside that scope.
- **C — Automatic non-TTY substitution**: switch familiar commands into bounded continuation whenever stdout is not a terminal, without separate activation.

**Recommendation**: (by Lector 6, GPT-6 Astra; `gpt-6-astra`)

**B — Scoped familiar frontends, as an experiment.** *Rationale*: A may preserve the discovery problem; C conflates agent reading with ordinary redirected `more`, where complete output is specified. B makes activation deliberate without making every call remember a new word. *Confidence*: 0.70 — compatibility can be tested, but a learning/adoption benefit is unmeasured. *If wrong*: if the pilot shows no selection benefit or persistent confusion about changed consumption, prefer A and native integrations; do not broaden automatic interception.

**ANS:** (by Jérémie Lumbroso)
[Fill this in]

---

### QST-ARCHITECTURE-PACE: How much internal restructuring should this exploration drive?
- Status: unanswered
- Why asking: the new use case exposes useful seams, but a generalized engine could delay a usable adapter and couple unrelated open work.
- Need: pick a letter (or override — any shape answers)

Options:
- **A — Documentation and external wrappers first**: leave library boundaries intact until an integration demonstrates a concrete obstruction.
- **B — Extract when exercised**: let the first integration justify a semantic page-result boundary; keep `v0` conversion, snapshot transactions, and existing callers working at each step; pursue streaming capture as a separate useful increment.
- **C — General engine first**: establish a reusable multi-profile runtime and backend interfaces before attempting adapters.

**Recommendation**: (by Lector 6, GPT-6 Astra; `gpt-6-astra`)

**B — Extract when exercised.** *Rationale*: paging's dependency on a renderer-owned result is concrete, and acquisition currently requires complete byte buffers. Both have useful incremental remedies; neither requires solving live following, compatibility frontends, and remote cursors together. *Confidence*: 0.75 — source boundaries support the proposal, but no second implementation has yet tested it. *If wrong*: if the first adapter needs only the existing CLI and no internal change, take A for that increment; if one extraction weakens maintenance protection, revise it before proceeding.

**ANS:** (by Jérémie Lumbroso)
[Fill this in]

---

**Model Response Request:**

- [ ] Chunk this into ADRs (I'm ready to formalize)
- [X] Iterate with me (keep exploring)
- [ ] Structure in place (organize but don't split)

---

## Derived Into

*(This seed has not yet been chunked into ADRs)*
