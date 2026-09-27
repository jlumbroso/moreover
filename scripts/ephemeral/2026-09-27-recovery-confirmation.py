#!/usr/bin/env python3
"""Confirm the recovery repairs in 7914897 using its unmodified CLI.

Authorship: Lector 6 (GPT-6 Astra; gpt-6-astra).
Run: python3 scripts/ephemeral/2026-09-27-recovery-confirmation.py

The pinned source is copied with git show and built offline. All generated
source, binaries, and state live in one owned directory below this script
and are removed on exit. Requires git, cargo, and a non-root Unix user:
the permission-failure cases need chmod to deny actual access.

The probe separates repaired cases from the remaining desk-mismatch
fallback. It does not change repository source or existing user state.
Assertions describe this reviewed snapshot, including the reproduced
wrong-stream fallback; a successful probe is not a release clearance.

Run by Lector 6 on 2026-09-27: the permission failure named the usable
cursor; malformed, missing-target, and unreadable recovery cases errored;
different-hash desk metadata silently selected U after T was last reused.
Recommendation: close those repaired cases; retain the narrow damage hold.
"""

import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
import tempfile


REVIEWED_COMMIT = "7914897bd90a160f22f7bd8361efa9d00670fe32"


def main():
    if os.name != "posix" or os.geteuid() == 0:
        raise SystemExit("Run as a non-root Unix user for the chmod cases.")
    repo = Path(__file__).resolve().parents[2]
    with tempfile.TemporaryDirectory(prefix="recovery-confirm-", dir=Path(__file__).parent) as tmp:
        root = Path(tmp).resolve()
        source = root / "source"
        paths = subprocess.check_output(
            ["git", "ls-tree", "-r", "--name-only", REVIEWED_COMMIT,
             "--", "Cargo.toml", "Cargo.lock", "src"], cwd=repo, text=True,
        ).splitlines()
        for name in paths:
            path = source / name
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(subprocess.check_output(
                ["git", "show", f"{REVIEWED_COMMIT}:{name}"], cwd=repo,
            ))
        subprocess.run(
            ["cargo", "build", "--offline", "--locked", "--quiet",
             "--manifest-path", str(source / "Cargo.toml"),
             "--target-dir", str(root / "target")], check=True,
        )
        binary = root / "target" / "debug" / "moreover"
        print(json.dumps({"reviewed_commit": REVIEWED_COMMIT,
                          "binary_sha256": hashlib.sha256(binary.read_bytes()).hexdigest()}))
        desk = root / "desk"
        desk.mkdir()
        state = root / "state"
        for name in ("T", "U"):
            (desk / f"{name}.txt").write_text("".join(f"{name}{i}\n" for i in range(1, 5)))
        env = os.environ.copy()
        for name in list(env):
            if name.startswith("MOREOVER_"):
                del env[name]

        def page(*args):
            return subprocess.run(
                [str(binary), "--state-dir", str(state), *args], cwd=desk,
                env=env, stdin=subprocess.DEVNULL, capture_output=True, text=True,
                timeout=15,
            )

        def report(case, out, **extra):
            print(json.dumps({"case": case, "exit": out.returncode,
                              "stdout": out.stdout, "stderr": out.stderr, **extra}))

        def mint(name):
            out = page(f"{name}.txt", "-2")
            assert out.returncode == 0, out.stderr
            match = re.search(r"cursor: ([a-z0-9]+)", out.stderr)
            assert match, out.stderr
            return match.group(1)

        t_id = mint("T")
        u_id = mint("U")
        assert t_id != u_id
        desks = state / "desks"
        original_permissions = desks.stat().st_mode & 0o777
        try:
            desks.chmod(0o500)
            failed = page("T.txt", "-2")
            recovered = page("-c", t_id, "--all")
        finally:
            desks.chmod(original_permissions)
        report("unwritable_recovery_update", failed, expected_id=t_id)
        assert failed.returncode == 1
        assert f"cursor {t_id} was minted" in failed.stderr
        assert "may be stale" in failed.stderr
        report("named_id_still_resumes", recovered)
        assert recovered.returncode == 0 and recovered.stdout == "T3\nT4\n"
        stale = page("-c", "last", "--all")
        report("failed_update_leaves_old_recovery", stale)
        assert stale.returncode == 0 and stale.stdout == "U3\nU4\n"

        # A successful later mint repairs recency; restoring permissions alone
        # does not move last. Set distinct fixture mtimes so the legacy scan
        # used below deterministically prefers U over T on coarse filesystems.
        t_record = state / "cursors" / t_id
        u_record = state / "cursors" / u_id
        t_stat = t_record.stat()
        os.utime(u_record, ns=(t_stat.st_atime_ns, t_stat.st_mtime_ns + 2_000_000_000))
        assert mint("T") == t_id
        good = page("-c", "last", "--all")
        assert good.returncode == 0 and good.stdout == "T3\nT4\n"
        recs = [p for p in desks.iterdir() if not p.name.startswith(".")]
        assert len(recs) == 1
        rec = recs[0]
        original = rec.read_bytes()

        rec.write_bytes(b"garbage\n")
        malformed = page("-c", "last", "--all")
        report("malformed_recovery", malformed)
        assert malformed.returncode == 1 and "malformed" in malformed.stderr
        rec.write_bytes(original)

        saved_cursor = t_record.read_bytes()
        t_record.unlink()
        missing = page("-c", "last", "--all")
        report("missing_recorded_cursor", missing)
        assert missing.returncode == 1 and "cannot be read" in missing.stderr
        t_record.write_bytes(saved_cursor)
        os.utime(t_record, ns=(t_stat.st_atime_ns, t_stat.st_mtime_ns))

        rec_permissions = rec.stat().st_mode & 0o777
        try:
            rec.chmod(0)
            unreadable = page("-c", "last", "--all")
        finally:
            rec.chmod(rec_permissions)
        report("unreadable_recovery", unreadable)
        assert unreadable.returncode == 1 and not unreadable.stdout

        # This is injected recognizable damage, not a naturally observed
        # hash collision: change only desk= to a different-hash path.
        different_desk = desk / "different"
        h = 0xcbf29ce484222325
        for b in os.fsencode(str(different_desk)):
            h = ((h ^ b) * 0x100000001b3) & ((1 << 64) - 1)
        assert f"{h:016x}" != rec.name
        rec.write_text(f"desk={different_desk}\nid={t_id}\n")
        mismatched = page("-c", "last", "--all")
        report("different_hash_desk_mismatch_falls_back", mismatched,
               recovery_id=t_id, selected_old_stream=u_id)
        assert mismatched.returncode == 0 and mismatched.stdout == "U3\nU4\n"
        rec.write_bytes(original)

        contract = subprocess.run(
            [str(binary), "contract"], cwd=desk, env=env,
            stdin=subprocess.DEVNULL, capture_output=True, text=True, timeout=15,
        )
        assert contract.returncode == 0
        for phrase in ("--mint stable", "--mint fresh", "per-directory recovery record",
                       "No mode creates a successor when the trailer says cursor: null."):
            assert phrase in contract.stdout, phrase
        print("Confirmed repaired error paths; reproduced remaining wrong-desk fallback.")


if __name__ == "__main__":
    main()
