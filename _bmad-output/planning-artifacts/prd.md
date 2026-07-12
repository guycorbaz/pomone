---
stepsCompleted: ['step-01-init', 'step-02-discovery', 'step-02b-vision', 'step-02c-executive-summary', 'step-03-success', 'step-04-journeys', 'step-05-domain', 'step-06-innovation', 'step-07-project-type', 'step-08-scoping', 'step-09-functional', 'step-10-nonfunctional', 'step-11-polish', 'step-12-complete']
inputDocuments:
  - docs/analyse/qrop-vs-pomone.md
  - _bmad-output/project-context.md
  - docs/roadmap.md
  - docs/architecture.md
classification:
  projectType: native-desktop-application
  domain: agricultural-crop-planning-market-gardening
  complexity: medium-high
  projectContext: brownfield
workflowType: 'prd'
releaseMode: phased
---

# Product Requirements Document - pomone

**Author:** Guy
**Date:** 2026-07-08 – 2026-07-11

## Executive Summary

Pomone is a native desktop crop-planning application for market gardeners — the
maintained, free, open-source successor to QRop, an established tool whose author
stopped development (poorly tested, undocumented, closed to contributions) and
pivoted to a paid online product, orphaning its users. Pomone answers the market
gardener's daily question — *what do I do, what do I order, and when?* — and its
seasonal one — *do I have enough beds for what I want to grow?*

The product is organized around a two-phase planning workflow that mirrors real
practice: first the grower **plans the crops** and their cycles (sow→grow→harvest,
nursery-sow→transplant→grow→harvest, or plant→grow→harvest), building the list of
crops to produce; then **places** them into parcels, sectors and beds, where a
soil-occupancy curve acts as a live capacity check — tracking open-field and
scarce sheltered beds separately. Because the grower works from paper in the
field, the primary deliverable is the **printed field sheet** actually carried in
the pocket: a multi-day journal sheet (dated columns, note zones), a
planned/in-progress crop list, a bed-occupancy map, and the phytosanitary
treatment register. Work is recorded back into Pomone later, in batch, on the
grower's own rhythm — typically one or two PC sessions a week, on the attention
left over after the farm's real admin.

Target users: Guy's own market-garden farm first (dogfooding — real field use is
the proof and the advertising), then other market gardeners, including those left
without a home by QRop's shutdown. Primary job-to-be-done: reliably produce the
field sheet the grower trusts and follows; everything else — planning, ordering,
capacity — exists to feed it.

### What Makes This Special

- **Annual *and* perennial/agroforestry crops on standardized beds.** QRop is
  annual-only by design; Pomone models pluriannual crops, orchards and
  agroforestry from the ground up (`Lifespan`, `VarietyProfile`, `PruningSeason`).
  This is a market an annual-only tool structurally cannot serve — the grower who
  today plans perennials in a notebook and spreadsheets. The name *Pomone* (Roman
  goddess of fruit trees and orchards) seals this focus: a planner that honors the
  tree as much as the row.
- **Two-phase planning with live, cover-aware capacity feedback.** Separating
  "plan the crops" from "place them" makes the scarce-bed constraint felt at the
  moment of placement, with open-field and sheltered beds counted apart.
- **Paper-first field workflow.** Self-contained, re-printable, dated field
  documents and deferred weekly batch reconciliation — the software fits the
  farmer's week, not the reverse.
- **Sustainability as a differentiator, not hygiene.** Open, tested (≥80% coverage
  gate), documented, contribution-friendly, and free — deliberately built against
  the single-maintainer, closed, untested failure mode that killed QRop. Here,
  engineering quality is the moat: it is the direct antidote to why the
  predecessor died.

Core insight: the true unit of planning is the crop cycle — the *itinéraire
technique* — carrying activities anchored before and after establishment (e.g.
bed preparation ten days before planting); the binding constraint is the
time-occupation of scarce beds; and since the grower works from paper, the tool
must produce trustworthy printouts and accept reality entered days later. The
transformation Pomone delivers: from the solitary mental load of planning a season
by memory to the calm of seeing the whole year — beds, shelters, annuals and trees
— at a glance.

## Project Classification

- **Project type:** Native desktop application (Rust + Slint, local data in SQLite
  or MariaDB behind one repository trait).
- **Domain:** Agricultural crop planning / market gardening (medium-to-high
  domain complexity: agronomic date logic, successions, bed-occupancy capacity
  math, annual/perennial duality, dual backend, i18n FR/EN, strict quality gates).
- **Project context:** Brownfield — core CRUD, catalogs, calendar and perennial
  support already shipped (phases 0–9); this PRD scopes the planning convergence
  toward QRop while preserving Pomone's additions (perennials, agroforestry,
  strata, dual backend).

## Success Criteria

### User Success

Pomone succeeds for the grower when it becomes the single tool they run the
season from — replacing the notebook, the spreadsheets, and the abandoned QRop.
Concretely:

- **The printed field sheet is trusted and followed.** The grower carries the
  weekly sheet, works from it, and it matches the ground — including after a
  mid-week reprint (rain, or many activities closed). A wrong printout is a
  season-level failure, so "the paper is always right" is the top user-success
  bar.
- **The capacity check prevents over-commitment.** At placement, the grower sees
  — and feels — whether open-field and sheltered beds suffice for the planned
  crops, before anything goes in the ground.
- **"What to do / what to order / when" is answered without mental arithmetic.**
  The activity list is derived from the crop cycles (itinéraire technique); the
  order list aggregates seeds and plants with buy-by deadlines computed backward
  from sowing/planting dates.
- **The "aha" moments:** (1) placement — "do my beds fit my ambition?"; (2)
  materialization — an abstract crop cycle becomes dated tasks and a printable
  sheet; (3) reprint — last week's reality is reflected without re-entry friction.
- **A newcomer isn't lost.** A market gardener who is not Guy can start from
  seeded botanical families + a loadable demo farm + tooltips + a getting-started
  manual, and reach a first useful plan without a blank-slate wall.

### Business Success (Project / Adoption)

Pomone is free and open-source; "business" success means adoption and, above all,
project longevity — the explicit antidote to QRop's death.

- **Dogfooding proof:** Guy plans and runs at least one full growing season on
  Pomone, on his own farm, without falling back to paper-outside-Pomone or
  spreadsheets. Real field use is the proof and the advertising.
