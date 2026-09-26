# Baseline v2 audit rerun — 2026-09-26

Run by Lector 6 (GPT-6 Astra; `gpt-6-astra`) with
`python3 scripts/bench-throughput.py` at checkout
`a439b84cbf6fdf3a7f0699bde25d6b0568888de7`. The existing release binary
was used without rebuilding it. Both fingerprints match the Iteration 3
record:

- Binary SHA-256: `32f07bb7fef300221b195f65fbd03ba0c2eaaa8db3ea688891b15772ff674c38`
- Benchmark SHA-256: `c4b0c6e33375624f1d006e1ef3ffbd8bb3ad9daa12b01eae8cf76ebc4a7068ab`

All 21 rows completed without a reported failure or skip. The raw stdout
below is preserved verbatim. See the
[Iteration 3 re-check in ADR-0006](../adr/0006-bounded-read-resume-benchmark-gated.md)
for the remaining diagnostic-retention finding. This run tests the
baseline implementation; it contains no seek comparison.

---

## moreover throughput — moreover 0.2.0
- binary: sha256:32f07bb7fef30022 (identity) · checkout at run time: git a439b84 · bench script sha256:c4b0c6e33375 · macOS-26.2-arm64-arm-64bit
- timer: /usr/bin/time -l · single-process monotonic clock · 3 samples/case, median (min–max)
- page size fixed: 10 lines / 4096 bytes · 2026-09-26T22:56Z

| case | elapsed, median (min–max) | peak RSS |
|---|---|---|
| calibration (`true`) | 5 ms (5–5) | 1 MiB |
| FRESH ingestion+first page, 100k lines (3,488,895 B) | 31 ms (31–37) | 4 MiB |
| REPEAT ingestion (existing spool)+first page, 100k lines | 17 ms (17–20) | 4 MiB |
| resume EARLY -10 (offset 311), 100k lines | 14 ms (14–14) | 4 MiB |
| resume LATE -10 (offset 3140005), 100k lines | 13 ms (13–14) | 4 MiB |
| resume LATE --bytes 4096 (offset 3140005), 100k lines | 13 ms (13–15) | 4 MiB |
| resume LATE -10 --overlap 5 (offset 3140005), 100k lines | 16 ms (15–17) | 5 MiB |
| FRESH ingestion+first page, 1M lines (35,888,896 B) | 96 ms (96–102) | 35 MiB |
| REPEAT ingestion (existing spool)+first page, 1M lines | 77 ms (73–89) | 35 MiB |
| resume EARLY -10 (offset 311), 1M lines | 25 ms (24–52) | 35 MiB |
| resume LATE -10 (offset 32300006), 1M lines | 37 ms (37–93) | 35 MiB |
| resume LATE --bytes 4096 (offset 32300006), 1M lines | 27 ms (25–27) | 35 MiB |
| resume LATE -10 --overlap 5 (offset 32300006), 1M lines | 56 ms (53–68) | 44 MiB |
| FRESH ingestion+first page, 5M lines (183,888,896 B) | 415 ms (400–427) | 176 MiB |
| REPEAT ingestion (existing spool)+first page, 5M lines | 384 ms (342–479) | 176 MiB |
| resume EARLY -10 (offset 311), 5M lines | 86 ms (84–88) | 176 MiB |
| resume LATE -10 (offset 165500006), 5M lines | 167 ms (139–170) | 176 MiB |
| resume LATE --bytes 4096 (offset 165500006), 5M lines | 87 ms (83–99) | 176 MiB |
| resume LATE -10 --overlap 5 (offset 165500006), 5M lines | 224 ms (217–228) | 213 MiB |
| FRESH ingestion+first page, one 10MB line (emits whole line) | 50 ms (50–51) | 11 MiB |
| resume --bytes 4096 (offset 100), one 10MB line | 19 ms (18–20) | 11 MiB |

(scratch state and corpora deleted; nothing persisted)
