#!/usr/bin/env python3
"""Bounded CLI audit of the latest rename and deprecated-last ordering.

Authorship: GPT-6 Astra delegated reviewer (contract_audit).
Prepared for review and integration by Lector 6 (GPT-6 Astra).

Run with this file's path from any working directory. Requires git, cargo,
and cached dependencies for an offline build. Extracts Cargo.toml,
Cargo.lock, and src/ from pinned 8c6ac9e into an owned TemporaryDirectory
under scripts/ephemeral. No product source is changed. Build, states,
inputs, and desk directories are removed on exit; no user state is used.

The first cases demonstrate that rejected last creates a previously absent
store and that a file occupying the state path masks the rename signpost.
Positive controls check latest case variants, both cursor flag spellings,
an explicit uppercase printed ID from another desk, and empty-desk refusal.
Assertions pin the reviewed revision's behavior, including its ordering
defect; this is a historical audit entrypoint, not a current product test.
"""

import hashlib
import os
from pathlib import Path
import re
import subprocess
import sys
import tempfile


# Validation, 2026-10-04: invoked by absolute path from /private/tmp.
# Passed after separating fixture directory names numerically for
# case-insensitive filesystems. All three deprecated spellings created
# cursors/ and spools/ before exit 2. An unusable state path produced exit 1
# without the rename signpost. latest/LATEST/Latest, both cursor flags, and
# cross-desk explicit 116K replay passed; desk B still had no recovery entry
# after exhaustion. Owned build/state scratch was removed. Recommendation:
# move deprecated-word rejection ahead of store opening; no recovery or
# explicit-ID regression was found in these bounded checks.
REVIEWED_COMMIT = "8c6ac9ef2efe21585c3bee4f54eeba8e59e85cf6"


def checked(args, cwd, timeout=60):
    return subprocess.run(
        args, cwd=cwd, capture_output=True, text=True, check=True, timeout=timeout
    ).stdout


def exercise(binary, scratch):
    desk_a = scratch / "desk-a"
    desk_b = scratch / "desk-b"
    desk_a.mkdir()
    desk_b.mkdir()
    environment = os.environ.copy()
    for name in ("MOREOVER_TRAILER", "MOREOVER_STATE_DIR"):
        environment.pop(name, None)

    def invoke(state, args, desk=desk_a, payload=b""):
        return subprocess.run(
            [str(binary), "--state-dir", str(state), *args],
            cwd=desk, env=environment, input=payload,
            capture_output=True, timeout=10,
        )

    for index, spelling in enumerate(("last", "LAST", "Last")):
        # Numeric names keep cases separate on case-insensitive filesystems.
        state = scratch / f"absent-state-{index}"
        assert not state.exists()
        output = invoke(state, ["-c", spelling, "--all"])
        signpost = b"use 'latest'" in output.stderr
        entries = sorted(path.name for path in state.iterdir()) if state.exists() else []
        print(f"deprecated={spelling} exit={output.returncode} signpost={signpost} stdout_bytes={len(output.stdout)} created_state_entries={entries}")
        assert output.returncode == 2 and signpost and not output.stdout
        assert entries == ["cursors", "spools"], "the pinned ordering defect must reproduce"

    blocked = scratch / "state-path-is-file"
    blocked.write_bytes(b"fixture sentinel\n")
    output = invoke(blocked, ["--cursor", "LAST", "--all"])
    signpost = b"use 'latest'" in output.stderr
    state_error = b"cannot open state dir" in output.stderr
    print(f"unusable_state: exit={output.returncode} signpost={signpost} state_error={state_error} stdout_bytes={len(output.stdout)}")
    assert output.returncode == 1 and state_error and not signpost and not output.stdout
    assert blocked.read_bytes() == b"fixture sentinel\n"

    state = scratch / "valid-state"
    output = invoke(state, ["-2"], payload=b"T1\nT2\nT3\nT4\n")
    assert output.returncode == 0 and output.stdout == b"T1\nT2\n"
    match = re.search(rb"cursor: ([a-z0-9]+)>", output.stderr)
    assert match and match.group(1) != b"null", output.stderr
    cursor = match.group(1).decode()
    for flag, spelling in (("-c", "latest"), ("-c", "LATEST"), ("--cursor", "Latest")):
        output = invoke(state, [flag, spelling, "--all"])
        assert output.returncode == 0 and output.stdout == b"T3\nT4\n", output.stderr
        print(f"recovery: flag={flag} spelling={spelling} exit=0 expected_remainder=true")

    output = invoke(state, ["--cursor", cursor.upper(), "--all"], desk=desk_b)
    assert output.returncode == 0 and output.stdout == b"T3\nT4\n", output.stderr
    print(f"explicit_id_cross_desk: printed_id={cursor} supplied_id={cursor.upper()} exit=0 expected_remainder=true")
    output = invoke(state, ["-c", "LATEST", "--all"], desk=desk_b)
    assert output.returncode == 1 and not output.stdout
    assert b"no cursors were minted from this directory" in output.stderr
    print("empty_desk_after_explicit_exhaustion: exit=1 recovery_not_created=true")


def main():
    artifact_home = Path(__file__).resolve().parent
    repo = artifact_home.parents[1]
    revision = checked(["git", "rev-parse", f"{REVIEWED_COMMIT}^{{commit}}"], repo).strip()
    if revision != REVIEWED_COMMIT:
        raise RuntimeError(f"unexpected reviewed revision: {revision}")
    paths = checked([
        "git", "ls-tree", "-r", "--name-only", revision, "--",
        "Cargo.toml", "Cargo.lock", "src",
    ], repo).splitlines()
    if "Cargo.toml" not in paths or "src/main.rs" not in paths:
        raise RuntimeError("reviewed snapshot is missing build inputs")
    print(f"reviewed_commit={revision}")
    with tempfile.TemporaryDirectory(prefix=".latest-routing-audit-", dir=artifact_home) as temporary:
        scratch = Path(temporary)
        project = scratch / "source"
        for name in paths:
            contents = checked(["git", "show", f"{revision}:{name}"], repo)
            destination = project / name
            destination.parent.mkdir(parents=True, exist_ok=True)
            destination.write_text(contents)
            if name == "src/main.rs":
                print(f"reviewed_main_sha256={hashlib.sha256(contents.encode()).hexdigest()}")
        target = scratch / "target"
        checked([
            "cargo", "build", "--offline", "--locked", "--quiet",
            "--manifest-path", str(project / "Cargo.toml"),
            "--target-dir", str(target),
        ], repo)
        binary = target / "debug" / "moreover"
        print(f"binary_sha256={hashlib.sha256(binary.read_bytes()).hexdigest()}")
        exercise(binary, scratch)
    print("owned_temporary_artifacts_removed=true")


if __name__ == "__main__":
    try:
        main()
    except subprocess.CalledProcessError as error:
        sys.stderr.write(error.stdout or "")
        sys.stderr.write(error.stderr or "")
        sys.exit(f"audit command failed with exit {error.returncode}: {error.cmd}")
    except (OSError, RuntimeError, subprocess.TimeoutExpired) as error:
        sys.exit(f"audit failed: {error}")
