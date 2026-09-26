<!-- adr template version: "adr 3.11.5" -->

# ADR-0005: The cursor lifecycle — mint modes, the desk verbs, and equality

- **Date**: 2026-09-26
- **Iteration**: 1
- **Status**: Accepted
- **Deciders**: Jérémie Lumbroso (rulings, from the dogfooding seed's QST-MINT-POLICY answer); Ribbon 5 (this record, the concrete design); Lector 6 (the equality analysis this ADR must satisfy)

**TL;DR**: Cursor minting becomes a switchable mode — deterministic ids
keyed on (spool, offset, page) by default, fresh random ids on request —
selected by **assertive** flags (each names a mode outright, never
toggles a default), with the settings-level default arriving with the
config home. The desk verbs `ls`/`gc`/`drop` complete the lifecycle.

---

## Originating Context

**Source**: `seed-2026-09-23-first-human-dogfooding-session.md`,
QST-MINT-POLICY ANS (Jérémie, 2026-09-26) — chunked at his request. His
rulings, distilled with the load-bearing phrases verbatim:

1. **Switchable modes**: default to deterministic (option B, "which
   makes most sense to us now") while allowing fresh minting — because
   whether determinism or freshness serves a reader better is "something
   that models will have [to] determine for themselves, since the point
   of this tool is to match model intrinsic preferences."
2. **Assertive flags, never toggles**: "instead of having a flag that
   modifies the existing preference, the flags should be assertions of a
   specific mode, that way they are useful no matter what the default
   is." This is a house-wide flag doctrine from this ruling forward, not
   just a cursor rule.
3. **A debt acknowledged**: he noted the cost table "didn't provide
   sufficient details" on B's concrete design (files, contents, updates)
   and chose to trust rather than micromanage — so this ADR owes the
   full mechanism, stated checkably.

Also folded: Lector 6's equality analysis (seed thread, first pass) —
two cursors can share (spool, offset) while differing in page ordinal,
so any deterministic scheme must state its equality key; and the
first-contact field report's observation of concurrent readers'
cursors mingling (which shipped desk scoping in `-c last` and promotes
`ls`).

**Agency Grant**: the design below is Ribbon 5's to implement; mode-flag
names route to the naming authority before shipping; anything that
touches the frozen trailer grammar returns to Jérémie.

---

## Decision

### The deterministic mode's concrete design (the owed detail)

- **Identity key**: a next-cursor is identified by the triple
  `(spool hash, byte offset, page ordinal)`. Lector's equality catch is
  satisfied by construction: distinct paging histories meeting at one
  offset differ in page ordinal and therefore keep distinct records, so
  the trailer's `page N` stays exactly truthful.
- **Id derivation**: the printed id is a 4-character Crockford base-32
  rendering of a keyed hash over the triple (same alphabet, same
  mixed-letter-digit guarantee as random minting — derived candidates
  that come out all-digit or all-letter are re-hashed with a counter
  until mixed, deterministically). The id remains opaque to the reader:
  nothing about it is extrapolable, preserving the petname doctrine —
  determinism is a *storage* property, not a *legibility* one.
- **Record files**: unchanged format (`spool=`, `offset=`, `line=`,
  `page=`, `desk=`, plus `mode=stable`), one file per triple, created
  with `create_new`. A repeat resume finds the file already present:
  verify its contents match the triple; on match, reuse silently (this
  is the idempotency working); on mismatch (a 20-bit collision between
  different triples), extend the id one character and retry — the same
  collision ladder random minting already uses.
- **The `desk` field on reuse**: first-writer wins; the record keeps its
  original mint desk. `-c last` semantics are unchanged (newest matching
  record by mtime; reuse refreshes mtime so "last" tracks actual use).
- **Fresh mode**: exactly today's behavior (random mint per resume),
  selected by assertion; records carry `mode=fresh`.

### The assertive flags (names pending the naming authority)

Working names, to be struck or confirmed: `--mint stable` and
`--mint fresh` — one flag, one vocabulary, each value a complete
assertion (the QST-TRAILER-DEST shape, reapplied). The settings-level
default lands when the config home ships (QST-ADOPTION-WIZARD's
dependency); until then the built-in default is `stable` from this
ADR's implementation onward.

### The lifecycle verbs

`ls` (this desk's cursors; `--everywhere` for the machine), `gc [DAYS]`,
and `drop CURSOR` implement per ADR-0003's accepted designs — `ls` was
promoted to next-ship by the first-contact field report. Deterministic
minting shrinks the debris `gc` exists to sweep; they compose, not
compete (his QST answer's own observation).

### Consequences

- A reader replaying its transcript sees the *same* next-cursor for the
  same resume — divergence in the record now means divergence in fact.
- The million-identical-calls cost becomes one record file.
- The mode choice is observable data for the ThirdX question he raised
  (which minting matches model intrinsic preference) — records carry
  their mode, so real usage can answer it.

## Questions

### QST-MINT-FLAG-NAMES: What are the assertive mode flags called?
- Status: unanswered — routed to the naming authority (docket, hub inbox)
- Why asking: house rule — surface vocabulary is the naming authority's; and this ruling coined a flag *doctrine* (assertions, never toggles) that deserves its naming pattern stated once for every future mode flag.
- Need: the flag name(s) + the doctrine's canonical phrasing

Options: none (elicitation) — working names `--mint stable|fresh` are in
the docket as the strawman.

**ANS:** (by )
[Fill this in]   <!-- literal placeholder — parser-significant, do not paraphrase -->

---

## Action Items

- [ ] Naming authority strike on the mode flags - Owner: Ribbon 5 (docket), naming authority (strike)
- [ ] Implement deterministic mode + assertion flag + `mode=` field, tests incl. the collision ladder and the replay-idempotency property - Owner: Ribbon 5
- [ ] `ls` / `gc` / `drop` per ADR-0003 - Owner: Ribbon 5
- [ ] Lector 6 audit invited on the derivation/collision design before release - Owner: Lector 6

## Iterations

### Iteration 1 (2026-09-26)
- Trigger: QST-MINT-POLICY answered in the dogfooding seed; chunked here at his request.
- Contributors: Jérémie (rulings); Ribbon 5 (concrete design, the owed detail); Lector 6 (equality constraint, by prior analysis).
- Outcome: `— → Accepted` (the policy); implementation pending the flag strike.

---

## Links

- Related: the dogfooding seed (QST-MINT-POLICY); ADR-0003 (desk verbs, `-c last`, desk scoping); Lector 6's seed thread (equality)
