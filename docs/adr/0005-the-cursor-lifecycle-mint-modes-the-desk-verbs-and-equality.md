<!-- adr template version: "adr 3.11.5" -->

# ADR-0005: The cursor lifecycle — mint modes, the desk verbs, and equality

- **Date**: 2026-09-26
- **Iteration**: 3
- **Status**: Partially Implemented
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
- **Id derivation** *(wording corrected at Iteration 2, per the
  pre-release audit)*: the printed id is a 4-character Crockford base-32
  rendering of **a deterministic hash of the triple and collision
  counters** (ordinary FNV-1a over serialized fields — there is no
  separate hash key; same alphabet, same mixed-letter-digit guarantee,
  with all-digit/all-letter candidates re-hashed deterministically).
  Opacity means what the petname doctrine means: **readers obtain ids
  from the tool** — the public derivation does not make them impossible
  to compute, and never needs to.
- **Record files**: unchanged format (`spool=`, `offset=`, `line=`,
  `page=`, `desk=`, plus `mode=stable`), one file per triple, created
  with `create_new`. A repeat resume finds the file already present:
  verify its contents match the triple; on match, reuse silently (this
  is the idempotency working); on mismatch (a collision between
  different triples) or an unreadable occupant (legacy debris), climb
  the ladder — eight rungs per length, lengths 4 through 16 (the id
  space readers accept), then an explicit exhaustion error; a mint that
  cannot round-trip through the reader is never created. Publication is
  atomic (write-then-hard-link): a contender never observes a
  named-but-incomplete record. *(Corrected and hardened at Iteration 2.)*
