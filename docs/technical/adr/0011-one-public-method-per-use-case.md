---
id: "adr-0011-one-public-method-per-use-case"
type: "technical"
owner: "architect"
status: "current"
updated: "2026-07-27"
relations:
  related:
    - "architecture-overview"
    - "adr-0007-habit-lifecycle-aggregate"
    - "adr-0008-goal-based-dose-user-paced-progression"
  depends-on:
    - "adr-0006-cqrs-light"
answers:
  - "How many public methods may a command, a query or a use case expose?"
  - "Two gestures are symmetric (grow/lighten, pause/resume, anchor/readmit) — one type with two methods, or two types?"
  - "Is the duplication between GrowGoal and LightenGoal an accident to refactor away?"
  - "May I extract a shared trait, base type, Direction enum or load→mutate→save helper between two symmetric use cases?"
  - "Does the one-method rule apply to the domain layer too — must Habit be split?"
  - "Why is there no AdjustGoal type when the screen says « Ajuster, à votre rythme »?"
decided_in:
  - "2026-07-27 slice 3 adjust-goal cycle (human ruling, overruled the Architect's proposal)"
---

# ADR 0011 — One public method per use case (the application layer's unit of responsibility)

> **One-liner**: A command, a query or a use case exposes **exactly one public method**. Two symmetric gestures are **two types**, each with its own struct, its own error enum, its own wiring and its own tests — and the duplication that produces is the **accepted choice, not debt**. The rule **stops at the application layer**: the domain keeps cohesive multi-method types.
> **Links**: [[adr-0006-cqrs-light]] (the read side this shapes — one query use case per screen, and now one public method per query), [[adr-0008-goal-based-dose-user-paced-progression]] (the `grow`/`lighten` gestures whose implementation forced the question), [[architecture-overview]] (the layer table where the two use cases now sit side by side).
>
> **Provenance — human ruling, Architect proposal overruled.** The Architect proposed a single `AdjustGoal` type carrying two methods (`grow` / `lighten`), on the usual "one concept, one type" reading. The human overruled it and stated the standing rule verbatim: *« Single responsability principale donc une commande, un use case, une query ont toujours qu'une seul méthode public. »* This ADR records the ruling and its perimeter so it is not relitigated on slices 5 (`pause-resume`), 6 (`anchor-habit`) or 7 (`readmit-habit`), each of which presents the exact same symmetric-pair temptation.

## Context

Slice 3 (`adjust-goal`) delivers two symmetric gestures over one aggregate field: raise the goal by one minute, lower it by one minute. Both load a habit by id, call one aggregate method, save. They differ in a single verb.

Every instinct trained on DRY points at one type with two methods — the two paths share their struct, their dependencies (`HabitRepository`, `Clock`), their failure mode (unknown habit) and their shape (load → mutate → save). The screen even names the pair: « Ajuster, à votre rythme ».

The counter-pressure is the Single Responsibility Principle read at the level of the *reason to change*. A type with `grow` and `lighten` has two reasons to change, and the shared surface makes each change reach the other. The question this ADR settles is which reading governs the **application layer**, and how far the answer travels.

## Decision

