# Deferred work

Items surfaced during reviews that are real but not actionable in their originating story.

## Deferred from: code review of story-1.1 (2026-07-13)

- **Story 1.3 must tolerate pre-1.3 `occurred_at > recorded_at` rows.** The 1.1 journal is append-only, so any inverted (occurred_at, recorded_at) pair recorded before 1.3 introduces the `occurred_at ≤ recorded_at` constructor invariant becomes permanent history that the forward-only guard cannot retroactively reject. 1.3's guard should tolerate (or backfill-flag) pre-1.3 violators rather than assume journal-wide validity. [crates/pomone-domain/src/field_event.rs]

## Deferred from: code review of story-1.2 (2026-07-13)

- **Story 1.3: reopen `corrects` linkage on same-day ties.** In 1.2, `recorded_at` is the caller date at midnight, so two settling events on a task the same day tie on `recorded_at`; `latest_settling_event` then breaks the tie by the random UUID `id`, so a same-day done→reopen→done→reopen can link `corrects` to the wrong settling event (task state stays correct — only the audit link is imprecise; unreachable from the current UI). 1.3's real injected timestamps remove the ties. [crates/pomone-app/src/facts.rs]