- **Adoption beyond the author:** at least a small number of other market
  gardeners (target: ≥ 3) adopt Pomone for a real season within 12 months of a
  usable R1.
- **Sustainability signals (the anti-QRop metric):** the project stays open,
  documented, and contribution-friendly — at least one external contribution
  (issue triaged to fix, or merged PR) is accepted, proving the closed-solo
  failure mode is not reproduced.

### Technical Success

Engineering quality is a differentiator here, not overhead — it is the moat.

- **Dual backends stay behaviourally identical** (SqliteRepository ≡
  MariaDbRepository), asserted by `cross_backend_tests` for every new entity and
  column.
- **Coverage gate held:** ≥ 80% workspace-wide, raised to ≥ 95% branch coverage
  on the print/document and capacity/autogen modules — the code whose defects
  hide for days in the field.
- **The weekly "paper-loop" acceptance test is green and release-blocking:**
  plan → place → print multi-day journal → simulated field week (done at varied
  dates / skipped with reasons / not touched) → single batch reconciliation with
  a brutal mid-session interruption → resume → re-print → second loop; asserting
  done tasks are never clobbered, skipped tasks never reappear, and no validated
  line is ever lost or duplicated.
- **Zero-warning CI** (`-D warnings`, clippy pedantic), additive-only migrations,
  every user-facing string present in both `fr` and `en`.

### Measurable Outcomes

- One complete season planned and run on Pomone by the author (dogfooding).
- ≥ 3 external market gardeners using Pomone within 12 months of R1.
- ≥ 1 accepted external contribution.
- The four printed documents pass golden-snapshot review in both FR and EN.
- Bed-occupancy capacity math validated by property tests (aggregation
  conservation + boundary `[start, end)`), open-field and sheltered counted apart.
- Zero regressions on the paper-loop acceptance test across releases.

## Product Scope

Three releases plus a vision horizon; the authoritative, journey-refined detail
lives in «Project Scoping & Phased Development» below. In one breath:

- **R1 «Usable in the field» (MVP, indivisible):** plan crop lines (unplaced) →
  place with live cover-aware capacity → weekly batch reconciliation (three
  gestures: done / skipped / pending, interruptible, backdated) → four printed
  PDF documents (multi-day journal sheet, crop list, occupancy map, treatment
  register) + needs list with buy-by deadlines + onboarding without a
  blank-slate wall. Annual **and** perennial from day one.
- **R2 «Piloter»:** phytosanitary consumption ledger (before the June 2027
  inspection), end-of-season review + season rollover, Acorda census summary,
  observation journal + photo inbox, variety review, rotation visibility,
  bed-use optimizer.
- **R3 «Chiffrer»:** economics (yield / price / revenue estimation — never
  invoicing).
- **Vision:** mobile capture client (evidence-gated), local-first optional AI
  advisor, shareable ITK format, field-crop extensions, Windows/macOS.

## User Journeys

### Journey 1 — Guy: winter planning, spring placement (primary — planning, capacity, arbitrage)

**Opening scene.** A January evening. Guy still remembers last April: seed trays parked on the greenhouse floor because no bed was free, leggy seedlings, losses. This winter he plans in Pomone instead of the notebook and two spreadsheets — but not in one sitting: a Sunday evening, a stolen quarter-hour, a resume two weeks later. When he reopens the plan, draft lines are visibly distinct from finished ones; he picks up where he left off.

**Rising action.** Sixty-some crop-plan lines is a real farm's volume, so he duplicates last month's lettuce line and edits the variety and dates rather than filling a blank form each time. Lines carry quantity as *series × bed-geometry* and a staggering interval; each drags its itinéraire technique (bed prep J-10, fleece J+1). Crucially, the lines are **not placed yet** — and the seed & plant needs list, with buy-by deadlines, prints correctly from unplaced lines alone. He orders in January; placement can wait for February.

**Climax.** At placement, the sheltered-bed curve crosses 100% in April — the ghost of last spring, caught on screen. The peak is *explainable*: clicking it lists the series that compose it. Shifting one succession two weeks isn't enough this year. He makes the call the tool can't make for him — the third lettuce succession is cut. The curve settles. It cost him something, and it was his choice, made in January instead of in mud.

**Resolution.** Along the way he'd typed 300 plugs instead of 30 and noticed after placement; the tool offered *correct the entry* — distinct from *replan* — and the plan history stayed clean. The season is decided, ordered, and placeable. The mental load lives in the software; the calm stays with Guy.

**What can go wrong.** Interrupted mid-line (called to the greenhouse): the draft survives. A capacity peak that can't explain itself is a dead end — detection without diagnosis fails the journey. Re-entry without duplication sends Guy back to his spreadsheet.

**The journey fails if** a capacity conflict is not detected *and explained* before anything is in the ground, or if entering a realistic plan (≈60 lines) requires more evenings than the old spreadsheet did.

### Journey 2 — Guy: the field week and the weekly return (primary — paper loop, batch reconciliation)

**Opening scene.** Monday, 6 a.m. Guy prints the field sheet — not a daily list but a **multi-day journal**: one dated column per day, tasks in bed order, free-note zones per day and at the foot, and a header line that says what it covers and when Pomone last heard from the field («Week of 6–12 July — last reconciliation: 2 days ago»). He folds it into his pocket. By Wednesday it carries a muddy thumbprint and a pencil line half-dissolved by rain.

**Rising action.** The week does what weeks do. A plowing day for the wheat is one single tick; a day on the wheel hoe across the vegetable beds is fifteen. Rain pushes two sowings. Slugs take 60% of a young lettuce planting. A false-seedbed pass becomes pointless after the downpour — he strikes it through on the paper: *not done, and not to be done*. In the margin he scribbles «cleared walkway 3». Ticking the right dated column *is* the date capture — no memory required later.

**Climax.** Thursday evening the PC boots — for invoicing, because customer orders must not be forgotten. Pomone gets the leftover attention: it opens in seconds, directly on «Since Monday: 14 planned tasks», in the exact order of the sheet. Three gestures per line: **done** (date proposed from the column, one tap), **skipped** (one-tap reason: too late / weather / not needed / other — never a mandatory justification), or **leave pending** (default). The struck-through false seedbed is skipped-weather; it will never darken a future sheet, but November's retrospective will remember it. Each validated line is written immediately — there is no Save button — and when the phone rings at line 9 of 14, he just walks away. Next session resumes at line 10.

