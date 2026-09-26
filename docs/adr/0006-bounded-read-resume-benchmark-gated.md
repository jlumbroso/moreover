<!-- adr template version: "adr 3.11.5" -->

# ADR-0006: Bounded-read resume — benchmark-gated

- **Date**: 2026-09-26
- **Iteration**: 3
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

### Iteration 2 (2026-09-26) — the corrected baseline (the before-picture)

`scripts/bench-throughput.py` (supersedes the .sh) implements the full
spec: one probed platform timer, single-process monotonic clock,
calibration row, fresh-vs-repeat ingestion separated, 3 samples/case
with median (min–max), explicit failure/skip reporting, binary
sha256+git-rev metadata. The staked run, verbatim:

binary sha256:32f07bb7fef30022 · git 5167f9a · macOS-26.2-arm64 ·
/usr/bin/time -l · 2026-09-26T22:16Z · pages fixed 10 lines / 4096 B

| case | elapsed, median (min–max) | peak RSS |
|---|---|---|
| calibration (`true`) | 5 ms (5–5) | 1 MiB |
| FRESH ingestion+first page, 100k lines (3 MB) | 27 ms (24–28) | 4 MiB |
| REPEAT ingestion (existing spool)+first page, 100k lines | 17 ms (16–17) | 4 MiB |
| resume EARLY -10, 100k lines | 11 ms (11–12) | 4 MiB |
| resume LATE (~90%) -10, 100k lines | 14 ms (13–14) | 4 MiB |
| resume LATE --bytes 4096, 100k lines | 12 ms (11–12) | 4 MiB |
| resume LATE -10 --overlap 5, 100k lines | 14 ms (14–14) | 5 MiB |
| FRESH ingestion+first page, 1M lines (36 MB) | 87 ms (85–87) | 35 MiB |
| REPEAT ingestion (existing spool)+first page, 1M lines | 70 ms (70–73) | 35 MiB |
| resume EARLY -10, 1M lines | 24 ms (23–24) | 35 MiB |
| resume LATE (~90%) -10, 1M lines | 35 ms (33–36) | 35 MiB |
| resume LATE --bytes 4096, 1M lines | 23 ms (22–23) | 35 MiB |
| resume LATE -10 --overlap 5, 1M lines | 53 ms (51–55) | 44 MiB |
| FRESH ingestion+first page, 5M lines (184 MB) | 374 ms (373–411) | 176 MiB |
| REPEAT ingestion (existing spool)+first page, 5M lines | 325 ms (321–330) | 176 MiB |
| resume EARLY -10, 5M lines | 86 ms (84–87) | 176 MiB |
| resume LATE (~90%) -10, 5M lines | 155 ms (140–192) | 176 MiB |
| resume LATE --bytes 4096, 5M lines | 93 ms (91–98) | 176 MiB |
| resume LATE -10 --overlap 5, 5M lines | 222 ms (218–225) | 213 MiB |
| FRESH ingestion+first page, one 10MB line | 46 ms (45–52) | 11 MiB |
| resume --bytes 4096, one 10MB line | 18 ms (18–19) | 11 MiB |

Reading, in the ruled wording discipline: resume elapsed and peak RSS
scale with saved-input size, not requested output (86–222 ms and
176–213 MiB to deliver ten lines of 184 MB); the seek implementation's
target is work proportional to requested bytes plus indexing/overlap.
Lector 6's audit of this rebuild is invited before the after-picture
claims anything.

- Contributors: Ribbon 5 (rebuild, run); Lector 6 (the spec; audit pending).
- Outcome: gate step 2 complete; implementation (step 3) may begin.

#### Independent audit — Lector 6 (GPT-6 Astra; gpt-6-astra), 2026-09-26

**Disposition: the main-corpus baseline is corroborated; the full-spec
claim still needs correction.** I reviewed benchmark revision `3544223`
and reran all 21 rows using the existing release binary whose SHA-256
begins `32f07bb7fef30022`, matching the recorded fingerprint. The
[complete rerun and full hash](../benchmarks/2026-09-26-lector-baseline-recheck.md)
are retained. Every row completed without a reported failure or skip.
This review concerns the measurement gate; it does not replace the
human's seek decision or establish any after-picture result.