| Facet | Decision | Anchor |
|---|---|---|
| The rule | A command, a query or a use case exposes **exactly one public method**. Two gestures = two types | `core/src/habit_management/use_cases/grow_goal.rs`, `core/src/habit_management/use_cases/lighten_goal.rs` |
| Method name | `execute` for a command, `handle` for a query — unchanged; the rule constrains the *count*, not the name | idem, `core/src/habit_management/queries/get_habit_detail.rs` |
| Each type owns its error | `GrowGoalError` and `LightenGoalError` are **distinct enums** with identical variants today. A shared error type would be a shared surface, and the first divergence would have to unpick it | `core/src/habit_management/use_cases/grow_goal.rs`, `core/src/habit_management/use_cases/lighten_goal.rs` |
| Each type owns its wiring | Two fields on `Services`, two constructor calls, two `Rc::clone` pairs | `app/src/composition.rs` |
| Each type owns its tests | The unknown-habit case, the out-of-bounds id case and the `Display` assertion are written **twice**, once per use case. Sharing them would reintroduce the coupling through the test suite | idem |
| **The duplication is the choice** | The resulting duplication between `GrowGoal` and `LightenGoal` is **accepted, deliberate and not to be refactored away**. A later reader is not looking at debt someone failed to clean up | — |
| **Perimeter: stops at the application layer** | The rule governs commands, queries and use cases. It does **not** reach the domain: `Habit` legitimately keeps `grow` / `lighten` / `toggle_done` / `is_done_on`, `Goal` keeps `grown` / `lightened` / `value`. Splitting a cohesive aggregate into one-method types is exactly how an aggregate becomes anemic ([[adr-0007-habit-lifecycle-aggregate]] keeps `Habit` the single lifecycle root) | `core/src/habit_management/domain/habit.rs`, `core/src/habit_management/domain/goal.rs` |
| `Ajuster` survives as **copy only** | « Ajuster, à votre rythme » is the screen's eyebrow text. **No code type bears that name** — not a struct, not a module, not a trait | `app/src/views/habit_detail.rs` |

## Explicitly forbidden by this ruling

Each of these was considered during slice 3 and each is out — for the same reason: it recreates the shared surface the rule exists to prevent.

| Forbidden | Why |
|---|---|
| A shared struct or base type between the two use cases | The shared surface *is* the coupling; hiding it behind a base type does not remove it |
| A trait both implement (`AdjustsGoal`, `Command`, …) | No second consumer needs the abstraction; it exists only to host the duplication |
| An `AdjustGoal` type with `grow` / `lighten` | The overruled proposal, verbatim |
| A `Direction` / `Adjustment` enum parameterizing one type | Two responsibilities behind one method and a discriminant — the same defect with an extra branch |
| A generic (`AdjustGoal<D: Direction>`) | Type-level version of the same |
| A mutualised load → mutate → save helper | The shape being identical is not a reason to share it; the shape is three lines |

## Rejected alternatives

| Alternative | Why rejected |
|---|---|
| **One `AdjustGoal` type, two methods** (the Architect's proposal) | Overruled by the human. Two public methods = two reasons to change on one type; every future change to one gesture reaches the other through the shared struct, error enum and tests |
| Applying the rule to the domain as well | Would produce an anemic `Habit` — behavior scattered into one-method services, which is the anti-pattern [[adr-0007-habit-lifecycle-aggregate]] was written to avoid. The aggregate's methods are cohesive *because* they guard one invariant set |
| Deduplicating "later, once there are three" | The rule is not a threshold heuristic. Three symmetric pairs are already scheduled (slices 5, 6, 7); a "later" that never arrives would still be a promise this ADR would have to break |
| Sharing only the tests | The tests are where the coupling would bite hardest: a divergence in one gesture's failure mode would first appear as a broken shared test, and the cheapest fix is always to weaken the assertion |

## Consequences / Constraints

- **MUST**: give every new command, query or use case **exactly one** public method.
- **MUST**: give each one its own error enum, its own `Services` field and its own tests, even when a sibling's are identical today.
- **MUST NOT**: introduce a shared struct, trait, base type, `Direction` enum, generic or mutualised load→mutate→save helper between symmetric use cases.
- **MUST NOT**: name a code type `AdjustGoal` (or any equivalent umbrella over a symmetric pair). `Ajuster` is screen copy.
- **MUST NOT**: apply this rule to domain types — `Habit`, `Goal`, `StepHistory` and `CompletionHistory` keep their cohesive method sets.
- **Accepted consequence — visible duplication.** `grow_goal.rs` and `lighten_goal.rs` are near-identical files, as will be the pause/resume, anchor/readmit pairs. Reviewers, linters and future agents will all be tempted to merge them. **This node is the answer**: cite it, do not re-decide it.
- **Applies ahead of the code**: slices 5 (`pause-resume`), 6 (`anchor-habit`) and 7 (`readmit-habit`) each specify a symmetric pair. Each yields **two** use cases.
