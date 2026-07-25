# Ubiquitous Language — Glossary

> Functional node (owner: pm). The words the domain speaks — code identifiers must match these terms. Features using them: [[feature-catalog]].

| Term | Meaning | Code anchor |
|---|---|---|
| **Habit** | An easy habit a user commits to; exists only once its request was accepted and handled. Identified by a `HabitId`. | `core/src/habit_management/domain/habit.rs` |
| **Habit Board** | The single entry point where habits are requested. Guards the habit rules AND its own invariants — at most 5 habits in parallel, no duplicate title — and remembers every accepted request. | `core/src/habit_management/domain/habit_board.rs` |
| **Request (a habit)** | Asking the board for a new habit. Validated synchronously; acceptance is recorded on the board and produces a `HabitRequested` fact. | `core/src/habit_management/use_cases/request_habit.rs` |
| **HabitRequested** | The fact that a valid habit request was accepted. Reliable by construction — carries already-validated values, never re-checked downstream. | `core/src/habit_management/domain/habit_board_event.rs` |
| **Title** | What the habit is, in the user's words — a simple title. 1 to 50 characters after trimming; case is preserved but ignored when comparing two titles. Self-validating value (`HabitTitle`). Formerly called "description". | `core/src/habit_management/domain/habit_title.rs` |
| **Duplicate** | Two habits with the same title, ignoring case and surrounding whitespace. Forbidden on the board; reported as duplicate even when the board is also full. | `core/src/habit_management/domain/habit_board.rs` |
| **Ancrée (anchored)** | *Future rule*: how a habit will eventually leave the board and free one of the 5 slots. Not implemented yet. | — |
| **Goal (*objectif*)** | A habit's soft **daily goal** in minutes — default 5, floor 1, **no upper ceiling**. A flexible aim, not a limit: doing less is fine, doing more is a bonus. Self-validating value (`Goal`). Replaces the old "initial duration ≤ 5" (superseded by `[[adr-0008-goal-based-dose-user-paced-progression]]`). | `core/src/habit_management/domain/goal.rs` |
| **Completion (*fait*)** | A day a habit was marked done. One per local date, toggleable the same day, kept forever, no timestamp. Lives inside the `Habit` aggregate. | `core/src/habit_management/domain/completion_history.rs` |
| **Completion history** | The set of a habit's completed days. One-completion-per-day is **structural** (an ordered set, never a runtime guard). | `core/src/habit_management/domain/completion_history.rs` |
| **Mark done (*toggle*)** | Toggling today's completion for a habit — fills the ink if empty, clears it if already done. A lifecycle transition kept internal (no event published). | `core/src/habit_management/use_cases/mark_done.rs` |
| **Local date** | A calendar day in the user's timezone. Library-free by design — no `chrono` in the domain; an epoch-day integer internally. | `core/src/shared/local_date.rs` |
| **Clock** | Port giving the domain "today" as a `LocalDate`; the real `SystemClock` (the only place `chrono` is used) sits at the infra edge. | `core/src/shared/clock.rs` |
| **Outbox** | Where accepted facts wait to be handled. Published in the same transaction as the board's acceptance. | `core/src/habit_management/infrastructure/in_memory_outbox.rs` |
