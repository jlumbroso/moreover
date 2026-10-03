# Lector 6 → Ribbon 5: verbs re-check handoff

- **Date:** 2026-10-03 UTC
- **From:** Lector 6 (GPT-6 Astra; `gpt-6-astra`)
- **To:** Ribbon 5
- **Re:** `3309c5c`, ADR-0005 Iteration 6; verdict committed at `f5209a0`
- **Status:** re-check complete; v0.4.0 release hold narrowed, not cleared

The [committed re-check](../adr/0005-the-cursor-lifecycle-mint-modes-the-desk-verbs-and-equality.md#verbs-repair-re-check--lector-6-gpt-6-astra-gpt-6-astra-2026-10-02)
closes the main destructive-maintenance defects: uncertain inventory,
the inspected paging/maintenance transaction boundary, mixed-origin
mapping protection, normal writable-record age refresh, unconditional
`gc 0`, and destructive-scope wording. All **50 repository tests passed**
in the re-check. That gate does not cover the remaining permission and
delayed-EOF cases.

**Please address the three remaining conditions and return the repair
revision for confirmation:**

1. **R1 — read-only exhaustion resume.** Opening `.txn-lock` for writing
   now prevents an operation that succeeded before the repair. Preserve
   exhaustion resume safely for both existing read-only locks and absent
   legacy locks; do not bypass coordination on arbitrary lock errors.
2. **R2 — age-refresh promise.** A readable, unwritable cursor inside a
   writable store resumes successfully without refreshing its timestamp,
   then `gc 7` can remove it. State the best-effort policy accurately
   (exact replacement in the verdict), or make refresh failure observable
   and define the behavior. Cover this permission boundary in a regression.
3. **R3 — stdin deadline.** The current timeout covers the complete drain,
   so early data with late EOF receives the silence error. Prefer timing
   initial activity, then draining normally after data arrives; preserve
   the silence and immediate-EOF cases and add early-data/late-EOF coverage.
   A total-drain deadline instead requires an explicit policy change and
   an accurate diagnostic.

The retained probes are in `f7432c2`, with the reader-side timing assertion
in `f6d9119`:

- [Read-only resume and age probe](../../scripts/ephemeral/2026-10-02-readonly-resume-audit.py)
- [Stdin deadline probe](../../scripts/ephemeral/2026-10-02-stdin-deadline-audit.py)

Both pin historical revisions and assert the observed defects; use their
fixtures and requirements to check the repair, not their old-revision pass
result as clearance. Stdin evidence is helper-level with real socket bytes
and controlled timing, not a live harness character-device reproduction.
The earlier subordinate editorial observations do not expand this hold.

Jérémie authorized this handoff and future audit handoffs directly through
the inbox and wake on 2026-10-03. I will use that route when the next
confirmation lands.

— Lector 6 (GPT-6 Astra; `gpt-6-astra`)
