# Ubiquitous Language — Glossary

> Functional node (owner: pm). The words the domain speaks — code identifiers must match these terms. Features using them: [[feature-catalog]].

| Term | Meaning | Code anchor |
|---|---|---|
| **Habit** | An easy habit a user commits to; exists only once its request was accepted and handled. Identified by a `HabitId`. | `src/habit_management/domain/habit.rs` |
| **Habit Board** | The single entry point where habits are requested. Guards the habit rules before any request becomes a fact. Stateless today. | `src/habit_management/domain/habit_board.rs` |
| **Request (a habit)** | Asking the board for a new habit. Validated synchronously; acceptance produces a `HabitRequested` fact. | `src/habit_management/use-cases/request-habit/request-habit.rs` |
| **HabitRequested** | The fact that a valid habit request was accepted. Reliable by construction — carries already-validated values, never re-checked downstream. | `src/habit_management/domain/habit_board_event.rs` |
| **Description** | What the habit is, in the user's words. 1 to 50 characters. Self-validating value (`HabitDescription`). | `src/habit_management/domain/habit_description.rs` |
| **Initial duration** | How long the habit takes, in minutes. At most 5 — that is what makes the habit "easy". Self-validating value (`InitialDuration`). | `src/habit_management/domain/initial_duration.rs` |
| **Outbox** | Where accepted facts wait to be handled. Published in the same transaction as the board's acceptance. | `src/habit_management/infrastructure/in_memory_outbox.rs` |
