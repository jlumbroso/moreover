#!/usr/bin/env bash
# bench-throughput.sh — throughput benchmark for moreover's paging costs.
#
# Context (first dogfooding seed, 2026-09-23): before changing resume from
# whole-spool reads to seek+bounded reads, measure the current costs so the
# improvement is a number, not a claim. Design per Lector 6's thread notes:
# drain measured separately from resume; page size fixed while input size
# grows; early AND late resume offsets; line mode, byte mode, overlap, and
# a very-long-line specimen; elapsed time + peak RSS; warm-cache runs
# (each case runs twice, the second is reported).
#
# CAVEATS (Lector 6 audit, seed thread 2nd pass): elapsed includes ~14ms
# two-process timer overhead; the warmed "first page" reuses an existing
# spool (fresh-ingestion case pending); treat figures as indicative until
# the single-process-timer rerun. Not yet a regression gate.
# Usage: scripts/bench-throughput.sh [BIN]   (default: target/release/moreover)
# Writes nothing outside its scratch dir; prints a markdown table.
set -euo pipefail

BIN="${1:-target/release/moreover}"
[ -x "$BIN" ] || { echo "build first: cargo build --release" >&2; exit 2; }
SCRATCH=$(mktemp -d "${TMPDIR:-/tmp}/moreover-bench.XXXXXX")
trap 'rm -rf "$SCRATCH"' EXIT
export MOREOVER_STATE_DIR="$SCRATCH/state"

gen_lines() { # $1=count $2=path
  seq -f "line %.0f of the benchmark corpus" "$1" > "$2"
}

# one very long line: ~10MB, no newline until the end
gen_longline() {
  python3 - "$1" <<'EOF'
import sys
with open(sys.argv[1], "w") as f:
    f.write("x" * 10_000_000)
    f.write("\n")
EOF
}

# time_case NAME CMD... — runs twice (warm cache), reports 2nd run's
# elapsed ms and peak RSS (MB) from /usr/bin/time -l (macOS) or -v (linux).
time_case() {
  local name="$1"; shift
  "$@" >/dev/null 2>>"$SCRATCH/warmup.log" || true
  local t0 t1 rss
  t0=$(python3 -c 'import time; print(time.time_ns())')
  /usr/bin/time -l "$@" >/dev/null 2>"$SCRATCH/time.out" || \
    /usr/bin/time -v "$@" >/dev/null 2>"$SCRATCH/time.out"
  t1=$(python3 -c 'import time; print(time.time_ns())')
  # macOS line: "  180224  maximum resident set size" (bytes, value first);
  # GNU line: "Maximum resident set size (kbytes): 180224" (value is $6) —
  # parser corrected per Lector 6's fixture check (the first draft printed
  # 0MB on the GNU format).
  rss=$(awk '/maximum resident set size/{print int($1/1048576)"MiB"} /Maximum resident set size/{print int($6/1024)"MiB"}' "$SCRATCH/time.out" | head -1)
  printf "| %s | %d ms | %s |\n" "$name" $(( (t1 - t0) / 1000000 )) "${rss:-?}"
}

# cursor_of FILE PAGEARGS... — drains FILE, returns the printed cursor
cursor_of() {
  local f="$1"; shift
  "$BIN" "$f" "$@" 2>&1 >/dev/null | sed -n 's/.*cursor: \([a-z0-9]*\)>.*/\1/p'
}

echo "## moreover throughput — $($BIN --version) — $(date -u +%Y-%m-%dT%H:%MZ)"
echo "(second-of-two runs: warm filesystem cache; page size fixed at 10 lines / 4KB)"
echo
echo "| case | elapsed | peak RSS |"
echo "|---|---|---|"

for N in 100000 1000000 5000000; do
  F="$SCRATCH/corpus-$N.txt"
  gen_lines "$N" "$F"
  time_case "repeat ingestion+first page (existing spool), ${N} lines" "$BIN" "$F" -10
  EARLY=$(cursor_of "$F" -10)
  time_case "resume EARLY (-10), ${N} lines" "$BIN" -c "$EARLY" -10
  # a late cursor: jump most of the way in via a big byte page, then resume
  LATE=$(cursor_of "$F" --bytes $(( N * 30 ))) || true
  if [ -n "$LATE" ]; then
    time_case "resume LATE (-10), ${N} lines" "$BIN" -c "$LATE" -10
    time_case "resume LATE (--bytes 4096), ${N} lines" "$BIN" -c "$LATE" --bytes 4096
    time_case "resume LATE (-10 --overlap 5), ${N} lines" "$BIN" -c "$LATE" -10 --overlap 5
  fi
done

L="$SCRATCH/longline.txt"
gen_longline "$L"
time_case "repeat ingestion+first page (existing spool), one 10MB line" "$BIN" "$L" -10
LC=$(cursor_of "$L" --bytes 100)
[ -n "$LC" ] && time_case "resume (--bytes 4096), one 10MB line" "$BIN" -c "$LC" --bytes 4096

echo
echo "(state dir and corpora were scratch; nothing persisted)"