**Resolution.** The re-print reflects the world as it is: done tasks stay done, the skipped pass is gone, the delayed sowings sit on their new days, the recurring series carries on. The paper is right — *again*. That, precisely, is the trust the whole product hangs on: the sheet must survive a real week, a real interruption, and a farmer who boots his PC twice a week.

**Epilogue — November.** Season over, Guy looks back: what was planned, what actually happened at which real dates, which passes were skipped and why — «the March false seedbed was rained off three years out of four» is a planning lesson, and it exists only because skipped was never conflated with done. That review — and carrying the plan into next January — is the loop that makes year 2 better than year 1.

**What can go wrong.** A skipped week (harvest rush, holidays) must reconcile as a bounded batch («accept last week's 9 tasks as planned?»), not 40 lines one by one. A wrong *done* or *skipped* is correctable — an explicit data-entry correction, never a silent loss. The printer dies: the same document exists as a PDF. An overloaded week overflows to page two, still legible in black-and-white.

**The journey fails if** catching up a full week of paper notes exceeds ~15 minutes, if any re-print clobbers a done task or resurrects a skipped one, or if a line validated before an interruption is ever lost or duplicated.

### Journey 3 — Guy: the orchard row and the lettuce between (primary — perennial & agroforestry differentiator)

**Opening scene.** The east parcel's apple row was planted in 1996 — long before any software. Guy enters it retroactively: the planting starts *active* with its true establishment date, and the agenda receives only future tasks — no avalanche of thirty years of overdue prunings.

**Rising action.** This spring he extends the system: a new mixed row — young apples, currant understory in the shrub stratum — and lettuce successions *between* the rows. Two time-horizons now share one parcel: annuals that free their ground in October, perennials that hold it to the end of the horizon. The capacity math carries both without lying about either. Yield expectations for the new varieties? Unknown — he leaves them empty; the table fills year by year as reality arrives (0, then 12, then 80 kg).

**Climax.** A February morning: the generated pruning task tells him which currants under which apples — where the old notebook's page for this row is warped from the rain that got it in 2023, the year a forgotten pruning cost him a harvest that never came back.

**Resolution.** A currant bush dies in year 4: *terminated* releases its occupancy instead of haunting the capacity curve; its replacement goes into the same row, two ages coexisting. Annuals and trees live in one plan, on the same printed sheet. Guy closes the notebook and puts it on the shelf.

**The journey fails if** a retro-entered 1996 orchard floods the agenda with decades of phantom tasks, or if a dead perennial keeps blocking capacity to the end of horizon.

### Journey 4 — Marie: the QRop orphan (secondary — onboarding, adoption)

**Opening scene.** Marie ran her Breton micro-farm on QRop for five years — until the update that opened a popup pushing the new online subscription. The forum thread where she vents is the same one that mentions Pomone.

**Rising action.** First reflex: she looks for «Import from QRop». There isn't one — and Pomone says so up front, in the getting-started manual, with the honest reason (a clean start over a lossy migration) and a fast re-entry path designed for exactly her: botanical families pre-seeded, species and varieties deliberately *not* (hers are Breton; seeded defaults would be someone else's farm). The disappointment is real, managed, and short. She loads the demo farm first, pokes at a planned season, then switches to her own farm — a separate, clean database the demo can't contaminate.

**Climax.** Within half an hour she has her core varieties in and prints her first plan skeleton. Then, hesitant — five years of QRop never had a box for them — she types «framboisiers». A place exists. Her raspberry rows move from a margin note into the plan.

**Resolution.** She plans her real season on Pomone: free, local data, GPL — a home nobody can take away again, because this time she *owns* it. The same journey plays in English for the Welsh grower from the same forum thread; every document and label ships in both languages.

**The journey fails if** Marie cannot print a first useful plan within ~30 minutes unaided, or if demo data ever mixes into her real farm.

### Journey 5 — Thomas: the contributor (project-viability journey)

*This journey reveals project-viability requirements (contribution infrastructure), not end-user functional requirements — with one exception: extensibility by data.*

**Opening scene.** Thomas, a Rust developer who gardens on weekends, wants Swiss-German holiday rules in the calendar. He's been here before: years ago he offered QRop a patch and found the door closed. QRop died of that closed door; he knows it, and so does the Pomone README.

