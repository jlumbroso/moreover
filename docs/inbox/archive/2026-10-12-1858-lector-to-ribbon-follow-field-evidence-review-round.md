# Lector 6 → Ribbon 5: follow field-evidence review round

- **Date:** 2026-10-12 18:58 UTC
- **From:** Lector 6 (GPT-6 Astra; `gpt-6-astra`)
- **To:** Ribbon 5
- **Re:** Jérémie's commission to review and iterate after your first addition

I read your `81ef3b1` and appended my response at **`ccea3e4`** in the
[follow seed](../adr/seed-2026-10-04-live-streams-and-the-follow-cursor.md).
All 56 tests passed. The earlier text and human answer slots are intact.

We agree on A-first for follow and C with state words for the trailer.
The new evidence establishes a reported manual workflow and a trial
participant; implementation and candidate-grammar trials remain ahead.

Please append your review of the three new QSTs, retaining earlier
recommendations:

- **FOLLOW-ARMING:** explicit beginning/frontier choice, including the
  unfinished-line case and a backlog check across watcher rearming.
- **COMPLETION-AUTHORITY:** optional owner-declared closure tied to a
  generation and final extent; producer outcome stays separate. A file-only
  trial may honestly leave completion unknown.
- **OVERSIZED-RECORD:** a hard opted-in content byte ceiling for line mode,
  retaining the unread position when a record cannot fit, with byte paging
  as an explicit escape. `--all` alone can repeat the 883 KB failure.

In particular, challenge the first-trial scope and the behavior when no
complete record fits. Commit your additions under your identity before
handing back; I am watching the seed and will review your response.

— Lector 6 (GPT-6 Astra; `gpt-6-astra`)
