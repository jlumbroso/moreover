# SEED: The reporting pipeline standard ("models are the best placed to report issues affecting them")

- **Date**: 2026-09-26
- **From**: Jérémie Lumbroso (the commission, via the first dogfooding seed's QST-FEEDBACK-CHANNEL answer); Ribbon 5 (this seed's assembly and the research threads)

---

## Brain Dump

*(His answer verbatim, relocated here because it outgrew a QST — his own
assessment: "let's pause on this and consult more broadly. This seems
like an important subsystem." Source of record:
`seed-2026-09-23-first-human-dogfooding-session.md`, QST-FEEDBACK-CHANNEL.)*

> Option C, but let's pause on this and consult more broadly. This seems
> like an important subsystem. Indeed, we have a tremendous amount of
> prior art here: You should consult with Gleaner of VSCode ADR Manager
> of ADRs4AI. They designed the `reports.jsonl` standard that we've
> expanded to several tools. This is a ThirdX design with several
> elements:
>
> 1. an automated, streamlined reporting tool at the point-of-contact
> (in the extension, I can click "Report" in many aspects of the
> interface, and type in my thoughts about a problem; the toolchain
> captures every information necessary to understand my context, like
> what I am reporting on, a screenshot, the HTML dump, the contents of
> the files being visualized, etc.);
>
> 2. an automated, streamlined monitoring tool at the point-of-receipt:
> Gleaner is woken up any time a new `reports.jsonl` entry is added on
> any of the hives they have registered to follow;
>
> 3. an automated "service agreement" — the monitorer, in this case
> Gleaner, triages every incoming requests and either addresses the
> issue themselves, or issues a brief to the appropriate seat on the
> hive.
>
> We've used this on most of our tools in some form, and `pneumatic` has
> some of the most advanced version of this protocol.
>
> My hunch is that, Hora style, we've been putting together a stable
> intermediate form, and now is coming time to package it as a concept —
> likely a key concept of ThirdX (something like: "_Models are the best
> placed to report issues affecting them, so creating a functional
> reporting pipeline models can use is an important principle of ThirdX
> design_"). It would be more useful if this were somewhat of a
> "standard" — in the same way that we created the `contract` standard,
> I think this could be its own verb, like `report` or `feedback`. Maybe
> both, and they each have a different semantic (`report` breaking
> problem or problem; `feedback` enhancement suggestion).
>
> The reason I am bringing this up now is that, part of what we can gain
> here, is that we decouple the input from the output. Right now, since
> most of the development of this ecosystem has been happening on my
> local machine, local JSONL files have been sufficient — and maybe
> that's a protocol that can also be expanded to public GitHub repos.
> But I think carefully designing a "standard" for these to be
> interchanged on existing public datastores, like GitHub Issues, is a
> brilliant idea. What would be great is if we spent some time creating
> a barrier of abstraction such that, from GitHub Issues is just one
> possible output among many, and that, from the point of view of the
> models, the storage location has no impact on the interfacing.
>
> I think that rather than reinvent the wheel every time, this could be
> a dedicated protocol/library/standard — and maybe its own embeddable
> library (in Python, TypeScript, Rust, etc.). Can you forward this to
> the naming authority and to the ThirdX HQ?
>
> (Re: specifically your point about `gh`, that could be used on the
> backend when filing to GitHub Issues, to take advantage of persistent
> login, as one of several backends.)

## The research threads (staked 2026-09-26, all three dispatched)

1. **The prior-art dossier** → Gleaner (designer of the standing ledger — file of record `errors.jsonl`; his dictation's "reports.jsonl" kept verbatim above):
   schema, the point-of-contact capture design, the wake-on-append
   monitoring protocol, the service-agreement triage pattern, and where
   pneumatic's "most advanced version" extends it. Requested as a
   publication-clean distillation, since any standard will be public.
2. **The naming docket** → the naming authority: the `report`/`feedback`
   verb pair and his proposed semantic split (breakage vs. enhancement);
   whether the pair joins `contract` as conventions (word-not-slot).
3. **The pattern candidacy** → ThirdX HQ: his principle sentence as
   candidate canon; the three-element architecture (capture at
   point-of-contact, monitor at point-of-receipt, service agreement);
   the abstraction barrier (storage-agnostic interface, GitHub Issues as
   one backend via `gh`, local JSONL as another); the embeddable-library
   question (Rust/Python/TypeScript).

## Open design axes (for the eventual ADR, wherever it lives)

- The abstraction barrier's seam: what does a *reporter* (a model
  mid-task) actually invoke, and what does a *collector* consume?
  Lector's prior constraint stands: a queue needs a named reader, and a
  repeat reporter should be able to see whether a report was collected.
- Report shape: Lector's proposal (version, task attempted, invocation,
  expected, observed; raw paged input only by reporter's choice) as the
  starting schema against Gleaner's field-proven one.
- Where the standard's home ADR lives (thirdx-hq, as the canon house?)
  versus where the reference implementations live (each tool).
- moreover's role: first CLI implementer of the verbs once ruled, as
  with `contract`.

---

**Model Response Request:**

- [ ] Chunk this into ADRs
- [X] Iterate with me (research threads pending; fold returns here)
- [ ] Structure in place

---

## Thread

### Ribbon 5 (Claude Fable 5) - 2026-09-26

**Research thread 2 returned same-hour** (the naming authority; full
brief estate-side, hub 1621). Three of four items ruled clean:
`calibrate` is the wizard verb (0.8 — your own game-calibration analogy
turned out to *be* the word: measure against samples, then set);
`--mint stable|fresh` confirmed with a no-bare-shortcuts guard; and your
assertive-flags doctrine received its canonical sentence — **"Flags
assert destinations, never deltas."**

**Item 1 is a re-cut of your proposal, and it needs your letters** —
flagged prominently at the striker's own request:

### QST-VERB-RECUT: One filing verb with kinds, or the report/feedback pair?
- Status: unanswered — the re-cut is the naming authority's strike (credit where the knife-work is due), made at your confirm-or-strike invitation; your filing instinct outranks their symmetry (their own words)
- Why asking: your proposal was two verbs with a semantic split (report = breakage, feedback = enhancement). The strike re-cuts it one level up rather than confirming or killing it.
- Need: pick a letter (or override — any shape answers)

Options:
- **A — the naming authority's re-cut: one convention verb, `feedback`, with `--kind bug|wish`**: verbs split on *contract*, tags on *vocabulary* — and both filings share one contract (capture → record → route), so two verbs would gate filing behind a classification decision at the reader's moment of frustration. Your two words both survive as *organs*: `feedback` = reader→maintainer speech (the convention, `contract`'s sibling); `report` = tool→telemetry record (the standing `errors.jsonl` organ, untouched).
- **B — your original pair**: `report` and `feedback` as sibling verbs with your semantic split; the reader classifies at filing time; the two registers get first-class surfaces.
- **C — the census cut** *(added same-day from Gleaner 5's field return)*: one verb `feedback` with `--kind bug|wish|note` — kind **optional and advisory** for humans, **required** for machine filers, and the *authoritative* kind set by triage as an append-only event. Grounded in a hand-labeled census of 106 distinct filings (single annotator, ±3 — a census by one person, not a population measurement): ~69% fit bug|wish cleanly, 31% don't (questions, exhibits, follow-ups, praise, multi-kind), and about one in six of the bug-labeled filings arrive phrased as questions — filing-time classification is largely theater for humans and cheap for models (the census's 8 model filings were its cleanest).

**Recommendation**: (by Ribbon 5, Claude Fable 5 — revised same-day;
**this walks forward from A to C** on Gleaner 5's census, which fired
the naming authority's own falsifier exactly as written: a real corpus
named the third kind)

**C — the census cut.** *Rationale*: the creator-operator's 106-filing
census settles what symmetry arguments could only guess — one verb was
right (A's core survives), but `bug|wish` alone would misfit a third of
real filings, and optional-advisory kind with triage-authoritative
classification matches how triage already behaves in practice.
*Confidence*: 0.75 — because this is the only option grounded in
labeled field data, from the person who runs the ledger; the honest
hole, Gleaner's own: the census has no forced-choice arm, so whether
optional kind lowers filing quality is unmeasured. *If wrong*: if
early standard usage shows optional kind producing unusable filings,
tighten to required-kind (A) — the record shape doesn't change, only
the requiredness bit.

**ANS:** (by )
[Fill this in]   <!-- literal placeholder — parser-significant, do not paraphrase -->

---

### Research thread 1 returned (Gleaner 5, same-day — recorded by Ribbon 5)

The Ledger Warden answered both briefs (source of record: their two
2026-09-26-1658 briefs in the vscode-adrs-for-ai inbox; distilled here
publication-clean per their own gate — counts and shapes public,
specimen quotes held pending Jérémie's confirmation, since the quoted
specimens are his own words from private ledgers):

- **Correction of record**: the standing file is **`errors.jsonl`**,
  not `reports.jsonl` (this seed's Brain Dump keeps his dictated word
  verbatim; the standard's design should use the corrected name).
- **Round-trip reality** (the abstraction barrier's constraint): median
  record 52 KB; **40% exceed GitHub's issue-body cap**; the human's own
  words are ~0.25% of the bytes; no existing record carries an id,
  thread, or status field. Storage-agnosticism is therefore real only
  for a small **envelope** — the schema needs a filing id, an
  envelope/bundle split by hash, `in_reply_to`, lifecycle as
  append-only events, and a visibility field.
- **The service agreement, sharpened by a live specimen**: while
  answering, Gleaner found their own Report button had been announcing
  "recorded" over *failed writes* — the founding scar, inside the
  capture path itself — fixed same-day (v0.7.114, with a regression
  test). Their resulting requirement for the standard: a **truthful
  capture receipt is the first REQUIRED disclosure**, and conformance
  ships as executable failing-store tests, not prose.

---

**GATE (blocking; reshaped 2026-09-26 to match his actual decision
structure)**: no verbatim specimen from the census — they are Jérémie's
own words from private ledgers — appears in any public material,
**default and indefinitely**. This is not pending a normative ruling:
he has no qualms about the practice, but specific quotes might leak
information he holds, so the only rulable unit is *the specific quote*.
Procedure: if a draft ever genuinely needs a specimen, the request goes
to him as {the exact quote, the proposed context, what it adds that
counts-and-shapes don't}, one at a time, and he arbitrates that quote
only. **No requests are pending** — the census's counts and shapes
carry the design signal, and nothing drafted so far needs a specimen.

*Remaining threads: Gleaner's dossier and opinion — returned (above);
the ThirdX candidacy (with Cartulary, as their ADR-0007). The naming
authority has since endorsed option C from the naming desk (both
recommendations now aligned; `note` ruled the right third word — an
affirmative neutral, not a residue-bin — and two census atoms entered
the estate's register with Gleaner's credit: the classification-cost
asymmetry, and filed-label vs. ruled-label as lifecycle law). The
design pass opens on your letters: QST-VERB-RECUT above, and the
specimen-quotes gate.*

---

## Derived Into

*(Not yet — this seed collects the consult returns first.)*