**What is fixed.** Timer selection is separate from workload success;
the timestamps come from one monotonic process; the GNU RSS parser reads
the numeric field with the correct units; and each fresh sample for the
100k-, 1M-, and 5M-line corpora uses a new state directory. Calibration,
three-sample spreads, and missing-cursor rows are now visible. My rerun's
early-resume medians were 12, 25, and 78 ms, with peak RSS of 4, 35, and
176 MiB. For all three, the generated input and current paging semantics
give the same start offset, 311 bytes, and the same 320-byte output
(lines 11–20). Together with the whole-spool read in `src/store.rs`,
these measurements support the saved-input-size cost that motivates seek.
The calibration row was 6 ms; it describes overhead, not a correction
factor to subtract from a future fast result.

**1. The long-line fresh row still mixes conditions.**
`scripts/bench-throughput.py:159–161` supplies one state directory to
`Bench.case`, which repeats the command three times. A small probe of
that exact call shape found no spool before the first sample and the
same spool inode before both later samples. The reported median therefore
combines one fresh ingestion and two repeats. Give each fresh sample its
own state and rerun this row against the baseline binary before comparing
it with seek. This finding does not invalidate the separate byte-resume
row or the fresh-state loops for the three multi-line corpora.

**2. Finish preserving setup failures and diagnostics.** The repeat
spool preparation at line 140 ignores its command result; `cursor_of`
discards the setup command's status and diagnostic. A generic "no cursor
minted" skip makes the missing measurement visible but loses its cause.
Timed failures also keep only the final 300 stderr characters and then
the final line. In a native macOS probe, moreover rejected an unknown
flag with exit 2 and `unknown argument: --lector-audit-unknown-flag`;
the benchmark reported `FAILED exit 2: ... peak memory footprint` instead.
The timer's statistic displaced the actual error.

Check each preparation command before labeling dependent work as ready;
retain its status and original diagnostic, and retain workload stderr
separately from timer output. A failed preparation should leave dependent
rows explicitly skipped with that cause. These are remaining parts of
the adopted reporting specification; the successful rerun above does
not exercise those failure paths.

**3. Separate binary identity from checkout identity, and record exact
workloads.** The executable hash is useful. The `built at git` value is
obtained from the caller's checkout, however, without checking which
revision built `--bin`. My rerun printed `3544223` for the same binary
fingerprint the earlier run labeled `5167f9a`. Call this **checkout
revision at run time**; record build provenance separately if it is
known. Also identify the benchmark revision or hash, including local
script changes: the current dirty check covers only `src` and
`Cargo.toml`.

The table rounds input sizes to MB and labels late offsets approximately.
The exact values below are derived from the committed generator, not
metadata emitted by the benchmark:

| Corpus | Input bytes | Early resume offset | Late resume offset |
|---|---:|---:|---:|
| 100k lines | 3,488,895 | 311 | 3,140,005 |
| 1M lines | 35,888,896 | 311 | 32,300,006 |
| 5M lines | 183,888,896 | 311 | 165,500,006 |

The long-line input is 10,000,001 bytes; its byte-resume starts at 100.
The late corpus offsets fall inside lines. Record these values alongside
the requested unit, count, overlap, actual output bytes, and state/cache
conditions so that the before and after exercise the same work. Preserve
the partial-line behavior when checking the seek implementation. The
long-line `-10` case emits the entire input, and `-10 --overlap 5` includes
overlap; neither is simply "ten lines delivered" in the same sense as
the early-resume cases. Input files are recently written/read here; fresh
saved state does not mean a cold operating-system cache.

**Before the after-picture is published.** Correct the mixed long-line
case, finish failure reporting, and record the missing provenance and
workload fields. Keep or reproduce an identified baseline executable
and run both implementations with the corrected harness. Verify expected
page bytes and trailer/cursor behavior outside the timing interval:
the current harness captures stdout but does not check it, so a success
status alone cannot establish equal work. Publish the measured changes
for those matched cases, with their sample spreads.

For later graduation to an automated regression gate, failures and
required skips must also affect the benchmark's exit status, and the
comparison needs an explicit acceptance rule. Currently `main` returns
success even when a row says `FAILED`; no performance threshold is
enforced. That is a separate step from collecting a descriptive baseline.

The public claim I can support now is: **"In two macOS runs, resuming
the same 320 bytes from saved inputs of 100k, 1M, and 5M lines used
approximately 4, 35, and 176 MiB of peak resident memory. The current
implementation reads the entire saved input on each resume."** The
anticipated memory reduction remains a target until the after-picture
measures it. The earlier table remains useful evidence with the long-line
fresh row excluded; it should not yet be described as the full spec met.

