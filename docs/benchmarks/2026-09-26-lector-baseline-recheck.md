# Benchmark audit rerun — 2026-09-26

Run by Lector 6 (GPT-6 Astra; `gpt-6-astra`) with
`python3 scripts/bench-throughput.py` at benchmark revision
`3544223efe60f24ceb01d0d4b64baaea22169464`. The existing release binary
was used without rebuilding it. Its full SHA-256 was
`32f07bb7fef300221b195f65fbd03ba0c2eaaa8db3ea688891b15772ff674c38`,
matching the fingerprint recorded in ADR-0006 Iteration 2.

The stdout below is preserved verbatim. Two labels require the audit's
qualification: `built at git` is the checkout revision at run time,
and the long-line `FRESH` row mixes one fresh sample with two repeat
samples. This is evidence for the
[independent audit in ADR-0006](../adr/0006-bounded-read-resume-benchmark-gated.md#independent-audit--lector-6-gpt-6-astra-gpt-6-astra-2026-09-26),
not an endorsement of those labels. All 21 rows completed without a
reported failure or skip. No seek implementation was tested.

---

## moreover throughput — moreover 0.2.0
- binary: sha256:32f07bb7fef30022 · built at git 3544223 · macOS-26.2-arm64-arm-64bit
- timer: /usr/bin/time -l · single-process monotonic clock · 3 samples/case, median (min–max)
- page size fixed: 10 lines / 4096 bytes · 2026-09-26T22:21Z

| case | elapsed, median (min–max) | peak RSS |
|---|---|---|
| calibration (`true`) | 6 ms (6–6) | 1 MiB |
| FRESH ingestion+first page, 100k lines (3 MB) | 28 ms (26–30) | 4 MiB |
| REPEAT ingestion (existing spool)+first page, 100k lines | 17 ms (17–18) | 4 MiB |
| resume EARLY -10, 100k lines | 12 ms (12–12) | 4 MiB |
| resume LATE (~90% offset) -10, 100k lines | 13 ms (12–13) | 4 MiB |
| resume LATE --bytes 4096, 100k lines | 12 ms (12–15) | 4 MiB |
| resume LATE -10 --overlap 5, 100k lines | 16 ms (15–18) | 5 MiB |
| FRESH ingestion+first page, 1M lines (36 MB) | 90 ms (89–91) | 35 MiB |
| REPEAT ingestion (existing spool)+first page, 1M lines | 74 ms (73–98) | 35 MiB |
| resume EARLY -10, 1M lines | 25 ms (24–25) | 35 MiB |
| resume LATE (~90% offset) -10, 1M lines | 35 ms (35–37) | 35 MiB |
| resume LATE --bytes 4096, 1M lines | 24 ms (24–24) | 35 MiB |
| resume LATE -10 --overlap 5, 1M lines | 51 ms (50–52) | 44 MiB |
| FRESH ingestion+first page, 5M lines (184 MB) | 387 ms (379–446) | 176 MiB |
| REPEAT ingestion (existing spool)+first page, 5M lines | 322 ms (321–329) | 176 MiB |
| resume EARLY -10, 5M lines | 78 ms (76–79) | 176 MiB |
| resume LATE (~90% offset) -10, 5M lines | 142 ms (141–162) | 176 MiB |
| resume LATE --bytes 4096, 5M lines | 85 ms (84–85) | 176 MiB |
| resume LATE -10 --overlap 5, 5M lines | 231 ms (218–308) | 213 MiB |
| FRESH ingestion+first page, one 10MB line | 38 ms (38–49) | 11 MiB |
| resume --bytes 4096, one 10MB line | 17 ms (17–19) | 11 MiB |

(scratch state and corpora deleted; nothing persisted)
