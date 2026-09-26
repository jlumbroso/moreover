<!-- adr template version: "adr 3.11.5" -->

# ADR-0006: Bounded-read resume — benchmark-gated

- **Date**: 2026-09-26
- **Iteration**: 1
- **Status**: Accepted
- **Deciders**: Jérémie Lumbroso (the gate ruling); Ribbon 5 (implementation); Lector 6 (measurement corrections the gate consists of)

**TL;DR**: Resume stops reading the whole spool. The change is
authorized behind one gate, in his words: making the improvement without
first fixing the benchmark "is motivated reasoning! That's not
good/acceptable" — so Lector's measurement corrections land first, the
baseline is re-run, then the seek implementation ships against it.

---

## Originating Context

**Source**: the dogfooding seed's QST-SEEK-GO ANS (Jérémie, 2026-09-26,
option B), chunked at his request. The evidence base: the current resume
reads the entire spool per invocation (confessed in the seed thread;
measured at ~176 MiB peak RSS to deliver ten lines of a 5M-line input —
the memory figure Lector's audit did not dispute). The gate: the timing
side of that benchmark is not yet trustworthy (two-process timer
overhead ~14 ms; the "first page" rows re-ingested an existing spool;
the GNU-time parser bug is fixed but the run predates the fix; failures
could silently omit cases).

## Decision

1. **The gate (first)**: rebuild `scripts/bench-throughput.sh` to
   Lector's specification — one platform timer selected up front and a
   single monotonic measuring process; fresh-saved-state ingestion
   measured separately from warm-input repeat ingestion; every case's
   status reported, failures explicit, never silently omitted; recorded
   run metadata (binary revision, platform, input bytes, resume offset,
   requested output, cache/spool condition). Re-run → the honest
   before-picture, staked in this ADR at Iteration 2.
2. **The implementation (second)**: spool metadata written once at drain
   time (total bytes, total lines); resume seeks to the cursor's byte
   offset and reads only the requested page plus overlap (line-mode
   overlap retreats via bounded backward scan); trailers use stored
   totals. The public claim, in Lector's discipline: **work proportional
   to the requested bytes plus indexing/overlap** — never "O(1)", and
   one line can be huge.
3. **The after-picture (third)**: same benchmark, same metadata, staked
   beside the before; only then does the benchmark graduate to a
   regression gate.

### Consequences

- The 176 MiB class of cost disappears from resume; first ingestion
  remains proportional to input size by nature (totals must be counted).
- The v0 "drain fully" trade-off (ADR-0002) is untouched — this changes
  *resume*, not the live-stream question (`Y = ?` stays deferred).
- The benchmark becomes the standing guard for every future performance
  claim, including any paper's.

## Questions

*NOT: no open questions — the ruling and the gate are complete; the
work is sequenced in Action Items, and anything the corrected baseline
surprises us with lands here as an iteration.*

## Action Items

- [ ] Benchmark rebuild to Lector's spec - Owner: Ribbon 5, audit invited: Lector 6
- [ ] Corrected baseline run, staked (Iteration 2) - Owner: Ribbon 5
- [ ] Seek implementation + spool metadata + tests - Owner: Ribbon 5
- [ ] After-picture run + graduation of the benchmark to regression gate (Iteration 3) - Owner: Ribbon 5

## Iterations

### Iteration 1 (2026-09-26)
- Trigger: QST-SEEK-GO answered (B) in the dogfooding seed; chunked here at his request.
- Contributors: Jérémie (the gate ruling, verbatim); Ribbon 5 (record, sequencing); Lector 6 (the measurement spec the gate adopts).
- Outcome: `— → Accepted`; work sequenced.

---

## Links

- Related: the dogfooding seed (QST-SEEK-GO; Lector's benchmark audit, second pass); `scripts/bench-throughput.sh`; ADR-0002 (the drain-fully trade-off this does not touch)
