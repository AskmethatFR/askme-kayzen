---
id: "adr-0013-set-based-validation-outside-aggregates"
type: "technical"
owner: "architect"
# MADR status: accepted. Recorded as `current` because that is this graph's
# vocabulary (draft | current | superseded | deprecated) — see docs/INDEX.md.
status: "current"
updated: "2026-08-17"
relations:
  supersedes:
    - "adr-0002-habitboard-stateful-aggregate"
    - "adr-0012-synchronous-cross-aggregate-coordination"
  related:
    - "architecture-overview"
    - "adr-0006-cqrs-light"
    - "adr-0009-quality-gates"
    - "adr-0011-one-public-method-per-use-case"
  depends-on:
    - "adr-0001-validation-by-construction"
    - "adr-0007-habit-lifecycle-aggregate"
answers:
  - "How do I tell an aggregate invariant from set-based validation?"
  - "Where do « at most 5 habits in the daily life » and « no duplicate title » live now?"
  - "Why was HabitBoard deleted — did it outlive its usefulness, or was it a mistake?"
  - "Isn't reading the whole set inside a use case a leak of the invariant out of the domain?"
  - "Is the read-then-write in AddHabit::execute a race (TOCTOU)?"
  - "What exactly reopens the concurrency question, and what is the fix when it does?"
  - "Where does the number 5 live, and why is a constant on Habit not the same mistake?"
  - "What IS a real Habit invariant, then? (the counter-example landed in PR 2)"
  - "A transition grows the set a rule bounds — where does the check go?"
decided_in:
  - "2026-08-17 drop-habit-board refactor (human ruling on the invariant test)"
  - "2026-08-17 PR 2 guard-lifecycle-transitions (the counter-example landed; the symmetry table)"
  - "2026-08-18 slice 7 readmit-habit cycle (the standing MUST's first application lands)"
---

# ADR 0013 — Set-based validation lives outside aggregates

> **⚠️ Supersedes [[adr-0002-habitboard-stateful-aggregate]] and [[adr-0012-synchronous-cross-aggregate-coordination]].** Both nodes are correct reasoning built on a false premise: that « au plus 5 habitudes dans le quotidien » and « pas de doublon de titre » are aggregate invariants defining a boundary. They are not. `HabitBoard`, its repository, its event, the outbox, `RequestHabit` and `CreateHabitOnRequest` are **deleted** — 10 files, 1093 lines.

> **One-liner**: A rule whose predicate depends on another aggregate's **mutable** state, or on the composition of a set that changes over time, is **not an invariant** and must never be hosted by an aggregate — it is **set-based validation**, and it lives in the use case that performs the gesture, reading the set through the existing repository port.
> **Links**: [[adr-0001-validation-by-construction]] (per-instance invariants, which this node does **not** touch — they stay in the VO constructors), [[adr-0007-habit-lifecycle-aggregate]] (`Habit` is the only aggregate left; its **AD-9** is the counter-example this node closes on — landed 2026-08-17, amending AD-4), [[adr-0011-one-public-method-per-use-case]] (`AddHabit` is one gesture, one public method), [[architecture-overview]] (where this is applied).

## Context

LOCAL-2 met two rules no single `Habit` can enforce — **max 5 habits in parallel** and
**no duplicate title** — and concluded, in [[adr-0002-habitboard-stateful-aggregate]], that
"a rule that spans several habits needs one object that sees them all and mutates
atomically — the textbook definition of an aggregate boundary". `HabitBoard` was invented to
be that object: a private registry, its own repository port, its own error family, its own
event, and — because a `Habit` was created only on the consuming side of that event — an
outbox and an event handler.

Six slices later the human applied their own test to the capacity rule:

> *« Si cette valeur change pendant le cycle de vie de l'agrégat alors ce n'est pas un
> invariant ; on peut déplacer la règle dans un use case ou dans l'infra si plus simple. »*

Three checks follow from it, and the capacity rule fails all three:

1. **Can a single instance violate it alone?** No. A `Habit` does not know how many other
   habits exist. Nothing a `Habit` can do to itself breaks « at most 5 ».
2. **Does the predicate depend on another aggregate's mutable state?** Yes. The real
   predicate is **`state != Anchored`**, and `state` lives on `Habit` and **changes during
   its lifecycle** (`pause` / `resume` / `anchor`, `core/src/habit_management/domain/habit.rs:78-88`).
3. **Does the constrained set change composition?** Constantly — add, anchor, readmit.

No aggregate can hold an invariant whose predicate depends on another aggregate's mutable
state. That is the whole error, and every complication in the deleted design descends from
it: the board had to be told about each habit twice (an entry at request time, a removal at
anchor time), it had to hold a **copy** of the `HabitTitle` that `Habit` already owned, and
"an entry exists ⟺ a habit is non-anchored" became a fact written in two places that
nothing could keep in agreement.