- **Desk recency** *(redesigned at Iteration 2 — the audit's finding 1
  showed first-writer desk + shared-record mtime broke one desk's
  recovery and disturbed another's)*: the shared record keeps its first
  writer's desk and is never modified on reuse; each desk's own uses are
  tracked in per-desk recency files (`desks/`), which `-c last` reads
  first (legacy record-scan as fallback). One desk's activity can no
  longer affect another's recovery.
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
  *(Bounded claim, per the audit: the triple→id mapping under collision
  depends on earlier collision occupants remaining present — so the
  forthcoming `gc`/`drop` design must preserve established mappings for
  live triples, or this claim weakens to "the same next-cursor while
  the state dir is intact." Staked as a hard constraint on the verbs.)*
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

- [x] Naming authority strike on the mode flags — struck same-day (`--mint stable|fresh`; "Flags assert destinations, never deltas")
- [x] Implement deterministic mode + assertion flag + `mode=` field - Owner: Ribbon 5 — `0f3f128`; audit repairs follow-up commit
- [x] Lector 6 pre-release audit — returned with a HOLD; all three findings repaired with the audit's own regression specs (per-desk recency; atomic publication; bounded ladder + exhaustion error + round-trip invariant); language corrections applied above
- [ ] `ls` / `gc` / `drop` per ADR-0003 — with the Iteration-2 constraint: gc/drop must preserve established triple→id mappings for live triples - Owner: Ribbon 5
- [x] Lector 6 re-check of the repairs before the release that carries stable-default - Owner: Lector 6 — cleared `f9631d6`; see final clearance below

## Iterations

### Iteration 1 (2026-09-26)
- Trigger: QST-MINT-POLICY answered in the dogfooding seed; chunked here at his request.
- Contributors: Jérémie (rulings); Ribbon 5 (concrete design, the owed detail); Lector 6 (equality constraint, by prior analysis).
- Outcome: `— → Accepted` (the policy); implementation pending the flag strike.

### Iteration 2 (2026-09-27)
- Trigger: Lector 6's pre-release audit (below, 2026-09-26) — a HOLD with three findings, each demonstrated by probes compiled against the real store (`scripts/ephemeral/2026-09-26-stable-mint-audit.rs`).
- Contributors: Lector 6 (audit, probes, regression specs, language corrections); Ribbon 5 (repairs).
- Changes: per-desk recency files replace shared-record mtime (finding 1 — the audit's exact failure table is now a CLI regression test); publication made atomic via write-then-hard-link (finding 2 — the named-but-empty window is closed; the legacy-debris form is a deterministic-ladder regression test); the ladder bounded to the reader's id space with an explicit exhaustion error and an in-module 104-rung regression (finding 3); every-mint-round-trips invariant asserted; "keyed hash" and opacity wording corrected; the gc/drop mapping-preservation constraint staked; the convergence test (5+15 ≡ 10+10) added.
- Outcome: `Accepted → Partially Implemented`; release still HELD pending Lector's re-check; `ls`/`gc`/`drop` remain.

### Iteration 3 (2026-09-27) — the re-check's findings, repaired

The re-check (below) confirmed the normal-path repairs and found three
defects *in the repairs*, all now fixed with its regression specs:
(1) the desk-recency update no longer fails silently — a write failure
propagates with a diagnostic naming the minted id the reader would
otherwise lose (the chmod-500 table is a unix-gated CLI regression),
and a present-but-damaged recovery record is reported, never silently
scanned around; (2) recency updates stage in exclusively owned temps
(create_new + rename), closing the shared-.part truncation interleaving;
(3) cursor mint temps use exclusive creation with cleanup guaranteed on
every error path, so a repeated temp name can never truncate a published
record through its hard-link alias. The contract omission the re-check
caught was real — an earlier unverified text replacement had silently
no-opped (the same failure class as the release-gate incident, now also
in memory) — the contract now carries the mint modes, the triple, the
per-desk recovery source, the four-character-minimum id note, and the
`cursor: null` exception. 40 tests green on cargo's own exit code.

- Contributors: Lector 6 (re-check, probes, regression specs); Ribbon 5 (repairs, contract).
- Outcome: status unchanged; release remains HELD for Lector's confirmation pass; `ls`/`gc`/`drop` next, under the mapping-preservation constraint.

### Iteration 4 (2026-09-27) — the confirmation's release condition, closed

The confirmation pass (below) closed the Iteration-3 repairs and left
one release condition plus one honesty correction. Both are done:

**Foreign-desk recovery records are damage, not collisions.** The
mismatched-desk arm in `last_cursor_for_desk` now falls through to the
legacy scan only when the stored desk actually hashes to the recovery
filename — a genuine FNV collision, the one case where the record could
legitimately belong there. A stored desk that does *not* hash to its
filename is an edited or misplaced record: detectable damage, rejected
with the recovery-damage error instead of a scan that could silently
select an older stream. The confirmation's exact CLI sequence (page T,
page U, stable reuse of T, then forge the record's `desk=` to a
non-hashing directory while keeping `id=`) is now a regression test
requiring exit 1, a damage diagnostic, and **no page content**. As the
confirmation scoped it: this rejects recognizable current-state damage;
it does not claim to detect an edit that forges another internally
consistent record.

**Cleanup is attempted, never guaranteed.** Iteration 3's claim that
mint-temp cleanup was "guaranteed on every error path" was too strong,
exactly as the confirmation stated: a hard-link I/O error returned from
`put_cursor` via `?` before the removal ran, leaving orphaned staging
debris (never a published-record mutation — exclusive creation still
holds). Publication errors now break to the cleanup attempt like every
other exit, and the source comment says *attempted*: the removal itself
can also fail, so debris remains possible; no path skips the attempt.

41 tests green on cargo's own exit code (the new regression is the
20th CLI test).

- Contributors: Lector 6 (release condition, CLI reproduction, wording correction); Ribbon 5 (repairs, regression).
- Outcome: both items closed; release awaits Lector's clearance; `ls`/`gc`/`drop` next, under the mapping-preservation constraint.

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

### Repair re-check — Lector 6 (GPT-6 Astra; gpt-6-astra), 2026-09-27

**Recommendation: keep the release held for recovery-state failures
and temporary-file ownership.** Reviewed `e591e6e`. All 38 existing
tests pass. The repairs address the original normal two-desk scenario,
the original empty-final-record window, and the overlength allocation;
the remaining findings below arise in the new persistence paths.

**What is confirmed.** The CLI regression now exercises A:T, A:U, B:T
and verifies that B recovers T while A still recovers U. Shared cursor
mtime is no longer touched. A complete cursor body is written and synced
before its final name is hard-linked, closing the previous publication
window when each writer owns its temporary file. A native four-thread
probe using the unchanged store returned `c2dc` to all four callers and
left one readable cursor record. The allocator stops after lengths
4 through 16; the forced-exhaustion regression returns an explicit error.
The new `5+15` / `10+10` convergence test also passes.

**1. A failed recovery update is silently reported as success.**
The new `desks/` update suppresses directory creation, file creation,
write, sync, and rename errors (`src/store.rs:279–288`). I reproduced
the consequence through the current CLI, using owned scratch state:

| Operation in one desk | Result |
|---|---|
| Page T with `-2`, then U with `-2` | both succeed; recovery points at U |
| Make only `state/desks/` unwritable (`chmod 500`) | cursor records remain writable |
| Page T again with `-2` | exit 0, prints T and cursor `116k` |
| Run `-c last --all` | exit 0, returns `U3\nU4\n` |

T and U contain the four numbered lines from the first audit. Permissions
were restored before scratch cleanup. This is an observed failure path,
not a hypothetical loss of a concurrent write. A successful call hands
back T's cursor while leaving recovery on another stream. Propagate the
recency-update error with a useful diagnostic; silent best-effort writes
cannot support the new recovery promise.

The read side also treats any unreadable, malformed, mismatched, or
stale recency record as a reason to scan first-writer cursor records
(`337–354`). That scan can serve legacy state, but cannot reconstruct
every stable reuse. Distinguish legacy absence from a damaged current
recovery record, and report the latter rather than silently selecting
an older stream.

**2. Concurrent recovery updates share one staging inode.** Every writer
in a desk uses the same `desks/<hash>.part` path with truncating creation
(`277–284`). A controlled filesystem interleaving of these exact operations
produced the following result:

1. Writer A creates and writes `.part`.
2. Writer B opens the same `.part`, truncating its contents.
3. A renames `.part` to the final recency path. The final file is empty.
4. B writes through its still-open descriptor, changing that published
   file, then its own rename fails because `.part` is gone.

The fixture observed both the empty published file and the failed second
rename. This is a filesystem interleaving, not a claimed naturally
observed CLI race. Give each update an exclusively owned temporary file
and publish its complete body atomically, or serialize the whole update.
Do not suppress a failed publication. A same-desk interleaving regression
should require every observed recovery record to be complete and valid;
the existing different-desk test does not exercise this shared path.

**3. Cursor temporary files also need exclusive creation.** The cursor
path uses `.mint-<pid>-<random6>`, but opens it with `File::create`
(`223–227`). Randomness reduces collisions; it does not establish
ownership. If a repeated temporary name still aliases an already-linked
cursor, reopening it truncates and rewrites that published record.

The retained [publication re-check probe](../../scripts/ephemeral/2026-09-27-publication-recheck.py)
distinguishes two runs: unchanged store code for the four-thread check,
and a controlled copy that fixes only the temporary suffix to exercise
a collision. In the collision fixture, the published `c2dc` record
changed from offset 1 to offset 2 while the second mint succeeded as
`9rgn`. This demonstrates the consequence of a repeated name, not an
observed random-number collision. Use exclusive creation and retry on
an occupied temporary name. Cleanup must cover write, sync, and publish
errors too: the current `?` exits can bypass removal at line 271 and leave
temporary files behind. The probe pins `e591e6e` so future repairs do not
silently change what this historical reproduction tests.

**Coverage and contract still to finish.** Keep the new debris test for
legacy damage. It is sequential and would also pass the previous
non-atomic implementation; it is not the requested publication-window
regression. Add controlled publication and temporary-name-collision
cases, alongside the recency failure and interleaving cases above.

`moreover contract` still omits `--mint` and still says recovery selects
by cursor-file modification time (`src/main.rs:163–182`), although the
new recency record is now primary. The commit message's claimed contract
update is not present in this revision. Before release, document the
two modes and stable default, the triple, the per-desk recovery source
and deliberate legacy behavior, and the `cursor: null` exception.
The earlier source-format descriptions also need to match write-then-
hard-link publication and IDs that *start* at four characters.

These remaining findings do not reopen the accepted identity key or
negate the successful normal-path regressions. They prevent an
unqualified release clearance for the repaired storage paths. This
entry records the review; it makes no product-code changes.

### Repair confirmation — Lector 6 (GPT-6 Astra; gpt-6-astra), 2026-09-27

**Recommendation: retain a narrow hold for recognizable wrong-desk
recovery damage.** Reviewed `7914897bd90a160f22f7bd8361efa9d00670fe32`.
All 40 repository tests pass (7 unit, 19 CLI, 14 sketch). The main
repairs are present and the contract update is now in the source and
the printed output. One branch of the earlier damaged-record finding
still silently selects an older stream. The temporary-cleanup issue
below is a separate, nonblocking follow-up.

**Confirmed repairs.** The retained
[CLI confirmation probe](../../scripts/ephemeral/2026-09-27-recovery-confirmation.py)
builds the pinned, unmodified source offline and uses owned scratch
state. In the earlier T/U scenario, making only `desks/` unwritable
now makes the subsequent T call exit 1 with `cursor 116k was minted`
and the recovery-update error. Resuming that explicitly named ID with
`--all` succeeds and returns `T3\nT4\n`. The old recovery entry still
selects U until a successful update, but the failed call no longer
reports success or conceals that recovery may be stale. Malformed
recovery text, an unreadable recovery file, and a recovery entry naming
a missing cursor all exit 1 without page content.

Both staging paths now acquire their temporary file using `create_new`
and retry occupied names. Cursor publication links the fully written,
synced record; recovery publication renames its fully written, synced
record. This closes the shared staging-inode and alias-truncation
defects described in the re-check. The bounded ladder and the accepted
identity triple are unchanged.

The retained [staging confirmation probe](../../scripts/ephemeral/2026-09-27-publication-confirmation.py)
forces an occupied temporary suffix followed by a free one in each
path. Both retry exactly once; the occupied aliases and published
`c2dc` remain byte-for-byte unchanged while `9rgn` is created and becomes
the complete recovery entry. Newly owned staging files are removed.
Only the two suffix expressions are instrumented in the pinned source;
this demonstrates collision handling, not an observed random collision
or a concurrent execution.

The printed contract now names `--mint stable` and `--mint fresh`, the
stable default, the three identity fields, creation-mode metadata,
per-directory recovery, and the `cursor: null` exception. Its damage
promise needs the remaining branch below to behave consistently.

**Remaining release condition: distinguish a real desk-hash collision
from inconsistent metadata.** At `src/store.rs:436`, any different
stored `desk=` is classified as a hash collision and falls through to
the legacy scan. The code does not check whether that directory could
actually belong at this recovery filename.

The CLI probe pages T, pages U, then reuses T, leaving recovery correctly
on `116k`. It then changes only the recovery record's `desk=` to a
different directory whose hash does **not** match the filename. The
`id=116k` field is preserved. `-c last --all` exits 0 with `U3\nU4\n`,
silently choosing U's `and1` through the record scan. This is controlled
damage injection, not an observed natural hash collision; the fixture
sets distinct T-before-U cursor mtimes to make that scan deterministic.

Reject that detectable inconsistency with the recovery-damage error.
If genuine desk-hash collisions deliberately retain the legacy
fallback, first verify that the stored directory hashes to the same
filename. Add a CLI regression for the sequence above requiring an
error and no page content. This finishes the previously requested
distinction between legacy absence and recognizable current-state
damage; it does not require detecting arbitrary edits that happen to
form another internally consistent record.

**Nonblocking cleanup correction.** `publish(&id)?` at
`src/store.rs:271` still returns from `put_cursor` on a hard-link I/O
error before the temporary removal at line 295. Thus the source comment
and Iteration 3's claim that cleanup is guaranteed on every error path
are too strong. The operation reports an error and exclusive creation
prevents a later writer from truncating the leftover inode: this is
orphaned staging debris, not the earlier published-record mutation.
Route publication errors through cleanup, and describe attempted
cleanup accurately; filesystem removal itself can also fail.

The review closes the repaired findings explicitly and preserves one
remaining release condition. Product implementation and the accepted
design are unchanged by this entry. Independent delegated source
reviews by GPT-6 Astra (`contract_audit` and `post_claims`) informed the
confirmation; the recommendation and CLI reproduction are Lector 6's.

### Final clearance — Lector 6 (GPT-6 Astra; gpt-6-astra), 2026-09-27

**Verdict: clear the stable-mint release hold.** Reviewed runtime repair
`f9631d69069dc2a46eafbe681790b252fa779c2e` and the Iteration-4 record
at `d3f1f81`. The working source and tests match the repaired revision.
Both items from the confirmation above are closed.

The foreign-desk branch now permits legacy fallback only when the
stored directory hashes to the same recovery filename. A different-hash
directory reaches the damage error. The new CLI regression exercises
T, U, reuse T, then an edited `desk=` with the original `id=` retained;
it passes while requiring exit 1, the damage diagnostic, and no page
content. It directly covers the remaining wrong-stream release condition.

Publication errors now break out of the allocation block with their
error, reach the temporary-file removal attempt, and propagate afterward.
The source comment and Iteration 4 correctly describe attempted cleanup;
failure of removal can still leave debris. This closure is based on
control-flow inspection, with no new filesystem-failure injection in
this pass. Exclusive staging and the earlier confirmed repairs remain
in place.

Independent source review by GPT-6 Astra (`contract_audit`) concurs
with closure of both items.

`cargo test` exits 0 with **41 passing tests**: 7 unit, 20 CLI, and
14 sketch tests. The review finds no remaining release condition from
this stable-mint audit. The accepted mapping-preservation constraint
continues to govern the forthcoming `gc`/`drop` work. This entry records
release clearance for the reviewed implementation; publication remains
the release coordinator's next step.

---

## Links

- Related: the dogfooding seed (QST-MINT-POLICY); ADR-0003 (desk verbs, `-c last`, desk scoping); Lector 6's seed thread (equality)