**Rising action.** The repository reads like an invitation: layered crates, a project-context file for both humans and AI agents, an 80% coverage gate, and a README that preempts the first stumble (`libfontconfig-dev pkg-config` before the first build — the exact wall he'd have hit). His issue gets triaged with a pointer to the right module; holiday regions turn out to be data plus one rule function, not surgery.

**Climax.** Not the green CI — the human moment: the maintainer's review reply, the merge notification, his name in the contributors list. The door is open, provably.

**Resolution.** Pomone gains a region, and Marie — who will never read a pull request — gains the thing she actually needs: evidence that this home has more than one keeper.

**The journey fails if** a competent outsider can't go from clone to a green test suite by following the README alone, or if issues sit untriaged.

### Cross-cutting design principles (established through journey review)

- **«Pomone never asks what it already knows, and never more than one line at a time.»** Every entry starts from a prefilled proposition (the plan, the history, an online catalog) that the user *confirms or corrects* — never a blank form to fill. Confirm = one gesture; correct = one field.
- **No Save button.** Every validated line is persisted immediately; an interrupted session is a normal session end, and the next session resumes exactly where the last one stopped. This is vital, not cosmetic: Pomone lives on residual attention after the farm's real admin (invoicing) is done.
- **The week is the unit, not the day.** The nominal case is batch reconciliation of several days of paper notes, 1–2 PC sessions per week. Reconciliation opens on «since the last entry», in the paper sheet's order, supports bulk acceptance for long gaps, and costs proportionally to the work actually done (one plow tick costs one tick; fifteen hoe ticks flow tac-tac-tac with visible progress).
- **Two timestamps everywhere: `occurred_at` ≠ `recorded_at`.** Nearly every entry is backdated by design (paper first, PC later). The dated columns of the printed journal capture `occurred_at` at pencil-time; data freshness («last reconciliation: N days ago», shown in-app and printed in the sheet header) is computed from `recorded_at`. Faithful `occurred_at` is the raw material of next season's retrospective — this is a product requirement, not a technical nicety.
- **Task lifecycle: pending / done{date} / skipped{reason}.** A deliberately-not-done task (too late, weather, not needed) is *skipped*: never counted as done, absent from every future list and printed sheet, permanently visible in retrospective with its reason (closed set: too-late / weather / not-needed / other + optional free note — the reason is never mandatory). Skipping applies to one occurrence, never silently to a recurring series. Task auto-generation must never resurrect a skipped task. «Done» is reversible only as an explicit data-entry correction (recommended, to ratify): nothing is ever un-done silently.
- **The weekly paper-loop is the release-blocking acceptance test**, upgraded from the daily version: plan → print multi-day journal → simulated field week (done at varied dates, skipped with reasons, recurring occurrence skipped, margin notes) → single batch session with a brutal mid-session interruption → resume → re-print → second loop. Product invariants, in product language: done stays done; a skipped task never reappears on a future sheet nor counts as done; skipping an occurrence never kills its series; no validated line is ever lost or duplicated across interruptions; freshness reads from recording time, not field time.

### Journey Requirements Summary

| Capability area | Revealed by |
|---|---|
| Crop-plan lines: quantity model, staggering, line duplication, draft/complete state | J1 |
| Two-phase decoupling: needs list from unplaced lines; placement weeks later | J1 |
| ITK activity templates, signed offsets, generation at placement | J1, J2 |
| Capacity curve: covered/open split, explainable peaks, arbitrage support | J1, J3 |
| Correction ≠ replanification, surfaced in UI at the moment of error | J1, J2 |
| Task lifecycle incl. skipped{reason}; autogen never resurrects; occurrence ≠ series | J2 |
| Batch weekly reconciliation: since-last-entry, 3 gestures/line, bulk accept, interruptible, ≤15 min/week | J2 |
| Printed documents: multi-day journal sheet (dated columns, note zones, freshness header), crop list, occupancy map — dated, re-printable, B&W, overflow-safe, PDF fallback | J1, J2 |
| Planting state machine incl. reversible terminate, perennial release on death | J2, J3 |
| Retroactive entry of existing perennials without past-task avalanche | J3 |
| Mixed-horizon occupancy: annuals between perennial rows on one parcel | J3 |
| Optional/unknown yields; yearly-harvest fill-as-you-go | J3 |
| Onboarding: seeded families only, demo farm isolated from real data, honest no-import message, ≤ 30 min to first printed plan | J4 |
| FR/EN parity in UI and all printed documents | J4 |
| Contribution infrastructure: README-to-green-tests path, triage, extensibility by data | J5 |

### Scope decisions and boundaries (from journey review)

**Out of scope — permanent boundary:** customer orders, invoicing, accounting. Pomone plans and pilots production; it will not feed invoices (owner decision). Supplier «orders» in R1 are a **printable needs list** (aggregated seed/plant quantities with buy-by deadlines computed from the plan) — not order management.

**Mobile — decided:** out of R1 and R2. Reassessed after ~12 months of real dogfooding, on the metric «reconciliation sessions skipped or late». If ever built: a narrow **capture** client (immutable events + morning read-only snapshot — never bidirectional sync), plausibly a PWA on the farm LAN against the existing MariaDB backend. Pre-validated exception: a **photo inbox** (field photos taken with the phone's normal camera land in Pomone for evening qualification) is an R2 candidate — the camera is the one field gesture paper cannot replace.

**AI assistance — decided:** the R1-era workflow is *structured export + prompt templates*: Pomone exports the farm's structured history (planned vs actual, yields, dates, skipped-task patterns); the grower sets objectives, adds supplier-catalog links, and discusses next season with the LLM agent of their choice **outside Pomone**; the resulting recommendation is entered back by the human. Pomone is the source of truth and the scribe; the synthesis intelligence lives outside; the decision stays human. Vision (R3+): a return-trip format (draft plan proposal importable as line-by-line confirmable draft), and an optional local-first advisor trait — propose, never write. Shareable ITK format between growers is the long-term community moat, aligned with the activity-templates brick.

**R2 candidates surfaced (to confirm against dogfooding, not promises):** observation journal entity, photo inbox, variety «source/why» annotation, variety season-review view, deterministic bed-use optimizer, variety referencing from online catalogs without re-typing, startup < 3 s (engineering target).

**Open scope questions → Step 8 (Scoping):**
1. Season rollover — carry a plan into year N+1 (J1, J2 epilogue).
2. ~~Quick capture of unplanned work at reconciliation~~ — **resolved during UX design (owner-ratified)**: R1 gets a first-class «+ free line» gesture in reconciliation; structured journal stays R2.
3. Supplier stockout / variety substitution mid-plan (J1).
4. End-of-season review view — planned vs actual (J2 epilogue; the deterministic retrospective is a firm need, its release slot is the question).
5. Rotation visibility at placement — family interval on bed history (J1; QRop parity gap).
6. Ratify: «done» reversible as explicit data-entry correction (recommended yes).

*Already decided, restated for traceability: no QRop data import (clean-start); annual-crop harvest **quantities** are R2 (the R1 field-return gesture records states, not weights — perennial `YearlyHarvest` already ships); the third printed document is the bed-occupancy map, anchored at placement (J1) and in the Monday pocket (J2).*

## Domain-Specific Requirements

### Compliance & Regulatory

**Phytosanitary treatments — workflow and register (legal requirement, CH — controls tightening).**

*The treatment flow is plan → execute → confirm, like any field task:*

1. **Plan**: the grower detects a problem (e.g. grey aphids) and declares a protection treatment: target crop and bed(s), product, dose for that crop type. **Pomone computes the needed product quantity** from dose × treated area — a preparation aid («prepare 240 ml»); for non-surface doses (spray-mix concentrations, per-plant doses) the computed value is a starting point the grower adjusts.
2. **Execute**: the treatment happens in the field (it can appear on the weekly sheet like any planned task).
3. **Confirm done**: the grower marks it done (dated, backdatable like everything else); the **actually used quantity** is confirmed at this moment — prefilled with the computed value, correctable in one gesture. This confirmed quantity, in a closed unit set (g/kg/ml/L), is the source of truth of the register. A planned treatment can also be *skipped* (rain washed the aphids off) — same lifecycle as tasks.

*The bio-inspection report (printable; content legally imposed, format ours):*

- **One line per treatment**: crop, bed, date, product, concentration (dose), quantity of product used.
- **At the end, per product: total quantity used over the year** — sum of confirmed quantities, exact `Decimal` arithmetic; treatments missing a confirmed quantity are explicitly listed as gaps, never silently omitted.
- The grower hands this report to the controller together with a stock/flow inventory **maintained outside Pomone** (farm ERP / InvenTree / paper); the cross-check purchases − consumption = stock is, for now, the controller's own arithmetic. Pomone's contract: its half of the triangle is impeccable, printable, and CSV-exportable. Treatment history is never purged (unlimited retention; migration tests must round-trip decade-old records on both backends).

*Boundary rule (adopted as product law): an event on a crop or a parcel belongs to Pomone; a compliance rule, threshold, or cross-check with purchases does not. Pomone prints facts, not verdicts.*

*Release slots*: printable treatment register = R1 (fourth printed document — weekly sheet, crop list, occupancy map, treatment register); raw CSV export of treatments = R1 (near-zero-cost escape hatch if an inspection lands before R2); planned-treatment flow with quantity confirmation + annual per-product totals = R2, first in line (the regulatory calendar does not negotiate).

*To confirm against a real inspection report at scoping*: fertilization records (candidate: sibling entity of Treatment), organic-seed provenance evidence (organicXseeds derogations — record or just printout?), biodiversity promotion areas (instinct: out of scope, to be stated), soil-intervention journal, pre-harvest waiting periods, controller tolerance on theoretical vs actual quantities.

**Label certifications (Bio Suisse et al.).** Label audits impose record-keeping Pomone must be able to produce: documented rotations (bed history per family), seed/plant provenance, and the treatment register above. Principle: *if the grower must show it to an auditor, Pomone must be able to print it* — with the closed enumeration established at scoping against the certifier's actual checklist.

**Privacy by architecture.** Local-first, no account, no cloud, no third-party personal data — GDPR/LPD compliance is a structural guarantee, stated as such. Testable: a full enter→print cycle completes with the network cable unplugged, zero outbound connections.

### Technical Constraints

- **Agronomic date correctness**: leap years (day 366), cross-year cycles (winter wheat, autumn sowings), `Decimal` arithmetic for areas/yields/doses — property-tested domain rules (`date_calc.rs`), contractual in this PRD.
- **Rural environment**: modest hardware, 100% offline operation for the whole paper loop, PDF fallback for a capricious printer.
- **Data longevity**: perennials live 30 years; SQLite as de-facto archival format, additive-only migrations, integrated backups.

### Domain Patterns

- **The crop cycle structures everything**: direct-sow / nursery+transplant / bought plants; ITK activities anchored on establishment with signed offsets.
- **Occasional weather observations feed next season's planning.** The grower records notable weather facts when they matter (late frost, hailstorm, drought, exceptional rain) — low-frequency, manual, dated entries in the observation journal, not a weather-station integration. They print in the retrospective and explain skipped tasks and yield variances.
- **Anti-pattern (inherited from QRop's corpse)**: business logic in SQL triggers — forbidden; all date/yield computation lives in tested Rust.
- **Seasonal usage rhythm**: planning peak January–February, weekly loop March–October, retrospective November — the product must be *re-learnable* after a two-month absence.

### Risk Mitigations

- **#1 risk (already enforced)**: a wrong printed document → the weekly paper-loop acceptance test is release-blocking.
- **Database loss = losing the farm's memory**: automatic pre-migration backups + manual backup button (shipped); periodic backup reminder to consider.
- **Agronomic liability**: Pomone never prescribes — derived dates and computed quantities are correctable proposals; agronomic responsibility stays with the human. No AI-agronomist.

## Innovation & Novel Patterns

### Detected Innovation Areas

1. **Mixed-horizon occupancy: annuals and perennials in one capacity model.** Existing tools serve either annual market gardening (QRop, SaaS planners) or orchard/vineyard management — never both on the same parcel. Pomone models lettuce successions *between* apple rows: ground that frees in October next to ground held for thirty years, strata layering included, in a single bed-meters capacity curve. This is the structural innovation — a market the annual-only incumbent cannot follow into, sealed by the product's own name.

2. **Paper-first as a deliberate paradigm, not a fallback.** Against the industry's mobile-first reflex, the printed multi-day field sheet is the primary deliverable and the pocket «mobile» of phase 1: dated columns capture `occurred_at` at pencil-time, batch reconciliation on residual attention (1–2 PC sessions/week) is the *nominal* case, and the release-blocking paper-loop test treats a wrong printout as the worst defect class. The software fits the farmer's day; the day is not reorganized around the software.

3. **AI outside the tool.** Against the embedded-copilot trend: Pomone exports impeccable structured farm history; the grower discusses next season with the LLM agent of their choice *outside* the application; the human enters the verdict back. The farm's data never leaves without the grower carrying it. Local-first optional advisor and a confirmable draft-import are vision items — «propose, never write» is the invariant.

4. **Engineering quality as the moat (anti-QRop pattern).** The predecessor died of a precise failure mode: solo, untested, closed, then a paid pivot. Pomone inverts each factor and treats openness + tests + docs as the *product's survival feature* — with a measurable proof: at least one external contribution accepted.

5. **(Vision) Shareable itinéraires techniques.** An ITK is encoded craft; a free project can make it a forkable, community-shared format — a network effect structurally unavailable to subscription SaaS.

### Market Context & Competitive Landscape

QRop: abandoned, annual-only, its orphans are the first adoption pool. SaaS farm planners: subscription, cloud-resident data, mobile-first, annual-focused — the betrayed-user segment distrusts exactly this shape. Orchard software: perennial-only, no market-garden successions. Nobody serves the mixed farm from one local, free tool.

### Validation Approach

- Mixed-horizon model: the agroforestry journey (J3) + property tests on capacity aggregation with both horizons on one parcel.
- Paper-first: the weekly paper-loop acceptance test (release-blocking) + one full dogfooding season with the metric «reconciliation sessions skipped».
- AI-outside: the export exists in R1 at near-zero cost; validation is the author's own winter workflow with real catalogs.
- Moat-by-quality: ≥1 external contribution within 12 months of R1.

### Risk Mitigation

- **Innovation risk #1 — the mixed model complicates the simple case**: market-gardening-only users must never pay complexity for perennials they don't have (progressive disclosure; R1 capacity = bed-meters only, polymorphic occupancy deferred behind a discriminant column).
- **Paper-first misread as backwardness**: the positioning states it as a choice («the paper is the mobile»), and the 12-month mobile checkpoint keeps the door open with evidence.
- **Fallbacks**: if mixed-horizon capacity proves confusing, degrade to separate views per horizon (data model already permits); if paper-first fails dogfooding, the event-ingestion schema was designed from day one to accept a capture client without rework.

## Desktop Application Specific Requirements

### Project-Type Overview

Native desktop application (Rust + Slint, software renderer — no GPU dependency), local data, no server component. The desktop is where the heavy cognition happens (winter planning, weekly reconciliation); the field interface is paper.

### Platform Support

- **Phase 1: Linux only** — CI, packaging (.deb + AppImage), and support are Linux-first (existing decision). Windows (.msi) and macOS (.dmg) are post-v1; the codebase stays portable (no platform-specific APIs outside std/Slint) so the ports are packaging work, not rewrites.
- Modest hardware target: an aging farm laptop must run it comfortably; software rendering already serves this.

### System Integration

- **Documents are PDFs, generated and saved — printing is delegated.** Every printable document (weekly sheet, crop list, occupancy map, treatment register) is generated as a PDF and **saved to a user-visible directory** with a dated filename (`pomone-fiche-semaine-2026-07-13.pdf`). The user prints with any PDF tool; Pomone may offer «open after export» as a convenience, but embeds no print dialog in R1. Rationale (owner-confirmed): electronic archival is itself a requirement — the inspection register must be savable, e-mailable, and re-openable years later; the printer is just one consumer of the PDF. Document generation stays deterministic and headless-testable (golden snapshots FR/EN), immune to printer-driver misery.
- File system: XDG data/config locations (existing); a configurable documents/export directory; backups next to the database (existing); F1 opens the embedded PDF manual (existing).

### Update Strategy

- **No auto-update** (owner-confirmed). Distribution via GitHub releases (.deb/AppImage); optional discreet «new version available» notice (manual or opt-in check, never blocking, never phoning home by default). The version is printed in every document footer — an old version producing documents is diagnosable from paper.

### Offline Capabilities

- **100% offline for everything in the product** (contractual, see Domain section): the full plan→place→print→reconcile cycle runs with no network. The only network-touching acts are user-initiated: an opt-in release check, and the user carrying exports to their own AI agent.

### Implementation Considerations

- Startup < 3 s to the catch-up screen (engineering target from journeys); no Save button — line-level persistence (SQLite WAL fits).
- Single instance: two Pomone instances on one database must not corrupt it — a friendly «already running» message beats a mystery.
- Long-session robustness: the app may stay open for weeks on the farm PC (sleep/wake cycles) — graceful refresh on wake.
- **A decade of data stays fast**: perennials guarantee long-lived databases (10+ seasons of tasks, treatments, harvests); screens stay responsive via bounded queries (pattern exists: bounded agenda history), retrospective paginates by season.
- **Local diagnostics, zero telemetry**: no automatic error reporting, ever; a rotating local log file in the data directory that the user can *voluntarily* attach to a GitHub issue — serving the contributor/support journey.
- **Aging-eyes legibility**: a font-size setting (or large-print mode) as a low-cost accessibility consideration; on-screen legibility targets follow the same B&W-legible discipline as the printed documents.

## Project Scoping & Phased Development

### MVP Strategy & Philosophy

**MVP approach:** problem-solving MVP — «Usable in the field», indivisible: planning without placement has no user value, placement without printing never reaches the field, printing without the weekly return loop lies within two weeks. The MVP is the smallest loop that stays *true*.
**Resource requirements:** solo maintainer + AI coding agents, hoped-for external contributors (the project's structure is itself a recruitment artifact — J5). Scope discipline is the survival constraint: QRop parity on the skeleton, novelty concentrated on perennials/agroforestry.
**Scope arbitrations are provisional by design**: ratified now, re-examined at production time against dogfooding evidence (owner decision).

### MVP Feature Set (R1 — «Usable in the field»)

**Core journeys supported:** J1 (plan + place + arbitrage), J2 (field week + weekly batch reconciliation), J3 (perennial/mixed rows), J4 (onboarding), J5 (contribution-ready repo).

**Must-have capabilities:**
- Crop-plan lines (quantity = series × geometry, staggering, duplication, draft state) generating staggered plantings; ITK activity templates with signed offsets; annual + perennial.
- Placement (parcel → sector → bed) with live capacity curve — bed-meters only, covered/open split, explainable peaks; horizon = season/year.
- Planting state machine (planned→placed→active→terminated, reversible terminate); **task lifecycle pending/done{date}/skipped{reason}** — skipped leaves future lists/prints, stays in retrospective; autogen never resurrects a skipped task; «done» reversible as explicit data-entry correction (ratified).
- **Weekly batch reconciliation**: since-last-entry, 3 gestures/line, bulk accept, interruptible (line-level persistence, no Save button), backdating via sheet columns; ≤15 min for a full week; `occurred_at` + `recorded_at` on all field events (both R1 — the printed freshness line requires `recorded_at`).
- **Four printed documents (PDF-first, saved then printed)**: multi-day journal sheet (dated columns, note zones, freshness header), planned/in-progress crop list, bed-occupancy map, treatment register (legal content). Raw CSV export of treatments (inspection escape hatch). **The document engine must make adding a future compliance register a small, additive exercise** — the regulatory document list will grow (owner evidence: peers already audited on phyto stock).
- **Needs list, not order management**: aggregated seeds/plants quantities with buy-by deadlines, printable — the grower orders by hand. Stockout/substitution needs no dedicated feature (plan-line editing covers it).
- Onboarding: seeded families only, isolated demo farm, honest no-import message, tooltips, getting-started manual, ≤30 min to first printed plan; FR/EN everywhere.
- Field-legibility grammar + «never asks what it knows» entry principle. Unplanned work: note zones on the printed sheet **and a first-class «+ free line» gesture in the reconciliation flow** (raw text + date, ≤10 s, no categorization — structured observation journal stays R2). *(Scope amended 2026-07-11 during UX design, owner-ratified: «nothing lost, nothing silent» applies to unplanned work.)*

### Post-MVP (R2 — «Piloter», ordered)

1. **Phytosanitary consumption ledger** + planned-treatment flow with quantity confirmation — **external milestone: delivered well before the June 2027 bio inspection** (the only hard external date in the project; peers' stock audits make Pomone's consumption half non-negotiable).
2. **End-of-season review** (planned vs actual, deterministic) + structured export for the external AI workflow + **season rollover** (plan N → N+1).
3. **Acorda census-filling summary** (CH-VD): printable/exportable cultures & areas per parcel at a reference date, formatted to transcribe into the census — Pomone prints the facts, the grower fills the form; no Acorda integration (boundary rule). Field enumeration from the service de l'agriculture's documents when available.
4. **Observation journal** (incl. weather entries, unplanned work) + photo inbox.
5. Variety review view + «source/why» annotation + online-catalog referencing without re-typing.
6. Rotation interval + visibility at placement (QRop parity); bed-use optimizer (deterministic).
7. Annual-crop harvest quantities; rich reconciliation; notes UI; improved Gantt; Type→Method→Implement exposure.

### R3 («Chiffrer») & Vision

R3: economics (yield/price/revenue estimation — never invoicing). Vision: mobile **capture** client (gated by the 12-month dogfooding checkpoint on the skipped-reconciliations metric), local-first optional AI advisor + confirmable draft-import, shareable ITK format, field-crop extensions (#29 remainder), Windows/macOS ports.

### Compliance Scope (consolidated)

**Committed (evidence: the only documents requested of the owner to date):** the treatment plan/register and the Acorda census summary.
**Watch list (never requested so far — no commitment, re-examined at every release boundary given the tightening regulatory trajectory):** fertilization balance, organic-seed provenance evidence, biodiversity promotion areas, soil-intervention journal, pre-harvest waiting periods.
**Boundary (product law):** an event on a crop or parcel belongs to Pomone; a compliance rule, threshold, stock, or cross-check with purchases does not. Pomone prints facts, not verdicts.

### Risk Mitigation Strategy

**Technical:** the two genuinely new engines are PDF generation and capacity math — both testable headless (golden snapshots FR/EN; property tests on aggregation + `[start,end)` boundaries); the weekly paper-loop acceptance test (brutal interruption + second loop included) is release-blocking.
**Market:** adoption rests on dogfooding proof + QRop orphans; mitigated by an R1 that is honest (no-import stated, rotation gap acknowledged) and a familiar planning skeleton. Checkpoint metrics: reconciliations skipped (mobile decision), ≥3 external growers within 12 months of R1.
**Resource:** solo-maintainer reality already shaped R1 (needs list instead of order management, bed-meters only, no print dialog, paper carries unplanned notes); if R1 still proves too big, the pre-agreed cut order is: onboarding demo-farm richness → occupancy-map document → never the loop itself.
**External calendar:** June 2027 bio inspection — R2 item 1 must land comfortably before it.

## Functional Requirements

### Crop Planning

- FR1 *(R1)*: The grower can define a crop-plan line — crop/variety, quantity as series × bed-geometry, staggering interval — **without assigning a location**.
- FR2 *(R1)*: The grower can duplicate an existing plan line and edit the copy.
- FR3 *(R1)*: Plan lines carry a draft/complete state; the grower can resume a fragmented planning session where they left off.
- FR4 *(R1)*: A plan line generates its staggered plantings (N successions at the defined interval).
- FR5 *(R1)*: The grower can define reusable itinéraires techniques (activity templates) anchored on establishment with signed day-offsets (before and after).
- FR6 *(R1)*: Tasks are generated from the ITK at placement, including pre-establishment activities.
- FR7 *(R1)*: The grower can produce a needs list — aggregated seed/plant quantities with buy-by deadlines — from plan lines, including unplaced ones.
- FR8 *(R2)*: The grower can carry a season's plan into the following year without re-entry.

### Placement & Capacity

- FR9 *(R1)*: The grower can place plantings into the location hierarchy (parcel → sector → bed).
- FR10 *(R1)*: The system shows a live soil-occupancy curve at placement, counting sheltered and open-field capacity separately.
- FR11 *(R1)*: The grower can see which series compose any capacity peak (explainable conflicts).
- FR12 *(R1)*: Capacity aggregates up the location hierarchy, readable at every level.
- FR13 *(R1)*: Annual and perennial plantings coexist on one parcel, each with its own time horizon (perennials occupy to end of horizon).
- FR14 *(R1)*: The grower can retro-enter pre-existing perennial plantings (historical establishment dates) without generating past tasks.
- FR15 *(R1)*: A terminated perennial releases its occupancy; a replacement can share the row (two ages coexist).
- FR16 *(R2)*: The grower can see a bed's crop history and family rotation interval at placement.
- FR17 *(R2)*: The system can propose bed-use optimizations (deterministic algorithm).

### Field Execution & Reconciliation

- FR18 *(R1)*: Tasks have three states — pending, done{date}, skipped{reason from closed set + optional note, never mandatory}; skipped tasks vanish from future lists and prints but remain in retrospective.
- FR19 *(R1)*: Task auto-generation never resurrects a skipped task; skipping an occurrence never affects its recurring series.
- FR20 *(R1)*: The grower reconciles in batch «since the last entry», in the printed sheet's order, three gestures per line (done / skipped / leave pending).
- FR21 *(R1)*: Every field event carries `occurred_at` (backdatable, proposed from the sheet's dated column) distinct from `recorded_at`.
- FR22 *(R1)*: Long gaps reconcile as a bounded batch (bulk accept), not line-by-line punishment.
- FR23 *(R1)*: A reconciliation session is interruptible at any line — each validated line is persisted immediately, the next session resumes exactly there; no Save button anywhere.
- FR24 *(R1)*: «Done», «skipped» and «terminated» are reversible as explicit data-entry corrections; nothing is ever un-done silently.
- FR25 *(R1)*: Replanning a task/planting and correcting an entry are two distinct actions, distinguishable in the UI at the moment of use.
- FR26 *(R1)*: Planting lifecycle transitions (placed → active → terminated{cause}) each carry an explicitly entered date.
- FR27 *(R2)*: The grower can declare a planned treatment (product, dose, target beds) — the system computes the needed product quantity as a preparation aid — then confirm it done with the actual quantity used (closed unit set, prefilled, correctable).
- FR28 *(shipped, kept)*: The grower can record treatments, recurring tasks, and split/move plantings.

### Documents & Exports

- FR29 *(R1)*: The system generates four documents as PDFs saved to a configurable directory with dated filenames: multi-day journal sheet, planned/in-progress crop list, bed-occupancy map, treatment register.
- FR30 *(R1)*: The journal sheet is a multi-day form: one dated column per day, tasks in bed order, free-note zones, and a header stating coverage period + data freshness («last reconciliation N days ago», from `recorded_at`).
- FR31 *(R1)*: Every document is re-printable at any time and reflects current state: done stays done, skipped never reappears, replanned items sit on their new dates.
- FR32 *(R1)*: All documents exist in FR and EN, are legible in black-and-white, paginate on overflow, and carry the generating version + print date in the footer.
- FR33 *(R1)*: The grower can export raw treatments as CSV (inspection escape hatch).
- FR34 *(R2)*: The system produces the consumption ledger — per product and period, summed confirmed quantities, gaps explicitly listed — printable and CSV-exportable.
- FR35 *(R2)*: The system produces the Acorda census summary — cultures and areas per parcel at a reference date — printable/exportable.
- FR36 *(R2)*: The grower can export the season's structured history (planned vs actual, yields, dates, skip patterns, observations) for use with an external AI agent.

### Harvest & Season Learning

- FR37 *(shipped, kept)*: The grower records yearly harvests per perennial planting (expected/actual/variance), yields optional and fillable as reality arrives.
- FR38 *(R2)*: The grower can record harvest quantities for annual crops.
- FR39 *(R2)*: The grower can review a season — planned vs actual dates, yields, skip patterns and their reasons, weather events — as the deterministic retrospective.
- FR40 *(R2)*: The grower can keep an observation journal: dated, typed entries (incl. weather facts and unplanned work), attachable to a planting/bed, with photos by reference; a photo inbox lets field photos land in Pomone for later qualification.
- FR41 *(R2)*: The grower can review varieties across seasons (results, observations) and annotate each variety with a free «source/why» note at creation.

### Catalogs & Farm Data

- FR42 *(shipped, kept)*: The grower manages crops, varieties, locations (hierarchy with dimensions), strata, families (colors), task types; display units m²/ha and kg/t; SQLite or MariaDB backend with live migration and backups.
- FR43 *(R2)*: The grower can reference a variety from an online supplier catalog without re-typing its data.

### Onboarding & Help

- FR44 *(R1)*: First-run experience: seeded botanical families only, a loadable demo farm strictly isolated from real data, an honest «no QRop import» notice with a fast re-entry path, contextual tooltips, and a getting-started manual (F1) — leading an unaided newcomer to a first printed plan.
- FR45 *(R1)*: The full UI and all documents are available in French and English, switchable at runtime.

### System & Trust

- FR46 *(R1)*: The complete plan→place→print→reconcile cycle works with no network connection.
- FR47 *(R1)*: The application starts into the reconciliation catch-up screen, resuming any interrupted work.
- FR48 *(R2)*: The grower can opt into a discreet «new version available» check; a periodic backup reminder; a rotating local log file supports voluntary bug reports; a font-size/large-print setting serves aging eyes.
- FR49 *(shipped, kept — affected by new task states)*: The grower visualizes work through the existing views — monthly task calendar (drag-to-reschedule, milestones, holidays greyed), agenda, season Gantt, crop map, home occupancy curve. Skipped tasks render struck-through/greyed in past views and vanish from future ones, per FR18.

## Non-Functional Requirements

### Performance

- Startup to the reconciliation catch-up screen in **< 3 s** on modest farm hardware (aging laptop, software rendering).
- A full week of paper notes reconciles in **≤ 15 min**; the cost of reconciliation is proportional to work done (one task = one gesture).
- An unaided newcomer reaches a first printed plan in **≤ 30 min** (J4 criterion).
- Screens stay responsive with **10+ seasons** of history: bounded queries, per-season pagination in retrospective views.
- Document generation completes within a few seconds on target hardware (engineering target — never long enough to discourage a re-print).

### Reliability & Data Integrity

- **No validated line is ever lost or duplicated** across a brutal interruption (kill mid-session, power loss) — line-level persistence, verified by a kill/replay test harness.
- **Done stays done; skipped never resurrects** — re-prints and task regeneration never clobber recorded reality (product invariants I1–I6, property-tested).
- Data survives decades: additive-only migrations; a database with 10-year-old records round-trips every migration intact, on both backends.
- Two application instances on one database cannot corrupt it (friendly refusal).
- Backups: automatic before any backend migration; manual button; periodic reminder (R2).

### Security & Privacy

- **Local-first, zero telemetry**: no outbound network traffic by default, ever; the only network acts are user-initiated (opt-in release check). Verified: the full plan→place→print→reconcile cycle runs with networking disabled.
- No account, no cloud, no third-party personal data — GDPR/LPD by architecture.
- The farm's data leaves the machine only by the user's explicit act (export, backup copy).

### Accessibility & Legibility

- All printed documents legible in **black-and-white**, outdoors, with a dated header and embedded legend — self-contained for a reader who never saw the software.
- On-screen field-legibility grammar: **filled = editable, no-fill = read-only**, consistently across all screens; contextual tooltips on fields and buttons.
- Font-size / large-print setting (R2) for aging eyes.
- **Re-learnable after a two-month absence** (seasonal usage): every screen answers «where am I, what can I do» without manual consultation.
- Entry ergonomics as contract: *Pomone never asks what it already knows, never more than one line at a time, and has no Save button.*
- **Localized formats**: numeric entry accepts the locale's decimal separator («12,5» as well as «12.5» in French locale); dates display in the locale's convention (JJ.MM.AAAA in Suisse romande) on screens and documents, while storage and CSV exports stay ISO-8601 (contract stability).

### Engineering Quality (the survival moat — contractual)

- Workspace test coverage **≥ 80 %**, raised to **≥ 95 % branch coverage** on document-generation and capacity/autogen modules.
- The **weekly paper-loop acceptance test** (brutal interruption + second loop included) is release-blocking.
- Dual-backend behavioral parity (SQLite ≡ MariaDB) asserted by cross-backend tests for every entity.
- Zero-warning CI (`-D warnings`, clippy pedantic); every user-facing string exists in both `fr` and `en` (key-set parity).
- Printed documents validated by golden snapshots in **both FR and EN**.
- A competent outsider goes from clone to green test suite by following the README alone (J5 criterion).

### Integration

- CSV exports (raw treatments, consumption ledger, season history) have **stable, documented column contracts** — they are consumed by external tools (stock system, spreadsheets, AI agents) and by the inspector's workflow; a breaking change to an export format is a breaking change of the product.
- PDFs are self-contained archival artifacts: dated filenames, embedded fonts, re-openable years later.
