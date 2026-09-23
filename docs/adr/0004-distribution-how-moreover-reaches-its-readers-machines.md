<!-- adr template version: "adr 3.11.5" -->

# ADR-0004: Distribution — how moreover reaches its readers' machines

- **Date**: 2026-09-22
- **Iteration**: 2
- **Status**: Accepted
- **Deciders**: Jérémie Lumbroso; Ribbon 5

**TL;DR**: Research on getting `moreover` into package channels (his ask:
"how we get something on packages distro like homebrew") — findings below,
three questions with recommendations: climb the standard ladder
(crates.io → prebuilt release binaries → Homebrew tap → core later), use
`cargo-dist` for the machinery, and open the tap under the personal
account without waiting for the repo's long-term home to settle.

---

## Originating Context

**Source**: Jérémie, 2026-09-22, in conversation: *"at some point it'd be
good to research how we get something on packages distro like homebrew."*

The audience matters here: `moreover`'s primary reader is a model, but
the *installer* is usually a human or a harness bootstrap script — and
those machines often have Homebrew or nothing, not a Rust toolchain.
`cargo install moreover` (live since 0.0.1) covers Rust-equipped machines
only; it also compiles from source, which is slow bootstrap for an agent
environment.

**Agency Grant**: research + recommendations; channel choices are his to
accept or override.

---

## DOC: research findings (2026-09-22)

- **The standard ladder** for Rust CLIs, in order of installer
  convenience: (1) crates.io (`cargo install`); (2) GitHub Releases with
  prebuilt per-platform binaries; (3) a Homebrew *tap* for macOS/Linux
  brew users; (4) official channels (homebrew-core, AUR, nixpkgs, apt)
  once adoption justifies maintainer attention. (rust-cli book; jlevy's
  distribution research survey.)
- **`cargo-dist` is alive and current**: v0.33.0 released 2026-09-11
  (axodotdev), having absorbed Astral's fork's features at 0.29. It
  generates the GitHub Actions release workflow, per-platform archives,
  checksums, shell/PowerShell installers, and **a Homebrew formula
  pushed to a tap repo automatically** on each tagged release. Its
  artifact naming also makes `cargo binstall moreover` work for free.
- **Counter-signal, recorded honestly**: a survey of 14 major Rust CLIs
  found 12 running fully custom release workflows; the two on cargo-dist
  (uv, ruff) customize it heavily. Custom buys control; it costs exactly
  the CI-maintenance attention this crew should not spend at espresso
  scale.
- **homebrew-core** (the default `brew install` namespace, no tap add)
  requires stable, checksummed, open-license releases and maintainer
  acceptance; new niche tools conventionally start in a
  **self-owned tap** (`brew install jlumbroso/tap/moreover`), a plain
  GitHub repo named `homebrew-tap` holding a Ruby formula. cargo-dist
  maintains that formula automatically.
- **Repo-transfer safety**: GitHub permanently redirects a transferred
  repository's URLs, including release-asset downloads — so a tap
  formula pinned to `github.com/jlumbroso/moreover/releases/...` keeps
  working if the repo later moves to an organization, until the formula's
  next auto-update rewrites the URLs anyway.

