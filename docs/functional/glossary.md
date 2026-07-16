# Ubiquitous Language — Glossary

> Functional node (owner: pm). The words the domain speaks — code identifiers must match these terms. Features using them: [[feature-catalog]].

| Term | Meaning | Code anchor |
|---|---|---|
| **Habit** | An easy habit a user commits to; exists only once its request was accepted and handled. Identified by a `HabitId`. | `src/habit_management/domain/habit.rs` |
| **Habit Board** | The single entry point where habits are requested. Guards the habit rules AND its own invariants — at most 5 habits in parallel, no duplicate title — and remembers every accepted request. | `src/habit_management/domain/habit_board.rs` |
| **Request (a habit)** | Asking the board for a new habit. Validated synchronously; acceptance is recorded on the board and produces a `HabitRequested` fact. | `src/habit_management/use-cases/request-habit/request-habit.rs` |
| **HabitRequested** | The fact that a valid habit request was accepted. Reliable by construction — carries already-validated values, never re-checked downstream. | `src/habit_management/domain/habit_board_event.rs` |
| **Title** | What the habit is, in the user's words — a simple title. 1 to 50 characters after trimming; case is preserved but ignored when comparing two titles. Self-validating value (`HabitTitle`). Formerly called "description". | `src/habit_management/domain/habit_title.rs` |
| **Duplicate** | Two habits with the same title, ignoring case and surrounding whitespace. Forbidden on the board; reported as duplicate even when the board is also full. | `src/habit_management/domain/habit_board.rs` |
| **Ancrée (anchored)** | *Future rule*: how a habit will eventually leave the board and free one of the 5 slots. Not implemented yet. | — |
| **Initial duration** | How long the habit takes, in minutes. At most 5 — that is what makes the habit "easy". Self-validating value (`InitialDuration`). | `src/habit_management/domain/initial_duration.rs` |
| **Outbox** | Where accepted facts wait to be handled. Published in the same transaction as the board's acceptance. | `src/habit_management/infrastructure/in_memory_outbox.rs` |
