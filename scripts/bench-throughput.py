#!/usr/bin/env python3
"""bench-throughput.py — moreover's paging-cost benchmark (ADR-0006's gate).

Supersedes scripts/bench-throughput.sh, rebuilt to Lector 6's measurement
specification (dogfooding seed, 2nd/3rd passes):
  - ONE platform timer, probed once up front (/usr/bin/time -l vs -v),
    with its GNU kbytes field parsed correctly ($6-equivalent);
  - a SINGLE monotonic measuring process (this interpreter) — no
    per-timestamp interpreter spawns; a calibration row (`true`) makes
    the residual harness overhead visible instead of pretended away;
  - fresh-ingestion (new state dir, warm input) measured separately from
    repeat-ingestion (existing spool) — the .sh mislabeled the latter;
  - every planned case is REPORTED with a status; failures and skips are
    explicit, never silently omitted;
  - 3 samples per case, median reported with min–max spread;
  - run metadata: binary sha256 + git revision (+dirty), platform,
    input bytes, so "0.2.0" can't hide which build ran.

Usage: scripts/bench-throughput.py [--bin PATH]   (default target/release/moreover)
Writes only under a scratch dir; prints markdown to stdout.
"""

import argparse, hashlib, os, platform, re, shutil, subprocess, sys, tempfile, time
from pathlib import Path
from statistics import median

def sh(args, **kw):
    return subprocess.run(args, capture_output=True, text=True, **kw)