The measured fact that made the exit cheap: **`HabitBoard` was read by no query.** All three
queries (`list_board_habits`, `get_habit_detail`, `list_anchored_habits`) read
`HabitRepository` alone, and had since slice 1. The board was write-only, and `Habit` was
already de facto the source of truth for everything the user sees.

## Decision

| Facet | Decision | Anchor |
|---|---|---|
| The rule's category | « At most 5 in the daily life » and « no duplicate title » are **set-based validation**, not invariants. Same family as cross-entity uniqueness in any relational system | — |
| Where it lives | In the **use case that performs the gesture**, reading the set through the **existing** `HabitRepository` port. No new port, no new aggregate, no query object | `core/src/habit_management/use_cases/add_habit.rs` |
| The set | `repository.all()` filtered on `habit.state() != LifecycleState::Anchored`. The predicate is read **live, at the moment of the gesture** — it is never mirrored, cached or recorded | `core/src/habit_management/use_cases/add_habit.rs` |
| Guard order | **Duplicate before capacity.** Load-bearing, not stylistic: on a full daily life a re-submitted title must be told it is a duplicate, not that there is no room. It preserves LOCAL-2's own precedence (*duplicate wins on a full board*) | `core/src/habit_management/use_cases/add_habit.rs` |
| The write | **Exactly one** — `repository.save(&Habit::new(...))`. Creation went from three writes (board entry, outbox event, habit) to one | idem |
| Error shape | `AddHabitError { InvalidHabit(HabitError), DuplicateHabit, DailyLifeFull { max } }` — the same three rejection reasons as the deleted `HabitBoardError`, owned by the use case that decides them | idem |
| Where the number lives | `Habit::MAX_IN_DAILY_LIFE: usize = 5`, on the domain type — **the number is domain vocabulary, the rule is the use case's** | `core/src/habit_management/domain/habit.rs:46` |
| What is gone | `HabitBoard`, `HabitBoardRepository`, `HabitBoardEvent`, `DomainEventPublisher`, `InMemoryHabitBoardRepository`, `InMemoryOutbox`, `RequestHabit`, `CreateHabitOnRequest`, and the app-service layer that drained the outbox | — |

### The full gesture, in order

```
AddHabit::execute(title: String, goal: u32) -> Result<(), AddHabitError>
  1. HabitId::new(guid_generator.generate())      # the generated id is parsed too (adr-0010)
  2. HabitTitle::new(title)                       # per-instance invariants — adr-0001, untouched
  3. Goal::new(goal)
  4. repository.all() filtered on state != Anchored        # the SET
  5. any(|h| h.title().matches(&title)) -> DuplicateHabit  # BEFORE 6, deliberately
  6. len() >= Habit::MAX_IN_DAILY_LIFE -> DailyLifeFull    # capacity
  7. repository.save(&Habit::new(id, title, goal, clock.today()))   # THE ONLY WRITE
```

Steps 1–3 are [[adr-0001-validation-by-construction]] unchanged: the per-instance rules stay
in the VO constructors, and `Habit::new` is still infallible. This node only relocates the
rules that were **never** per-instance.

### Why `HabitBoard` was a mistake, not a design that outlived its usefulness

The distinction matters, because "it served us and we grew out of it" invites re-inventing it
at the next cross-habit rule.

`HabitBoard` existed to **host a rule that is not an aggregate invariant**. From that single
error came every other property of the deleted design — none of which the product ever asked
for:

| Consequence of the error | What it cost |
|---|---|
| The board needed the habit's identity and title before the habit existed | Creation was split into a request side and a consuming side, which required an **event**, which required an **outbox** and a **dispatcher** |
| The same fact was written twice (a board entry ⟺ a non-anchored habit) | Nothing could keep the two in agreement; [[adr-0012-synchronous-cross-aggregate-coordination]] had to reason about which of the two writes may crash first |
| `BoardEntry` held a copy of `HabitTitle` | The uniqueness rule read the copy, not the habit — a second source of truth for a value `Habit` already owns |
| The board could not see `LifecycleState`, by its own MUST | The « non-anchored » predicate had to be **simulated** by adding and removing entries (`release`), instead of simply being read |

Recognising it as a mistake is what makes the exit final. There is no future rule that brings
`HabitBoard` back: the next cross-habit rule gets the same treatment as this one.

## Rejected alternatives

