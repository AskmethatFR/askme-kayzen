# Feature Catalog — Habit Management

> Functional node (owner: pm). What the product does today, in business terms, with the acceptance that pins each behavior. Technical rationale lives in [[architecture-overview]] and [[adr-0001-validation-by-construction]].

## F-1 — Request a habit from the board

A user asks the habit board to create a new easy habit by giving a **description** and an **initial duration** in minutes. The board checks the request against the habit rules **before** accepting it; an accepted request becomes a `HabitRequested` fact that the rest of the system can rely on without re-checking.

**Business rules** (see [[glossary]] for terms):
- An easy habit lasts **at most 5 minutes** (5 is accepted).
- A title has **1 to 50 characters** after trimming surrounding whitespace (1 and 50 are accepted; a whitespace-only title is rejected).
- The board holds **at most 5 habits in parallel** — a 6th request is rejected as board-full.
- **No two identical habits** on the board: identical = same title, ignoring case and surrounding whitespace ("Lire une page" and "lire une page " are the same habit). A duplicate is rejected — and reported as a duplicate even when the board is also full.

**Acceptance (pinned by tests in `src/habit_management/use-cases/request-habit/request-habit.rs`):**

| Given | When | Then |
|---|---|---|
| A valid title (1, mid, or 50 chars) and duration ≤ 5 | Requesting a habit | Exactly one `HabitRequested` is published, carrying a generated id, the title, and the duration; the caller gets the id back; the board records the request |
| Duration 6, or empty title, or 51-char title | Requesting a habit | The request is rejected with the specific rule violation; **nothing is published** |
| A board already holding 5 habits | Requesting a 6th | Rejected as board-full; nothing published, board unchanged |
| A title already on the board (any case, surrounding spaces ignored) | Requesting it again | Rejected as duplicate — even if the habit was requested but not yet created, and even on a full board |

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
- No way yet for a habit to leave the board: the future **"ancrée"** (anchored) rule will free a slot; until then the board can fill to 5 and stay full.
