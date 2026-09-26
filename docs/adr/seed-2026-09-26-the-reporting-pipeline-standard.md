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

1. **The prior-art dossier** → Gleaner (the `reports.jsonl` designer):
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
- **A — the naming authority's re-cut: one convention verb, `feedback`, with `--kind bug|wish`**: verbs split on *contract*, tags on *vocabulary* — and both filings share one contract (capture → record → route), so two verbs would gate filing behind a classification decision at the reader's moment of frustration. Your two words both survive as *organs*: `feedback` = reader→maintainer speech (the convention, `contract`'s sibling); `report` = tool→telemetry record (the standing `reports.jsonl` organ, untouched).
- **B — your original pair**: `report` and `feedback` as sibling verbs with your semantic split; the reader classifies at filing time; the two registers get first-class surfaces.

**Recommendation**: (by Ribbon 5, Claude Fable 5)

**A — the re-cut.** *Rationale*: the moment-of-frustration argument is a
field-tested one in this house (the first model filing arrived via a
destroyed trailer — a reader in exactly the state where a
classification gate loses filings); one verb with kinds also means the
eventual embeddable library exposes one entry point per direction of
speech, which is the abstraction barrier's shape. *Confidence*: 0.7 —
because the striker's own falsifier is honest: this re-cuts *your*
filing instinct, which outranks the symmetry, and a real filing that
fits neither `bug` nor `wish` would name the third kind. *If wrong*:
**B** — and nothing downstream breaks; the contract is shared either way.

**ANS:** (by )
[Fill this in]   <!-- literal placeholder — parser-significant, do not paraphrase -->

---

*Remaining threads: Gleaner's dossier (pending); the ThirdX candidacy
(with Cartulary). The design pass opens when the dossier and your
letters on the re-cut land.*

---

## Derived Into

*(Not yet — this seed collects the consult returns first.)*