def pick_timer():
    """Select the platform's rusage wrapper once; return (argv, parser)."""
    probe = sh(["/usr/bin/time", "-l", "true"])
    if probe.returncode == 0 and "maximum resident set size" in probe.stderr:
        rx = re.compile(r"^\s*(\d+)\s+maximum resident set size", re.M)
        return ["/usr/bin/time", "-l"], lambda s: (int(m.group(1)) // (1024 * 1024) if (m := rx.search(s)) else None)  # bytes→MiB
    probe = sh(["/usr/bin/time", "-v", "true"])
    if probe.returncode == 0 and "Maximum resident set size" in probe.stderr:
        rx = re.compile(r"Maximum resident set size \(kbytes\):\s*(\d+)")
        return ["/usr/bin/time", "-v"], lambda s: (int(m.group(1)) // 1024 if (m := rx.search(s)) else None)  # kbytes→MiB
    return None, lambda s: None  # no rusage wrapper: elapsed only, RSS reported as '?'

class Bench:
    def __init__(self, bin_path):
        self.bin = str(bin_path)
        self.timer_argv, self.rss_parse = pick_timer()
        self.rows = []

    def measure_once(self, argv, env, stdin=None):
        full = (self.timer_argv or []) + argv
        t0 = time.monotonic_ns()
        r = subprocess.run(full, input=stdin, capture_output=True, env=env)
        t1 = time.monotonic_ns()
        rss = self.rss_parse(r.stderr.decode("utf-8", "replace")) if self.timer_argv else None
        return (t1 - t0) / 1e6, rss, r.returncode, r.stderr.decode("utf-8", "replace")[-300:]

    def case(self, name, argv, env, samples=3, stdin=None):
        """Run a case `samples` times; report median elapsed, spread, max RSS."""
        ms, rss, fail = [], [], None
        for _ in range(samples):
            el, r, code, err = self.measure_once(argv, env, stdin)
            if code != 0:
                fail = (code, err.strip().splitlines()[-1] if err.strip() else "")
                break
            ms.append(el)
            if r is not None:
                rss.append(r)
        if fail:
            self.rows.append((name, f"FAILED exit {fail[0]}: {fail[1]}", "—"))
        else:
            spread = f"{median(ms):.0f} ms ({min(ms):.0f}–{max(ms):.0f})"
            self.rows.append((name, spread, f"{max(rss)} MiB" if rss else "?"))

    def skip(self, name, why):
        self.rows.append((name, f"SKIPPED: {why}", "—"))

def cursor_of(bin_path, env, argv):
    r = sh([bin_path] + argv, env=env)
    m = re.search(r"cursor: ([a-z0-9]+)>", r.stderr)
    return m.group(1) if (r.returncode == 0 and m) else None

def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--bin", default="target/release/moreover")
    a = ap.parse_args()
    binp = Path(a.bin)
    if not binp.is_file():
        sys.exit("build first: cargo build --release")

    scratch = Path(tempfile.mkdtemp(prefix="moreover-bench-"))
    try:
        b = Bench(binp)
        sha = hashlib.sha256(binp.read_bytes()).hexdigest()[:16]
        rev = sh(["git", "rev-parse", "--short", "HEAD"]).stdout.strip()
        dirty = "+dirty" if sh(["git", "status", "--porcelain", "src", "Cargo.toml"]).stdout.strip() else ""
        ver = sh([str(binp), "--version"]).stdout.strip()

        print(f"## moreover throughput — {ver}")
        print(f"- binary: sha256:{sha} · built at git {rev}{dirty} · {platform.platform()}")
        print(f"- timer: {' '.join(b.timer_argv) if b.timer_argv else 'none (elapsed only)'}"
              f" · single-process monotonic clock · 3 samples/case, median (min–max)")
        print(f"- page size fixed: 10 lines / 4096 bytes · {time.strftime('%Y-%m-%dT%H:%MZ', time.gmtime())}")
        print()

        env0 = dict(os.environ)
        env0.pop("MOREOVER_TRAILER", None)

        # calibration: the harness + wrapper overhead, stated not subtracted
        b.case("calibration (`true`)", ["true"], env0) if shutil.which("true") else b.skip("calibration", "no true(1)")

        for n in (100_000, 1_000_000, 5_000_000):
            label = f"{n//1000}k" if n < 1_000_000 else f"{n//1_000_000}M"
            corpus = scratch / f"corpus-{label}.txt"
            corpus.write_text("".join(f"line {i} of the benchmark corpus\n" for i in range(1, n + 1)))
            nbytes = corpus.stat().st_size
            # warm the input file itself once (input-cache vs spool-existence are separate conditions)
            corpus.read_bytes()

            # fresh ingestion: brand-new state dir EACH sample (spool must not exist)
            fresh_ms, fresh_rss, fail = [], [], None
            for i in range(3):
                st = scratch / f"state-fresh-{label}-{i}"
                env = {**env0, "MOREOVER_STATE_DIR": str(st)}
                el, r, code, err = b.measure_once([str(binp), str(corpus), "-10"], env)
                if code != 0:
                    fail = (code, err.strip().splitlines()[-1] if err.strip() else "")
                    break
                fresh_ms.append(el)
                if r is not None:
                    fresh_rss.append(r)
            if fail:
                b.rows.append((f"FRESH ingestion+first page, {label} lines ({nbytes/1e6:.0f} MB)", f"FAILED exit {fail[0]}: {fail[1]}", "—"))
            else:
                b.rows.append((f"FRESH ingestion+first page, {label} lines ({nbytes/1e6:.0f} MB)",
                               f"{median(fresh_ms):.0f} ms ({min(fresh_ms):.0f}–{max(fresh_ms):.0f})",
                               f"{max(fresh_rss)} MiB" if fresh_rss else "?"))

            # repeat ingestion + resumes share one persistent state dir
            st = scratch / f"state-{label}"
            env = {**env0, "MOREOVER_STATE_DIR": str(st)}
            sh([str(binp), str(corpus), "-10"], env=env)  # create the spool
            b.case(f"REPEAT ingestion (existing spool)+first page, {label} lines", [str(binp), str(corpus), "-10"], env)

            early = cursor_of(str(binp), env, [str(corpus), "-10"])
            b.case(f"resume EARLY -10, {label} lines", [str(binp), "-c", early, "-10"], env) if early \
                else b.skip(f"resume EARLY -10, {label} lines", "no early cursor minted")

            late = cursor_of(str(binp), env, [str(corpus), "--bytes", str(nbytes * 9 // 10)])
            if late:
                b.case(f"resume LATE (~90% offset) -10, {label} lines", [str(binp), "-c", late, "-10"], env)
                b.case(f"resume LATE --bytes 4096, {label} lines", [str(binp), "-c", late, "--bytes", "4096"], env)
                b.case(f"resume LATE -10 --overlap 5, {label} lines", [str(binp), "-c", late, "-10", "--overlap", "5"], env)
            else:
                for nm in ("-10", "--bytes 4096", "-10 --overlap 5"):
                    b.skip(f"resume LATE {nm}, {label} lines", "no late cursor minted")

        # one very long line, ~10 MB: fixed line counts do not bound output bytes
        longline = scratch / "longline.txt"
        longline.write_text("x" * 10_000_000 + "\n")
        st = scratch / "state-longline-fresh"
        b.case("FRESH ingestion+first page, one 10MB line",
               [str(binp), str(longline), "-10"], {**env0, "MOREOVER_STATE_DIR": str(st)})
        env = {**env0, "MOREOVER_STATE_DIR": str(scratch / "state-longline")}
        lc = cursor_of(str(binp), env, [str(longline), "--bytes", "100"])
        b.case("resume --bytes 4096, one 10MB line", [str(binp), "-c", lc, "--bytes", "4096"], env) if lc \
            else b.skip("resume --bytes 4096, one 10MB line", "no cursor minted")

        print("| case | elapsed, median (min–max) | peak RSS |")
        print("|---|---|---|")
        for name, elapsed, rss in b.rows:
            print(f"| {name} | {elapsed} | {rss} |")
        print("\n(scratch state and corpora deleted; nothing persisted)")
    finally:
        shutil.rmtree(scratch, ignore_errors=True)

if __name__ == "__main__":
    main()