| Alternative | Why rejected |
|---|---|
| Keep `HabitBoard`, teach its capacity check to count non-anchored habits | This is the alternative [[adr-0002-habitboard-stateful-aggregate]] itself rejected, and its rejection was **right on its own terms**: it forces the board to learn another aggregate's mutable state on every request. The error is one level up — the board should not exist, not "the board should learn more" |
| A `DailyLife` aggregate wrapping all non-anchored habits | Same mistake with a better name. Its boundary would still be defined by a predicate over another aggregate's mutable state, and it would still hold copies of data `Habit` owns. An aggregate whose contents change because a *different* aggregate changed state is not an aggregate |
| A domain service `DailyLifePolicy` holding the two rules | Moves the code, keeps the confusion. The rules are not *always true* (the definition of an invariant), they are **checked at one gesture** — that is application-layer orchestration, and [[adr-0011-one-public-method-per-use-case]] already puts one gesture in one use case |
| A `Specification` object over the habit set | Pattern for its own sake: one caller, one gesture, five lines. [[adr-0011-one-public-method-per-use-case]]'s standing ruling is that the duplication is cheaper than the abstraction |
| Enforce uniqueness in infrastructure (a unique index) | The human's test explicitly allows this (*« ou dans l'infra si plus simple »*), and it becomes the right answer under persistence + concurrency (see the escalation trigger below). Today there is no index to put it in — the store is a `HashMap` in one process — so the use case is the simpler of the two, not the compromise |

## The objection this node upholds and pays

[[adr-0002-habitboard-stateful-aggregate]] raised three objections against "count the habits
in the use case". They deserve a verdict each, because two of them died with the design and
one is real.

| adr-0002's objection | Verdict today |
|---|---|
| *"Invariant leaks out of the domain into orchestration"* | **True. A purity objection, accepted as the price.** The rule is now readable in a use case rather than guarded by a type. That is what it costs to stop pretending a set-based rule is an invariant, and it is cheaper than the two-writes design it replaces. Recorded openly so nobody re-discovers it as a defect |
| *"Non-atomic — two concurrent requests both pass the query"* | **Unreachable in this runtime, by the type system.** See the next section. It becomes real under one of three named conditions, and its fix is not "re-read" |
| *"Unsound: the habit only exists after the event handler runs, so the query cannot see a just-accepted request"* | **Dies with the handler.** There is no handler and no deferral — `AddHabit::execute` reads the set and writes the habit in one call. The soundness gap was created by the event indirection that the board itself required |

## Is the read-then-write a race? — No, and here is the proof

Between step 4 (`repository.all()`) and step 7 (`repository.save`) there is a window in the
source. **Its width in this runtime is zero, and that is enforced by the type system, not by
convention.** Security verified each leg independently this cycle:

- `Rc<dyn HabitRepository>` is unconditionally **`!Send` / `!Sync`** — no value holding one
  can reach another thread. The compiler refuses it; no discipline is involved.
- `InMemoryHabitRepository` uses `RefCell`, also `!Sync`.
- There is **no suspension point** between the read and the write. A grep over `core/src` and
  `app/src` finds zero `async`, zero `.await`, zero `thread::spawn`, zero `tokio`, zero `Arc`,
  zero `Mutex`/`RwLock`, zero `unsafe`.

One thread, no suspension, no sharing: the two statements are as atomic as a critical
section, without one.

### Security's escalation trigger — verbatim, do not paraphrase

> Reopen the security review of `AddHabit::execute` as soon as **any** of these becomes true:
> **(a)** `HabitRepository` gains an `async fn`, or a use case `await`s between `all()` and
> `save()`; **(b)** `Rc<dyn HabitRepository>` becomes `Arc<...>`, or the trait gains a
> `Send`/`Sync` bound; **(c)** a `HabitRepository` adapter backed by storage shared with
> another process, tab, worker or client is introduced (IndexedDB, service worker, server).

Two points Security insisted on, and they are the ones a reader gets wrong:

- **(a) alone suffices — async reopens the window with no thread at all.** Two concurrent
  `execute` futures on a single thread can interleave `all()` and `save()`; `Send`/`Sync` is
  not required for the bug. "We are single-threaded" is not a defence against `.await`.
- **(c) is the realistic case for this product.** Two browser tabs of the same origin sharing
  one IndexedDB each read 4 and each write → **6 habits in the daily life**.

**When one of the three falls, the fix is not to re-read.** Re-reading narrows the window; it
does not close it. The cap must move **into the write itself** — a conditional write, a
uniqueness constraint on a normalised title, or a serialisable transaction. That is the
same "or in the infrastructure if simpler" branch of the human's test, arriving at the moment
it stops being optional.

## Where the constant lives, and why that is not the same mistake

`Habit::MAX_IN_DAILY_LIFE: usize = 5` sits on the domain type
(`core/src/habit_management/domain/habit.rs:46`), while the rule that reads it sits in
`AddHabit`. That split is deliberate:

- **The number is domain vocabulary.** « Cinq » is a word the product says; it belongs with
  the type it bounds, exactly like `HabitTitle::MAX_LEN` and `Goal::MIN`.
