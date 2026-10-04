#!/usr/bin/env python3
"""Splice the binary's own printed surfaces into README.md.

`moreover --help` is the human IO and `moreover contract` is the model
IO; the README must SOURCE them, never paraphrase them (Jérémie's rule,
2026-10-04: explanations were outpacing the tool, so the tool's own
output is the only authority). This script regenerates the fenced
regions between the markers:

    <!-- surface:help:begin -->   ... <!-- surface:help:end -->
    <!-- surface:contract:begin --> ... <!-- surface:contract:end -->

Run after any change to USAGE or CONTRACT in src/main.rs:

    python3 scripts/readme-sync.py        (uses `cargo run -q`)
    python3 scripts/readme-sync.py BIN    (uses an explicit binary)

Staleness cannot ship regardless: tests/cli.rs carries
`readme_carries_the_binarys_own_surfaces`, which fails the gate when
these regions differ from the binary's actual output.
"""

import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
README = ROOT / "README.md"


def surface(args: list[str], binary: str | None) -> str:
    cmd = [binary, *args] if binary else ["cargo", "run", "-q", "--", *args]
    out = subprocess.run(cmd, cwd=ROOT, capture_output=True, text=True, check=True)
    return out.stdout.rstrip("\n")


def splice(text: str, name: str, body: str) -> str:
    begin, end = f"<!-- surface:{name}:begin -->", f"<!-- surface:{name}:end -->"
    pattern = re.compile(re.escape(begin) + r".*?" + re.escape(end), re.DOTALL)
    if not pattern.search(text):
        sys.exit(f"README.md is missing the {begin} / {end} markers")
    replacement = f"{begin}\n\n```text\n{body}\n```\n\n{end}"
    return pattern.sub(replacement, text)


def main() -> None:
    binary = sys.argv[1] if len(sys.argv) > 1 else None
    text = README.read_text(encoding="utf-8")
    text = splice(text, "help", surface(["--help"], binary))
    text = splice(text, "contract", surface(["contract"], binary))
    README.write_text(text, encoding="utf-8")
    print("README.md surfaces regenerated from the binary")


if __name__ == "__main__":
    main()
