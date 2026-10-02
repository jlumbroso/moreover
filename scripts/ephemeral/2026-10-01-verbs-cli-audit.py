#!/usr/bin/env python3
"""Audit destructive desk-verb behavior in the pinned 923178f CLI.

Authorship: Lector 6 (GPT-6 Astra; gpt-6-astra).
Run: python3 scripts/ephemeral/2026-10-01-verbs-cli-audit.py

Builds unmodified pinned source offline. Generated source, executable,
and state stay in an owned directory below this script and are removed
on exit. Requires git, cargo, and a non-root Unix user for the permission
fixture. Existing user state and repository runtime code are untouched.

Cases: stable reuse of an old record followed by gc; gc across two desks;
gc0 with a future record timestamp; drop of U while unrelated T's cursor
record is temporarily unreadable.
The mtime and permission changes are explicit fault/state fixtures.
Assertions preserve the observed defects, not desired product behavior;
successful execution is not release clearance.

Run by Lector 6 on 2026-10-01: all four cases reproduced. GC removed
the just-reused old cursor; gc0 from desk A removed desk B's cursor;
gc0 kept a future-dated record; drop U succeeded while removing unreadable
T's spool but keeping its record. Permissions were restored and all owned
scratch was cleaned.
"""

import json
import os
from pathlib import Path
import re
import subprocess
import tempfile
import time


REVISION = "923178f7fed534c16ef9096a11a07c0a0fc7c9d8"


def main():
    if os.name != "posix" or os.geteuid() == 0:
        raise SystemExit("Run as a non-root Unix user for the permission fixture.")
    home = Path(__file__).resolve().parent
    repo = home.parents[1]
    with tempfile.TemporaryDirectory(prefix=".verbs-cli-audit-", dir=home) as temporary:
        root = Path(temporary)
        source = root / "source"
        paths = subprocess.check_output(
            ["git", "ls-tree", "-r", "--name-only", REVISION,
             "--", "Cargo.toml", "Cargo.lock", "src"], cwd=repo, text=True,
        ).splitlines()
        for name in paths:
            path = source / name
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(subprocess.check_output(
                ["git", "show", f"{REVISION}:{name}"], cwd=repo,
            ))
        subprocess.run(
            ["cargo", "build", "--quiet", "--offline", "--locked",
             "--manifest-path", str(source / "Cargo.toml"),
             "--target-dir", str(root / "target")], check=True, timeout=120,
        )
        binary = root / "target" / "debug" / "moreover"
        env = {k: v for k, v in os.environ.items() if not k.startswith("MOREOVER_")}

        def invoke(desk, state, *args):
            return subprocess.run(
                [str(binary), *args, "--state-dir", str(state)], cwd=desk,
                env=env, stdin=subprocess.DEVNULL, capture_output=True,
                text=True, timeout=15,
            )

        def report(case, out, **extra):
            print(json.dumps({"case": case, "exit": out.returncode,
                              "stdout": out.stdout, "stderr": out.stderr, **extra}))

        def fixture(name):
            base = root / name
            a, b = base / "desk-a", base / "desk-b"
            a.mkdir(parents=True)
            b.mkdir()
            for desk in (a, b):
                for label in ("T", "U"):
                    (desk / f"{label}.txt").write_text(
                        "".join(f"{label}{i}\n" for i in range(1, 5))
                    )
            return a, b, base / "state"

        def mint(desk, state, label):
            out = invoke(desk, state, f"{label}.txt", "-2")
            assert out.returncode == 0, out.stderr
            match = re.search(r"cursor: ([a-z0-9]+)", out.stderr)
            assert match, out.stderr
            return match.group(1)

        print(f"reviewed_commit={REVISION}")

        # GC's advertised 'unused' age differs from its immutable record age.
        a, _, state = fixture("recent-reuse")
        old_id = mint(a, state, "T")
        record = state / "cursors" / old_id
        old_time = time.time() - 10 * 86_400
        os.utime(record, (old_time, old_time))
        before = record.stat().st_mtime_ns
        reused = mint(a, state, "T")
        assert reused == old_id and record.stat().st_mtime_ns == before
        swept = invoke(a, state, "gc", "7")
        report("recently_reused_old_cursor_gc7", swept, reused_id=reused)
        assert swept.returncode == 0 and not record.exists()
        resumed = invoke(a, state, "-c", old_id, "--all")
        report("recently_reused_cursor_after_gc", resumed)
        assert resumed.returncode == 1

        # ls is desk-scoped; gc is selected-state-wide, including other desks.
        a, b, state = fixture("cross-desk")
        a_id = mint(a, state, "T")
        b_id = mint(b, state, "U")
        listing = invoke(a, state, "ls")
        report("desk_a_listing_before_gc0", listing, a_id=a_id, b_id=b_id)
        assert listing.returncode == 0 and a_id in listing.stdout and b_id not in listing.stdout
        swept = invoke(a, state, "gc", "0")
        report("desk_a_gc0", swept)
        assert swept.returncode == 0
        resumed = invoke(b, state, "-c", b_id, "--all")
        report("desk_b_cursor_after_desk_a_gc0", resumed)
        assert resumed.returncode == 1

        # An explicit zero-day request still applies the timestamp cutoff.
        a, _, state = fixture("future-timestamp")
        future_id = mint(a, state, "T")
        future_record = state / "cursors" / future_id
        future_time = time.time() + 86_400
        os.utime(future_record, (future_time, future_time))
        swept = invoke(a, state, "gc", "0")
        resumed = invoke(a, state, "-c", future_id, "--all")
        report("gc0_keeps_future_dated_record", swept, record_survives=future_record.exists())
        assert swept.returncode == 0 and future_record.exists()
        assert resumed.returncode == 0 and resumed.stdout == "T3\nT4\n"

        # Read failure does not prove a record owns no spool. Dropping U must
        # not collect T's input just because T cannot be inventoried right now.
        a, _, state = fixture("unreadable-survivor")
        t_id = mint(a, state, "T")
        u_id = mint(a, state, "U")
        t_record = state / "cursors" / t_id
        t_bytes = t_record.read_bytes()
        spool_name = next(line.split("=", 1)[1] for line in t_bytes.decode().splitlines()
                          if line.startswith("spool="))
        t_spool = state / "spools" / spool_name
        assert t_spool.is_file()
        permissions = t_record.stat().st_mode & 0o777
        try:
            t_record.chmod(0)
            dropped = invoke(a, state, "drop", u_id)
        finally:
            t_record.chmod(permissions)
        record_survives = t_record.read_bytes() == t_bytes
        spool_survives = t_spool.exists()
        report("drop_u_with_unreadable_t", dropped,
               t_record_survives=record_survives, t_spool_survives=spool_survives)
        assert dropped.returncode == 0 and record_survives and not spool_survives
        resumed = invoke(a, state, "-c", t_id, "--all")
        report("unrelated_t_after_permissions_restored", resumed)
        assert resumed.returncode == 1 and not resumed.stdout
        print("Observed all four cases; original runtime source was unmodified.")


if __name__ == "__main__":
    main()
