# Feature Catalog — Habit Management

> Functional node (owner: pm). What the product does today, in business terms, with the acceptance that pins each behavior. Technical rationale lives in [[architecture-overview]] and [[adr-0001-validation-by-construction]].

## F-1 — Request a habit from the board

A user asks the habit board to create a new easy habit by giving a **description** and an **initial duration** in minutes. The board checks the request against the habit rules **before** accepting it; an accepted request becomes a `HabitRequested` fact that the rest of the system can rely on without re-checking.

**Business rules** (see [[glossary]] for terms):
- An easy habit lasts **at most 5 minutes** (5 is accepted).
- A description has **1 to 50 characters** (1 and 50 are accepted).

**Acceptance (pinned by tests in `src/habit_management/use-cases/request-habit/request-habit.rs`):**

| Given | When | Then |
|---|---|---|
| A valid description (1, mid, or 50 chars) and duration ≤ 5 | Requesting a habit | Exactly one `HabitRequested` is published, carrying a generated id, the description, and the duration; the caller gets the id back |
| Duration 6, or empty description, or 51-char description | Requesting a habit | The request is rejected with the specific rule violation; **nothing is published** |

## F-2 — Habit created from an accepted request

When the system handles a `HabitRequested` fact, the corresponding habit is created and persisted with the same id, description, and duration. Handling never fails on business rules — the request was already validated at the board (see [[adr-0001-validation-by-construction]]).

**Acceptance (pinned by tests in `src/habit_management/use-cases/create-habit-on-request/create-habit-on-request.rs`):**

| Given | When | Then |
|---|---|---|
| A published `HabitRequested` | Handling it | The habit exists in the repository with the same id, description, duration |
| A full round trip (request → handle) | — | End-to-end: the requested habit is the persisted habit |

## Not available yet (deliberate — manual development resumes from here)

- No automatic hand-off between F-1 and F-2 in production: the outbox is not drained by any dispatcher yet.
- No user interface (Dioxus UI not wired).
- Direct habit creation (the old `CreateHabit` command) was **removed**: the board request is the only entry point.