- **The rule is the use case's.** `Habit` exposes the bound; it does not enforce it, cannot
  enforce it, and gains no method that pretends to.

Publishing a constant is not hosting an invariant. The mistake this node corrects was giving
an aggregate a *behaviour* it could not honour — not giving a type a *name* for a number.

## The counter-example that keeps the rule usable — **LANDED 2026-08-17 (PR 2)**

This node is not "invariants are suspicious". The rule cuts both ways, and the
counter-example is no longer hypothetical — it shipped the same day, in PR 2
`guard-lifecycle-transitions`.

**Transition-table legality IS a real invariant of `Habit`.** *An anchored habit cannot be
resumed* is verifiable on **one instance**, consults **no other aggregate**, and depends on
nothing that changes outside that instance — it passes all three checks in the Context
section. It belongs **inside `Habit`**, and it is now there:
`core/src/habit_management/domain/habit.rs` guards all three transitions
(`pause()` requires `Active`, `resume()` requires `Paused`, `anchor()` requires `Active`,
each returning `Result<(), TransitionError>`), and the three use cases
(`core/src/habit_management/use_cases/pause_habit.rs`,
`core/src/habit_management/use_cases/resume_habit.rs`,
`core/src/habit_management/use_cases/anchor_habit.rs`) map the refusal to a flat variant of
their own error. Full table and rationale: [[adr-0007-habit-lifecycle-aggregate]] **AD-9**,
which amends AD-4.

**The symmetry, crisply — this is the calibration to reuse:**

| Rule | Nature | Where it lives | Why |
|---|---|---|---|
| « au plus 5 dans le quotidien » | **Set-based validation** — the predicate is a property of a *set whose composition changes* | The use case, reading the set live (`core/src/habit_management/use_cases/add_habit.rs`) | No single `Habit` can violate it alone; no `Habit` can see the set |
| « on ne reprend pas ce qui est ancré » | **Invariant** — a property of *one instance*, verifiable on it alone | The aggregate (`core/src/habit_management/domain/habit.rs`) | Consults no other aggregate; depends on nothing that changes elsewhere |

Same three checks, opposite verdicts, one cycle apart. When the next rule arrives, ask which
column it lands in before asking where to put the code.

**And the two are not independent** — that is the part worth keeping. The cap is what makes
the transition guard *security*-relevant rather than merely tidy: an unguarded
`Anchored → Active` was the one transition that could grow the set the cap bounds. The
induction that closes it is recorded in [[adr-0007-habit-lifecycle-aggregate]] AD-9, and it
is the reason slice 7's `ReadmitHabit` must re-apply the **set** check itself — it is the
first transition since `AddHabit` that increases the count.

## Consequences / Constraints

- **MUST**: apply the three checks before hosting any new cross-entity rule — *can one
  instance violate it alone? does the predicate depend on another aggregate's mutable state?
  does the constrained set change composition?* Two "no"s and a "no" make an invariant; any
  "yes" makes it set-based validation.
- **MUST**: put set-based validation in the **use case performing the gesture**, reading
  through an existing repository port. Never in a new aggregate, never in a domain service,
  never in a `Specification`.
- **MUST**: keep the duplicate guard **before** the capacity guard in `AddHabit::execute` —
  it decides which rejection the user is told about on a full daily life.
- **MUST**: keep `AddHabit::execute` at **one** write. A second write reintroduces the class
  of defect this refactor removed (a partial write consuming a seat without creating a habit).
- **MUST NOT**: reintroduce `HabitBoard` or any equivalent "set aggregate" for the next
  cross-habit rule.
- **MUST NOT**: mirror `state != Anchored` into a stored flag, counter or registry. The
  predicate is read live or not at all.
- **MUST NOT**: answer a future concurrency problem by re-reading the set. Move the cap into
  the write (see the escalation trigger).
- **MUST**: when a *transition* increases the size of a set some rule bounds, re-apply that
  set check **in the use case**, before the transition and before the single write. The
  aggregate guard is not a substitute — it cannot see the set. **LANDED 2026-08-18 (slice 7)**
  — `ReadmitHabit` is the second use case applying this, with the same predicate
  (`state() != Anchored`), the same duplicate-before-capacity precedence, and the same
  one-write/refusal-leaves-nothing contract ([[adr-0007-habit-lifecycle-aggregate]] AD-9,
  discharged). The predicate now serves three sites — the `AddHabit`/`ReadmitHabit` caps and
  the read-side footer (`ListAnchoredHabits.in_daily_life`) — and is still never mirrored.
- **Out of scope**: ~~readmission (slice 7)~~ **— landed 2026-08-18, see the LANDED note above**; ~~the transition-table guard (PR 2)~~ **— landed
  2026-08-17, see the counter-example section**; persistence, and the
  conditional-write/unique-index question it will force.
