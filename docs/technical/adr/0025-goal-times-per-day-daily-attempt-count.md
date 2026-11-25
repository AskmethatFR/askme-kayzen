---
id: "adr-0025-goal-times-per-day-daily-attempt-count"
type: "technical"
owner: "architect"
status: "current"
updated: "2026-11-25"
relations:
  supersedes:
    # FACET SUPERSESSION: both nodes stay `current`. In
    # [[adr-0008-goal-based-dose-user-paced-progression]] three facets are dead: the goal's
    # unit (minutes), its default of 5, and « completion stays binary ». In
    # [[adr-0007-habit-lifecycle-aggregate]] one facet is dead: « Completions », the ordered
    # set of dates with `toggle_done` inserting or removing today. Every other facet of both
    # nodes stands (floor 1, no ceiling, user-paced grow/lighten, StepHistory, the aggregate
    # root, LocalDate/Clock, LifecycleState).
    - "adr-0008-goal-based-dose-user-paced-progression"
    - "adr-0007-habit-lifecycle-aggregate"
  related:
    - "adr-0016-snapshot-store-persistence"
    - "adr-0011-one-public-method-per-use-case"
    - "adr-0013-set-based-validation-outside-aggregates"
answers:
  - "What does a habit's goal count: minutes, or times per day?"
  - "What happens when the user marks a habit done a second time on the same day?"
  - "Is going past the goal an error?"
  - "How is a day's practice stored, and can a day's count be 0?"
decided_in:
  - "goal as times per day, slice S1 — 2026-11-25"
---

# ADR 0025 — A habit's goal is a number of times per day; each day keeps its count of attempts

> **One-liner**: The goal stops being minutes and becomes **times per day**. Every « done » gesture is **one attempt**: it adds 1 to the day's count and never clears the day. A day's count belongs to one habit and one local date. Going past the goal is a **bonus**, counted and never refused.
> **Links**: [[adr-0008-goal-based-dose-user-paced-progression]] (unit, default and « completion stays binary » superseded; floor 1 and user-paced progression stand), [[adr-0007-habit-lifecycle-aggregate]] (the « Completions » facet superseded; the aggregate stands), [[adr-0016-snapshot-store-persistence]] (the store that will carry the counts).

## Context

[[adr-0008-goal-based-dose-user-paced-progression]] made the goal a soft target in minutes and kept completion binary. The « done » gesture was a same-day toggle ([[adr-0007-habit-lifecycle-aggregate]]): a second tap un-marked the day. For a goal of several times per day, that toggle erases the user's second attempt instead of counting it. And a count that hits the goal and then refuses more attempts would punish extra practice.

## Decision

| Facet | Decision |
|---|---|
| Goal unit | **Times per day.** The floor of 1 and the absence of a ceiling from adr-0008 stand. The default of 5 minutes no longer applies |
| Day's practice | `CompletionHistory` keeps a **count of attempts per local date** (`core/src/habit_management/domain/completion_history.rs`). A day with no attempt has **no entry**, so no stored count is ever 0, and the practised days are exactly the recorded dates |
| The gesture | `Habit::record_attempt(today)` adds 1 (saturating). `record_attempt` never decrements (undo is a separate gesture, see below), so a count never goes below 0. `MarkDone` calls it (`core/src/habit_management/use_cases/mark_done.rs`), still with one public method |
| Past the goal | Accepted and counted. Exceeding the goal is never an error |
| Reads | `Habit::attempts_on(day)` is the day's count; `is_done_on(day)` means at least one attempt. Range reads (`between`) still yield practised days only |
| Independence | A count is keyed by habit (inside its aggregate) and by local date. No count is shared across habits or days |

Undo, added later, is a separate gesture and does not touch the rows above: `Habit::undo_attempt(today)` removes one attempt of today (`UndoAttempt`, one public method), never below zero, and never removes a day that still has attempts. The gesture that records an attempt still never decrements.

## Options considered

- **A count of attempts per day, with no zero entries.** It keeps every attempt, it reads as « done » with no extra rule, and the days that existing reads iterate over stay the same.
- **Keep the toggle and add a separate counter.** Rejected: two sources of truth for « done today », and a second tap would still clear the day.
- **Cap the count at the goal.** Rejected: it turns extra practice into a refusal, against [[design-principes-kaizen]] (no guilt).

## Consequences

- Good: the second tap is never lost, and `is_done_on` and every range read keep their meaning, so their callers need no change.
- Good: an old payload, with each date stored once, reads as one attempt per day.
- Bad: until the snapshot carries the counts, a count above 1 reads back as 1 after a restart. Only in-session counts are exact.
- Bad: an undo gesture can no longer be a second tap. It would have to be a gesture of its own.
