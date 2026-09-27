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
- Status: answered — the naming authority, same-hour (hub brief 1621), 2026-09-26
- Why asking: house rule — surface vocabulary is the naming authority's; and this ruling coined a flag *doctrine* (assertions, never toggles) that deserves its naming pattern stated once for every future mode flag.
- Need: the flag name(s) + the doctrine's canonical phrasing

Options: none (elicitation) — working names `--mint stable|fresh` are in
the docket as the strawman.

**ANS:** (by the naming authority, 2026-09-26, relayed by Ribbon 5)
**`--mint stable|fresh` confirmed** — both values pass the assertion
doctrine — with one implementation guard: the values never grow bare
shortcut flags (a standalone `--fresh` would re-fracture the axis; the
bare-format-flag lesson, one door over). The doctrine's canonical
sentence, staked for every future mode-flag review: **"Flags assert
destinations, never deltas."** — a destination-naming flag is true under
every default and every future; a default-negating flag breaks the day
the default moves.

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

### Pre-release audit — Lector 6 (GPT-6 Astra; gpt-6-astra), 2026-09-26

**Recommendation: hold the stable-default release for the desk recovery,
record publication, and ID-length issues below.** This reviews `0f3f128`,
the stable-mint change after the `0.3.0` release commit; the shared package
version alone does not identify this implementation. All 34 existing
tests pass. The sequential replay test and the same-offset/different-page
test establish the intended equality distinction, but do not exercise
the collision ladder or publication window.

The store probes are retained in
[`scripts/ephemeral/2026-09-26-stable-mint-audit.rs`](../../scripts/ephemeral/2026-09-26-stable-mint-audit.rs),
with run instructions and attribution to the delegated audit reviewer.
They compile the actual `store.rs`, exposing its derivation function
through a probe wrapper. They do not reimplement the hash. The desk
result below is a CLI reproduction; the publication and full-ladder
cases are deliberately constructed filesystem states.

**1. Stable reuse breaks desk-local recovery (high priority).**
`put_cursor` retains the first writer's `desk` while touching the shared
record's mtime (`src/store.rs:236–243`); `last_cursor_for_desk` filters by
that retained desk. In one scratch state directory with two working
directories A and B, the current CLI produced:

| Operation | Result |
|---|---|
| A pages `T1\nT2\nT3\nT4\n` with `-2` | cursor `116k` |
| A pages `U1\nU2\nU3\nU4\n` with `-2` | cursor `and1` |
| A runs `-c last --all` | `U3\nU4\n`, then `cursor: null` |
| B pages the same T input with `-2` | reuses `116k` |
| B runs `-c last --all` | exit 1: no cursors minted from B |
| A runs `-c last --all` again | `T3\nT4\n`, then `cursor: null` |

B cannot recover the cursor it just received; its use also changes A's
recovery selection. This follows the ADR's first-writer rule, so the
decision's claim that `last` semantics are unchanged needs resolution,
not merely an implementation tweak. My recommendation is to retain the
accepted global triple and track each desk's use separately. Adding
`desk` to the identity would change the accepted equality rule. The
regression should have both desks use the same triple and assert that
B can recover it while A's selection remains its own most recent use.

**2. A published filename is not yet a published record (high priority).**
The successful `create_new` exposes the final name before the record's
multiple writes and sync complete (`src/store.rs:217–228`). A stable
contender treats an unreadable/incomplete record as a collision and
advances the rung (`231–246`). For spool `0123456789abcdef`, offset 1,
page 2, the controlled writer-pause fixture gives:

```text
primary candidate:                  c2dc
same mint while primary is empty:    q147
same mint after primary is complete: c2dc
records for this triple:             2
```

The fixture represents the first writer paused immediately after
`create_new`; it is not a claim that an uncontrolled thread race was
observed. It demonstrates a reachable state in which "one record" and
"same next-cursor" both fail. Serialize the identity lookup/publication
or publish complete records atomically without replacing an existing
candidate. A contender must distinguish a completed collision from an
unfinished writer. The regression needs an interleaving at this boundary,
with both returned IDs equal and the completed record readable.

**3. The ladder can return an ID its reader rejects (medium priority).**
The mint loop has no maximum length, while `normalize_id` rejects IDs
longer than 16. The probe fills the candidates for lengths 4 through 16
with valid records for another triple: 104 rung attempts. For offset 42
it then obtains `ttz6vqqr62652vprm`, length 17. Immediate `get_cursor`
returns `invalid cursor id`. This is a forced ladder-boundary check,
not an estimate of ordinary collision frequency. Bound allocation to
the accepted ID space and return an explicit exhaustion error before
creating an unusable record, or deliberately revise the reader limit
with matching compatibility tests. Every successful mint must round-trip
through `get_cursor`.

**The ordinary collision comparison works, with two qualifications.**
For the same spool and page 2, offsets 754 and 1399 genuinely share the
first candidate `7jnq`. Allocation gives `7jnq` and `283d`; replaying
the second triple returns `283d`. The code compares all three identity
fields and preserves the different positions. Both IDs remain four
characters: the actual ladder tries eight rungs at each length before
extending, whereas the decision text says each collision extends it.

If the first triple's `7jnq` record is removed, minting the second triple
again returns the now-free `7jnq`, even though its old `283d` record still
exists. Thus the current stable-ID promise depends on earlier collision
occupants remaining present. The forthcoming `gc`/`drop` design must
either preserve the triple's established mapping or state this limit;
"the same next-cursor forever" is too strong for this mechanism.

**Language and coverage to finish before release.** The implementation
uses ordinary FNV-1a over serialized fields and counters, without a
separate hash key. Describe it as **"a deterministic hash of the triple
and collision counters"**, rather than a keyed hash. Describe opacity
as the rule that readers obtain IDs from the tool; the public derivation
algorithm does not make them impossible to compute.

The help names `--mint`, but `moreover contract` does not. Once the
behavior above is settled, add both asserted modes, the stable default,
the full identity triple, and the exhaustion exception: no mode creates
a successor when the trailer says `cursor: null`. Describe `mode=` as
the record's creation mode; it is not a count of later choices, since
stable calls reuse records while fresh calls accumulate them. Reuse can
also retain a matching fresh or legacy record's original fields.

The equality key itself remains appropriate: `line` and `nl` follow from
the immutable spool prefix, and page size, unit, and overlap do not need
to become identity dimensions. One useful complementary test is that
`5+15` and `10+10` converge to the same successor: different histories
with the same spool, offset, and page ordinal should share it. The
existing `20` versus `10+10` test correctly separates different ordinals.

This is the pre-release audit and its proposed repairs. Product code,
the accepted identity key, and the human's decisions are unchanged.

---

## Links

- Related: the dogfooding seed (QST-MINT-POLICY); ADR-0003 (desk verbs, `-c last`, desk scoping); Lector 6's seed thread (equality)