### Iteration 3 (2026-09-26) — audit findings applied; baseline v2 is the before-picture of record

All three audit findings are implemented in the harness: (1) the
long-line fresh row uses a new state directory per sample; (2) timer
stats go to their own file (`-o`), preparation commands are checked
rows, and skips carry their prep-failure cause — the fake-flag probe's
displacement class is closed; (3) provenance is labeled honestly
("checkout revision at run time" beside the binary's sha256 identity,
plus the bench script's own hash and a scripts-inclusive dirty check),
and every resume row carries its exact byte offset with exact input
bytes. Baseline v2, the run the after-picture must match case-for-case:

binary sha256:32f07bb7fef30022 · checkout at run: git b4b3f99+dirty ·
bench script sha256:c4b0c6e33375 · /usr/bin/time -l · macOS-26.2-arm64 ·
2026-09-26T22:53Z

| case | elapsed, median (min–max) | peak RSS |
|---|---|---|
| calibration (`true`) | 5 ms (4–6) | 1 MiB |
| FRESH ingestion+first page, 100k lines (3,488,895 B) | 29 ms (26–34) | 4 MiB |
| REPEAT ingestion (existing spool)+first page, 100k lines | 17 ms (17–18) | 4 MiB |
| resume EARLY -10 (offset 311), 100k lines | 12 ms (12–12) | 4 MiB |
| resume LATE -10 (offset 3140005), 100k lines | 14 ms (13–15) | 4 MiB |
| resume LATE --bytes 4096 (offset 3140005), 100k lines | 12 ms (11–12) | 4 MiB |
| resume LATE -10 --overlap 5 (offset 3140005), 100k lines | 15 ms (14–16) | 5 MiB |
| FRESH ingestion+first page, 1M lines (35,888,896 B) | 86 ms (83–86) | 35 MiB |
| REPEAT ingestion (existing spool)+first page, 1M lines | 71 ms (70–75) | 35 MiB |
| resume EARLY -10 (offset 311), 1M lines | 24 ms (23–24) | 35 MiB |
| resume LATE -10 (offset 32300006), 1M lines | 35 ms (34–37) | 35 MiB |
| resume LATE --bytes 4096 (offset 32300006), 1M lines | 24 ms (23–26) | 35 MiB |
| resume LATE -10 --overlap 5 (offset 32300006), 1M lines | 51 ms (51–51) | 44 MiB |
| FRESH ingestion+first page, 5M lines (183,888,896 B) | 366 ms (365–686) | 176 MiB |
| REPEAT ingestion (existing spool)+first page, 5M lines | 318 ms (312–321) | 176 MiB |
| resume EARLY -10 (offset 311), 5M lines | 74 ms (73–77) | 176 MiB |
| resume LATE -10 (offset 165500006), 5M lines | 133 ms (131–145) | 176 MiB |
| resume LATE --bytes 4096 (offset 165500006), 5M lines | 77 ms (76–77) | 176 MiB |
| resume LATE -10 --overlap 5 (offset 165500006), 5M lines | 221 ms (221–225) | 213 MiB |
| FRESH ingestion+first page, one 10MB line (emits whole line) | 49 ms (48–50) | 11 MiB |
| resume --bytes 4096 (offset 100), one 10MB line | 18 ms (17–19) | 11 MiB |

Still owed before graduation to a regression gate, per the audit's
closing section: output-content verification outside the timing
interval (a success status alone cannot establish equal work); an
explicit acceptance rule; and failures affecting the harness's exit
status. Those land with the after-picture (Action Items). The public
claim of record remains the audit's own sentence: in these macOS runs,
resuming the same 320 bytes from saved inputs of 100k/1M/5M lines used
approximately 4/35/176 MiB of peak resident memory; the current
implementation reads the entire saved input on each resume.

- Contributors: Lector 6 (audit, corroborating rerun, retained recheck record); Ribbon 5 (fixes, baseline v2).
- Outcome: measurement gate closed for the before-picture; seek implementation (step 3) proceeds.

---

## Links

- Related: the dogfooding seed (QST-SEEK-GO; Lector's benchmark audit, second pass); `scripts/bench-throughput.sh`; ADR-0002 (the drain-fully trade-off this does not touch)
