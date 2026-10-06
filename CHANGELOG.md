# Changelog

Each release's section below becomes the GitHub release notes
automatically (cargo-dist reads this file at tag time), so the release
page says what happened, not just how to install. The design record
behind every line lives in `docs/adr/`.

## Unreleased

### Changed
- **Breaking**: the recovery word is now `-c latest` (was `-c last`).
  "last" read as absolute/final temporality — hazardous beside
  `cursor: null`, which really is final — while the feature selects
  *relatively*: this working directory's most recently used cursor,
  re-resolved on each call (the same semantics as docker's `:latest`).
  `-c last` is not silently aliased: it is rejected with a one-line
  pointer to `latest` for one minor version, then removed.
  (ADR-0003, QST-RECENCY-WORD.)

## 0.4.0 — 2026-10-04

The audited lifecycle release: seven iterations of independent audit
(ADR-0005) between first implementation and this ship.

### Added
- **Stable cursor ids** (`--mint stable`, the default): the next-cursor
  id is derived from (saved input, byte position, page number), so
  repeating the same resume reuses the same continuation ID while its
  cursor mapping and saved input remain intact, and a million identical
  calls cost one record. `--mint fresh` asserts the old random-id
  behavior. Flags assert destinations, never deltas.
- **Per-directory recovery hardened** for stable cursor reuse
  (`-c last` itself shipped in 0.3.0; renamed `latest` in the next
  release): another directory's reuse no longer redirects this
  directory's selection. Invocations sharing one directory still share
  its recovery selection.
- **The desk verbs**: `ls` (list this directory's parked cursors;
  `--everywhere` for the whole store), `drop CURSOR` (declare a stream
  finished), `gc [DAYS]` — GC collects by **last recorded use** across
  the selected state directory; mint, reuse, and resume attempt to
  refresh that time, and a failed refresh can leave a recently used
  cursor collectible; `gc 0` sweeps all, unconditionally. Removal
  preserves every surviving stable id's reproducibility via
  placeholder tombstones.
- **Agent-harness null-call guard**: a stdin that is an open device
  delivering neither data nor EOF within 2 seconds (the shape many
  agent harnesses give) now prints a guide and exits 2 instead of
  hanging until killed. (Field report: Gauge 5.)

### Fixed
- Destructive maintenance refuses to run over an unreadable cursor
  record instead of guessing — guessing could free another cursor's
  saved input.
- `gc`/`drop` wait for any in-flight paging in the same store (and
  vice versa), closing a window where a just-saved stream's input
  could be freed before its cursor existed.
- GC collects by last recorded use; mint, reuse, and resume attempt to
  refresh that time, a failed refresh can leave a recently used cursor
  collectible, and `gc 0` removes future-dated records too.
- Resuming to exhaustion works again on read-only state directories.

## 0.3.0 — 2026-09-26

### Added
- `-c last` (now `latest`): per-directory cursor recovery.
- `MOREOVER_TRAILER` environment default for the trailer destination;
  an explicit `--trailer` always wins, even over an invalid value.
- The null-call guide: bare `moreover` on a terminal explains itself
  and exits 2 instead of hanging like `cat`.
- **Bounded-read resume** (ADR-0006): resuming reads what you asked
  for, not the whole saved input — 176 MiB → 1 MiB, 133 ms → 16 ms on
  the benchmark stream.

## 0.2.0 — 2026-09-23

### Added
- First binary release: Homebrew tap (`jlumbroso/tap/moreover`), shell
  installer, and prebuilt binaries for macOS/Linux (ARM64 + x86-64)
  and Windows.
- `moreover contract`: the machine-facing contract — the rules a model
  reader needs, printed by the tool itself.
- The two-world grammar: bare `moreover` + flags is the pipe world;
  subcommands are the desk world; five verbs reserved from day one.

## 0.1.0 — 2026-09-22

### Added
- The founding sketch, working: `producer | moreover -10` saves the
  stream, prints a page, and hands back a cursor;
  `moreover -c CURSOR --all` resumes it from any later shell. The
  trailer grammar (`<moreover: page N, X/Y lines, cursor: C>`) is
  frozen as a compatibility promise. Crate name reserved.
