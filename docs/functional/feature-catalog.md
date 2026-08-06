# Feature Catalog — Habit Management

> Functional node (owner: pm). What the product does today, in business terms, with the acceptance that pins each behavior. Technical rationale lives in [[architecture-overview]] and [[adr-0001-validation-by-construction]].

> The acceptance tables below are mirrored as spec-only Gherkin in `docs/functional/features/habit-management/`: F-1 → [[request-habit]], F-2 → [[create-habit-on-request]]. Delivered since: [[today-habit-list]], [[mark-done]], [[adjust-goal]] and [[practice-staircase]]. Every scenario there resolves to a test through its `// @scenario:` anchor (`scenario_audit.py`).

## F-1 — Request a habit from the board

A user asks the habit board to create a new habit by giving a **title** and a **daily goal** in minutes. The board checks the request against the habit rules **before** accepting it; an accepted request becomes a `HabitRequested` fact that the rest of the system can rely on without re-checking.

> **Amended 2026-07-23 by `[[adr-0008-goal-based-dose-user-paced-progression]]`**: the dose is now a soft **goal** (default 5 min from the Add screen), **floor 1, no upper ceiling** — the old "≤ 5 minutes" cap is dropped.

**Business rules** (see [[glossary]] for terms):
- A habit carries a **daily goal ≥ 1 minute** (a soft target — flexible, no upper limit; a goal of 0 is rejected).
- A title has **1 to 50 characters** after trimming surrounding whitespace (1 and 50 are accepted; a whitespace-only title is rejected).
- The board holds **at most 5 habits in parallel** — a 6th request is rejected as board-full.
- **No two identical habits** on the board: identical = same title, ignoring case and surrounding whitespace ("Lire une page" and "lire une page " are the same habit). A duplicate is rejected — and reported as a duplicate even when the board is also full.

**Acceptance (pinned by tests in `core/src/habit_management/use_cases/request_habit.rs`):**

| Given | When | Then |
|---|---|---|
| A valid title (1, mid, or 50 chars) and a goal ≥ 1 (including **above 5**) | Requesting a habit | Exactly one `HabitRequested` is published, carrying a generated id, the title, and the goal; the caller gets the id back; the board records the request |
| Goal 0, or empty title, or 51-char title | Requesting a habit | The request is rejected with the specific rule violation; **nothing is published** |
| A board already holding 5 habits | Requesting a 6th | Rejected as board-full; nothing published, board unchanged |
| A title already on the board (any case, surrounding spaces ignored) | Requesting it again | Rejected as duplicate — even if the habit was requested but not yet created, and even on a full board |

## F-2 — Habit created from an accepted request

When the system handles a `HabitRequested` fact, the corresponding habit is created and persisted with the same id, description, and duration. Handling never fails on business rules — the request was already validated at the board (see [[adr-0001-validation-by-construction]]).

**Acceptance (pinned by tests in `core/src/habit_management/use_cases/create_habit_on_request.rs`):**

| Given | When | Then |
|---|---|---|
| A published `HabitRequested` | Handling it | The habit exists in the repository with the same id, description, duration |
| A full round trip (request → handle) | — | End-to-end: the requested habit is the persisted habit |

## F-3 — See today's habits

The Today screen lists the habits on the board, each with its title, its goal in minutes, and whether it is already done today. Everything shown is **derived on read** from the habit's own history — nothing about "today" is stored (see [[adr-0006-cqrs-light]]).

**Acceptance (pinned by tests in `core/src/habit_management/queries/list_board_habits.rs`, mirrored as [[today-habit-list]]):**

| Given | When | Then |
|---|---|---|
| A board with no habit | Asking for today's habits | An empty list |
| A board holding one habit | Asking for today's habits | A summary with the habit id, title and goal; not done today while no completion exists for today |
| A habit already marked done today | Asking for today's habits | The summary reports it done, read from the completion history |

## F-4 — Mark a habit done today

Tapping a habit's target records today as done; tapping it again clears it. One completion per local date, no timestamp, kept forever — the same-day gesture is a toggle, so a mistake costs nothing.

**Acceptance (pinned by tests in `core/src/habit_management/use_cases/mark_done.rs`, mirrored as [[mark-done]]):**

| Given | When | Then |
|---|---|---|
| A habit not done today | Marking it done | Today's local date is recorded in its completion history |
| A habit already done today | Marking it done again | Today's completion is removed |
| An id matching no habit | Marking it done | Rejected; nothing is recorded |

## F-5 — Read a habit's recent practice as a staircase

The detail screen draws one bar per calendar day over the **last seven days**. A day that was practised is a full bar standing at the goal active that day; a day that was not keeps the same bar at low opacity — present, never a gap and never a warning. The drawing credits **practice, never intent**: adjusting the goal adds no bar.

> Replaces the decisions staircase slice 3 shipped (one bar per goal change), on the owner's correction of 2026-07-27: *« le graph grandit à l'ajout d'une minute alors qu'elle devrait ajouter dans le graph quand un jour est complété »*. The step history stays as data — it gives each bar its height — but stopped being a drawing.

**Business rules** (see [[glossary]]):
- The window is **always seven days**, whatever the habit's age or activity — never one bar per completion, never one per goal change, never a variable span.
- A bar's height is the goal **active on its own day**: the last step dated on or before it. Growing today raises today and the days after, never the days already lived.
- A day older than the habit itself stands at the goal the habit **started on** — an empty start is still a start.

**Acceptance (pinned by tests in `core/src/habit_management/queries/get_habit_detail.rs` and `app/src/views/habit_detail.rs`, mirrored as [[practice-staircase]]):**

| Given | When | Then |
|---|---|---|
| A habit whose goal is 5 minutes | Marking it done today | Today's bar is full, standing at 5 minutes |
| A habit not marked done yesterday | Opening its detail | Yesterday's bar is still drawn, faint — neither a gap nor a warning |
| A habit not marked done today | Choosing *grandir* | No bar is added and no day becomes lived; the days already lived keep their height |
| Done at 5, grown to 6, done again the next day | Opening its detail | The earlier bar stands at 5, the later at 6 |
| A habit created three weeks ago | Opening its detail | Seven bars, one per day of the window |
| A habit created today and not yet done | Opening its detail | Seven faint bars, standing at the goal it started on |

## Not available yet (deliberate — manual development resumes from here)

- **Nothing survives a restart**: every store is in-memory (`InMemoryHabitRepository`, `InMemoryHabitBoardRepository`), and the Today screen is seeded with three demo habits at startup. No persistence adapter exists yet.
- Three of the six screens act: **Today** (list + mark done), **Add**, and **Detail** (adjust the goal, read the practice staircase). Ritual, Week and Ancrées are routed stubs.
- No way yet for a habit to leave the board: the **"ancrée"** (anchored) rule will free a slot (slice 6); until then the board can fill to 5 and stay full. Pause/resume (slice 5), goal adjustment (slice 3) and the recap (slice 8) are specified but not built — see [[lifecycle-backlog]].
- Direct habit creation (the old `CreateHabit` command) was **removed**: the board request is the only entry point.

> The F-1 → F-2 hand-off **does** run in production: `AddHabit` (app service) requests on the board, then drains the outbox synchronously and lets the create handler persist. The dispatcher is synchronous and in-process by design at this stage.