Sources: [Command Line Applications in Rust — Packaging](https://rust-cli.github.io/book/tutorial/packaging.html) ·
[jlevy, Rust CLI binary-distribution research](https://github.com/jlevy/rust-porting-playbook/blob/main/docs/project/research/research-rust-cli-binary-distribution.md) ·
[cargo-dist releases](https://github.com/axodotdev/cargo-dist/releases) ·
[cargo-dist docs](https://axodotdev.github.io/cargo-dist/) ·
[Homebrew: Acceptable Formulae](https://docs.brew.sh/Acceptable-Formulae) ·
[Ivan Carvalho, Homebrew + one-line installers for a Rust CLI](https://ivaniscoding.github.io/posts/rustpackaging2/)

## DOC: the release checklist (from Jérémie's cargo-install reference, 2026-09-22)

He supplied a cargo packaging/publishing reference in conversation; the
part that becomes house procedure is the **validation sequence before
every publish** — adopted as-is because tonight already demonstrated its
value (the crate's `include` globs were gitignore-style and unanchored;
`LICENSE` matched a skill directory's license file; `cargo package
--list` is precisely the check that catches that class before upload):

```bash
cargo test               # the gate, as always
cargo package --list     # exactly what ships — read it, every time
cargo publish --dry-run  # packaged version must compile on its own
cargo publish            # versions are immutable; bump, never reuse
```

Standing facts confirmed by the reference, recorded so nobody re-derives
them: the binary name follows the package name (`moreover` = `moreover`,
no `[[bin]]` needed); `cargo install --git <repo>` and `--path .` work
without crates.io (the README's current install line uses `--path .`
honestly, until 0.1.0 lands on the registry).

## Questions

### QST-DIST-LADDER: Which channels, in which order?
- Status: answered
- Why asking: each rung costs setup and ongoing surface; the audience (agent harnesses bootstrapping a machine) shapes which rungs matter.
- Need: pick a letter (or override — any shape answers)

Options:
- **A — the full ladder, in order**: crates.io (done) → tagged GitHub Releases with prebuilt binaries (macOS arm64/x86_64, Linux x86_64/arm64, Windows if free) → Homebrew tap → homebrew-core / distro channels only when adoption knocks. Each rung feeds the next; binaries alone already give harnesses a `curl | sh` bootstrap.
- **B — tap-first, source-built**: a formula that builds from the crates.io tarball; no CI to maintain, but every `brew install` compiles Rust — slow, and useless to non-brew harnesses.
- **C — hold at crates.io**: `cargo install` is enough until real installer demand shows up.

**Recommendation**: (by Ribbon 5, Claude Fable 5)

**A — the full ladder, in order.** *Rationale*: the installer for a
model-first tool is typically a bootstrap script on a machine without a
Rust toolchain — prebuilt binaries are the rung that serves *this* tool's
actual deployment shape, and the tap is nearly free once binaries exist
(cargo-dist maintains the formula). B inverts the cost (every user pays
compile time to save us one CI file); C waits on demand that
distribution itself creates. *Confidence*: 0.8 — because the ladder is
the surveyed consensus and the binary rung is what our own harness
experience says matters. *If wrong*: if his launch plan wants zero
release infrastructure before the announcement, C now → A at launch, no
work lost.

**ANS:** (by Jérémie Lumbroso)
Yes! Definitely Option A — and I am looking forward to learning how to release something on Homebrew. For the GitHub Release, let's make sure to use continuous integration, and -a workflow that includes `$GITHUB_STEP_SUMMARY` for a rich log creation- (ignore for now, since I agree with your recommendation in the next QST to use a canned workflow).

---

### QST-DIST-TOOLING: cargo-dist, or a custom release workflow?
- Status: answered
- Why asking: the machinery choice sets the crew's permanent CI-maintenance budget.
- Need: pick a letter (or override — any shape answers)

Options:
- **A — cargo-dist**: `dist init` generates the workflow; tagged pushes produce binaries, checksums, installers, and the tap formula. Active (0.33.0, 2026-09). One config in Cargo.toml; the crew maintains intent, not YAML.
- **B — custom GitHub Actions**: what 12 of 14 surveyed major tools do; full control over matrix, signing, artifact shape — at the cost of owning the YAML forever.

**Recommendation**: (by Ribbon 5, Claude Fable 5)

**A — cargo-dist.** *Rationale*: the survey's custom-workflow majority
are tools with needs we don't have (Python wheels, code signing, vendored
assets); at espresso scale the YAML we don't own is attention returned to
the tool; and A is reversible — dist's generated workflow can be forked
into custom later, while a custom workflow never gets simpler on its own.
*Confidence*: 0.75 — because I've verified activity and feature set from
the record, not yet run it against this repo. *If wrong*: if `dist
init`'s output fights this repo's shape (the workspace is unusual: a
public tool inside a private-hub submodule chain), that is **B** with the
narrowest possible matrix.

**ANS:** (by Jérémie Lumbroso)
Brilliant rationale! And I learned the expression "at espresso scale"! Yes, I agree with Option A. I'll go strike out the part about a custom workflow in the previous question. Thanks for the thoughtfulness here. I really agree that "the YANL we don't own is attention returned to the tool"!

---

### QST-TAP-HOME: Where does the tap live, given the repo's own home may move?
- Status: answered
- Why asking: a Homebrew tap is a public repo with a lasting install-command name (`brew install <owner>/tap/moreover`); the tool's repository may itself move to an organization someday, and the install command is user-facing vocabulary that shouldn't churn.
- Need: pick a letter (or override — any shape answers)

Options:
- **A — personal tap now** (`jlumbroso/homebrew-tap`): exists the day binaries do; GitHub's transfer redirects keep formula URLs working if the tool's repo later moves; if the *tap* itself ever moves, brew users re-tap once — a documented, ordinary migration.
- **B — wait for the repo's long-term home to settle** before opening any tap: no migration ever, at the cost of no brew channel meanwhile.

**Recommendation**: (by Ribbon 5, Claude Fable 5)

**A — personal tap now.** *Rationale*: the redirect behavior removes the
technical risk, leaving only the vocabulary cost of a possible future
re-tap — small, ordinary, and payable later — while B pays a certain
cost (no brew install) to avoid an uncertain one; a personal tap also
serves every future personal tool, not just this one. *Confidence*: 0.7
— because the placement question it hedges against is open elsewhere and
is his, not mine. *If wrong*: if that placement settles soon and the
install vocabulary should be born under its final name, **B** — and the
ladder's binary rung still ships meanwhile, unaffected.

**ANS:** (by Jérémie Lumbroso)
Option A sounds good for now. In parallel, I would like to explore and prepare the groundwork for [inclusion into Homebrew Core](https://github.com/Homebrew/homebrew-core/blob/main/CONTRIBUTING.md), if `moreover` proves to be useful. Even if we don't publish there immediately, it would help us to adopt their checklist and audit.

---

## Action Items

- [x] Answer the three QSTs - Owner: Jérémie — all three A, 2026-09-23 (`57481c1`)
- [x] Version 0.1.0: bump, tag, publish to crates.io - Owner: Ribbon 5 — done 2026-09-22 (`3059693`, tag `v0.1.0`); checklist run in full, package list verified clean
- [ ] `dist init` + release workflow committed; verified with `dist plan` - Owner: Ribbon 5 (in flight)
- [ ] Tap prerequisites, the two steps only he can do: create `jlumbroso/homebrew-tap` (empty public repo) and add a `HOMEBREW_TAP_TOKEN` secret (a fine-grained PAT with write access to the tap) to `jlumbroso/moreover`'s Actions secrets - Owner: Jérémie
- [ ] First tagged release with binaries — AFTER the tap prerequisites, so the homebrew publish job has somewhere to push - Owner: Ribbon 5
- [ ] Homebrew-core groundwork (his rider): adopt `brew audit`/core checklist against our own formula early; report readiness gaps in an iteration here - Owner: Ribbon 5, post-first-release
- [ ] `$GITHUB_STEP_SUMMARY` rich release logs (his idea, self-deferred while on canned workflow) — revisit if we ever fork dist's workflow - Owner: parked

## Iterations

### Iteration 1 (2026-09-22)
- Trigger: his distribution ask, morning after the option surface settled.
- Contributors: Jérémie (ask); Ribbon 5 (research, recommendations).
- Outcome: `— → Draft`

### Iteration 2 (2026-09-23)
- Trigger: all three QSTs answered A (`57481c1`) — the ladder, cargo-dist, personal tap — with two riders: homebrew-core audit groundwork early; STEP_SUMMARY parked while the workflow is canned.
- Contributors: Jérémie (answers, riders); Ribbon 5 (processing, implementation).
- Changes: action items rewritten into the implementation sequence; the two human-only steps isolated (tap repo, token secret) so his surface stays two clicks.
- Outcome: `Draft → Accepted`; dist wiring in flight.

---

## Links

- Related ADRs: ADR-0003 (the surface being distributed); ADR-0001 (crates.io reservation, closed)
