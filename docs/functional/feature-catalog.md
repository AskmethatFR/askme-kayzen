# Feature Catalog — Habit Management

> Functional node (owner: pm). What the product does today, in business terms, with the acceptance that pins each behavior. Technical rationale lives in [[architecture-overview]] and [[adr-0001-validation-by-construction]].

> The acceptance tables below are mirrored as spec-only Gherkin in `docs/functional/features/habit-management/`: F-1 → [[add-habit]]. Delivered since: [[today-habit-list]], [[mark-done]], [[adjust-goal]], [[practice-staircase]], [[pause-resume]], [[anchor-habit]], [[readmit-habit]], [[habit-stats]], [[ritual]] and [[week-recap]]. Every scenario there resolves to a test through its `// @scenario:` anchor (`scenario_audit.py`).

## F-1 — Add a habit to my daily life

A user creates a new habit by giving a **title** and a **daily goal** in minutes. The system checks the request against the habit rules **before** accepting it; validation is synchronous, and creation happens in the same gesture.

> **Amended 2026-07-23 by `[[adr-0008-goal-based-dose-user-paced-progression]]`**: the dose is now a soft **goal** (default 5 min from the Add screen), **floor 1, no upper ceiling** — the old "≤ 5 minutes" cap is dropped.

**Business rules** (see [[glossary]] for terms):
- A habit carries a **daily goal ≥ 1 minute** (a soft target — flexible, no upper limit; a goal of 0 is rejected).
- A title has **1 to 50 characters** after trimming surrounding whitespace (1 and 50 are accepted; a whitespace-only title is rejected).
- The daily life holds **at most 5 habits in parallel** — a 6th is rejected as daily-life-full.
- **No two identical habits** in the daily life: identical = same title, ignoring case and surrounding whitespace ("Lire une page" and "lire une page " are the same habit). A duplicate is rejected — and reported as a duplicate even when the daily life is also full.

**Acceptance (pinned by tests in `core/src/habit_management/use_cases/add_habit.rs`):**

| Given | When | Then |
|---|---|---|
| A valid title (1, mid, or 50 chars) and a goal ≥ 1 (including **above 5**) | Adding a habit | A new habit is created with a generated id, the title, and the goal; the caller gets the id back |
| Goal 0, or empty title, or 51-char title | Adding a habit | The request is rejected with the specific rule violation; nothing is created |
| Daily life already holding 5 habits | Adding a 6th | Rejected as daily-life-full; nothing created, daily life unchanged |
| A title already in the daily life (any case, surrounding spaces ignored) | Adding it again | Rejected as duplicate — even on a full daily life |

## ~~F-2 — Create habit from request~~ — **RETIRED**

This step was merged into F-1 (slice 6): habit creation is now one gesture, one write via `AddHabit`. The two-step *request* → *handle* flow and the `HabitRequested` fact no longer exist.

## F-3 — See today's habits

The Today screen lists the active habits in the daily life, each with its title, its goal in minutes, and whether it is already done today. Paused habits appear in their own zone below. Everything shown is **derived on read** from the habit's own history — nothing about "today" is stored (see [[adr-0006-cqrs-light]]).

**Acceptance (pinned by tests in `core/src/habit_management/queries/list_board_habits.rs`, mirrored as [[today-habit-list]]):**

| Given | When | Then |
|---|---|---|
| Daily life with no habit | Asking for today's habits | An empty list |
| Daily life holding one active habit | Asking for today's habits | A summary with the habit id, title and goal; not done today while no completion exists for today |
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

## F-6 — Pause a habit, and take it back

A habit can be set aside at any moment and taken back in a single gesture. Pausing it removes it from the daily list and places it under **« En pause · aucune pression »**, below the day's habits. Its detail becomes a **rest screen**: the practice staircase it already earned, and one way back — « La reprendre ». Nothing else is offered there, because a pause is real rest: nothing to practise, nothing to adjust.

Resuming works from either place — one tap on the paused row in Aujourd'hui, or the button on its detail — and the habit returns to the day with **every day it had already lived left untouched**.

A paused habit **keeps its seat in the daily life**. The five slots count habits that have not been anchored, so a sixth is still refused while one is paused. This is deliberate: resuming can then never fail. *Amends the designer's literal `active = !paused && !anchored` cap formula — see `[[design-ecrans]]`.*

The day's tally counts only the habits still in the day: a habit at rest is not a habit missed.

**Acceptance (pinned by tests in `core/src/habit_management/use_cases/pause_habit.rs`, `.../resume_habit.rs`, `.../queries/list_board_habits.rs` and `app/src/views/`, mirrored as [[pause-resume]]):**

| Given | When | Then |
|---|---|---|
| An active habit in the daily life | Pausing it | It leaves the Today list and appears in the paused zone |
| A paused habit | Resuming it | It is active again, back in the Today list, its completion history untouched |
| Daily life holding 5 habits, one of them paused | Adding a new habit | Refused as daily-life-full — a paused habit keeps its seat so resuming can never fail |
| A paused habit | Opening its detail | It offers to resume it and shows its practice staircase — neither the ritual, nor growing, nor lightening |

## F-7 — Anchor a habit that has become natural

A user can mark a habit "ancrée" whenever they feel it has become natural — a **user gesture**, never a system suggestion; no habit-streak threshold triggers it. Anchoring **removes the habit's entry from the daily life**: the seat is freed, and the title is freed with it, in the same act — not a filter that "stops counting" the habit, an actual removal (see [[adr-0012-synchronous-cross-aggregate-coordination]]). The habit itself is untouched: its completion and step histories stay exactly as they are, and it can still be marked done — anchoring ends the seat, not the habit.

The user stays on the habit's detail, which re-renders as a sober "anchored" screen: title, goal, its practice staircase — **no gesture at all**, nothing left to adjust or pause, only history to see.

The new **Ancrées** screen lists every anchored habit and states how many there are. Aujourd'hui links to it as « Mes habitudes ancrées · N », shown only once N ≥ 1.

> **Deferred, not built:** the designer's node also draws each anchored habit's last 7 days as dots, and a footer « Vous suivez N / 5 habitudes en parallèle. » Neither ships this slice — no scenario asks for them, and until a screen can mark an anchored habit done, the dots would freeze at the day of anchoring and replay a stale history forever. The footer belongs with slice 7, which is where daily-life-full refusal becomes the actual subject.

**Business rules** (see [[glossary]]):
- Anchoring is always user-initiated; nothing detects readiness and nothing suggests it — no streak, no 10-of-14 threshold.
- The daily life's cap of 5 counts **entries**, not habits: anchoring removes the entry, so the freed seat and the freed title are one act, not two.
- An anchored habit can still be marked done — anchoring changes what the daily life knows, not what the habit is.

**Acceptance (pinned by tests in `core/src/habit_management/use_cases/anchor_habit.rs`, `.../queries/list_anchored_habits.rs` and `app/src/views/`, mirrored as [[anchor-habit]]):**

| Given | When | Then |
|---|---|---|
| Daily life holding 5 habits | The user anchors one of them | A new habit can be added and is accepted — the daily life counts non-anchored entries only |
| An active habit | The user anchors it | It leaves the Today list and is counted on the Ancrées screen |
| An anchored habit | It is marked done | Today's completion is recorded — anchoring ends the seat, not the habit |
| A habit completed on 10 of the last 14 days | The user opens its detail | Nothing suggests anchoring it — anchoring is user-initiated |

## F-8 — Read a habit's story as a recap

The detail screen shows, under the practice staircase and in every habit state (active, paused, anchored), the habit's whole life told without guilt: **days done**, **other days** (never "failed", never "empty"), **minutes of practice accumulated**, how often the goal was **grown** and **lightened**, and one adaptive sentence — never a congratulations, never a reproach. The current goal is already stated by the lede and the adjust buttons; the recap does not restate it.

Everything is **derived on read** from the two dated histories (completions + steps): nothing about the recap is stored (see [[adr-0006-cqrs-light]]). The recap counts every day from creation to today, done or not (`days done + other days = the habit's age`); minutes are the sum of each completed day against the goal in force that day — total practised time, never a gain over the starting goal. After **7 days without practice** the sentence acknowledges the rest (« Elle se repose en ce moment. Elle vous attend, sans presser. »); a habit never done opens on « Un début parfait. Tout est encore devant. »; otherwise « Vous la faites vivre, à votre rythme. ». There is **no streak anywhere** — an empty day is never a failure, and the recap is a reading, not a gesture (nothing on it is clickable).

**Acceptance (pinned by tests in `core/src/habit_management/queries/get_habit_detail.rs` and `app/src/views/habit_detail.rs`, mirrored as [[habit-stats]]):**

| Given | When | Then |
|---|---|---|
| A habit whose life spans 30 days, completed on 12 of them | The user opens its recap | It reads « 12 réalisés » and « 18 autres jours », and the days without practice are never named a failure |
| A habit grown 3 times and lightened once, now at 7 minutes | The user opens its recap | It reads « 3 fois grandie » and « 1 fois allégée »; the lightening is never named a setback |
| A habit completed on two days whose goal was 5 minutes, then on one day whose goal was 6 | The user opens its recap | It reads « 16 minutes de pratique accumulées », and the label says time practised, never gain over the starting goal |
| A habit practised at least once, with no completion for the last 10 days | The user opens its recap | The message acknowledges the rest without blaming, because an empty day is never a failure |
| A habit created today and not yet done | The user opens its recap | The message reads « Un début parfait », because an empty start is still a start |
| A habit created today and not yet done | The user opens its recap | It reads « 0 réalisé · 1 autre jour » — the recap counts the day it was created |

## F-9 — Read the week as continuous improvement

The Week screen answers one question — *am I growing?* — and is forbidden from answering it with blame. It is a **reading, nothing on it is a gesture**, and it reports on the whole daily life at once rather than habit by habit.

It opens on a **large figure: the minutes actually lived** — every completed day, counted against the goal in force on that day, summed across every habit. Never a gain over the starting goal: someone who practises faithfully for months without ever growing has lived hundreds of minutes, and a gain figure would tell them « 0 ». Pausing or anchoring a habit never takes its lived minutes back — the past is not rewritten by what the daily life looks like today, so **every habit counts, whatever its state**.

Under it, **one row per habit**: its title, its `starting → current` line, and a mini-curve of one bar per recorded goal step. The curve draws the **intention** (each goal change), not the practice — the Detail screen's staircase already draws the practice (F-5). Bar heights are **relative to their own row**, so a row announces a shape of progression, not a volume of effort; two habits at very different scales read alike, and that is the point. A habit that has never been practised still draws its single starting bar.

Then the **rhythm**: a rolling seven-day window ending today, oldest day first, one dot per day, lit when **at least one** habit was practised that day. Never a gap and never a hole — an unpractised day is the same dot, in standby. **The week's word is derived from that same rhythm**, so the two can never disagree: a week with no practice at all rests (« Elle se repose en ce moment »), a week that never began is a perfect start (« Un début parfait »), and any other week grows.

**Acceptance (pinned by tests in `core/src/habit_management/queries/get_week_recap.rs` and `app/src/views/week.rs`, mirrored as [[week-recap]]):**

| Given | When | Then |
|---|---|---|
| Habits practised over several weeks, some paused, one anchored | The user opens the week | The large figure counts every lived minute, whatever each habit's state |
| A daily life where nothing was ever practised | The user opens the week | It reads « Un début parfait », never a bare « 0 » as a verdict |
| A habit grown from 2 to 4 minutes | The user opens the week | Its row reads « 2 → 4 » and draws one bar per goal step, not one per completed day |
| A brand-new habit, never practised | The user opens the week | Its row still draws its journey — a single bar — because an empty start is still a start |
| Two habits, each practised on a different day of the window | The user opens the week | Both days are lit — a day is lit when *any* habit was practised, not only the first |
| A window with unpractised days | The user opens the week | Those days show a dim dot, never a gap and never a mark of failure |

## F-10 — What I do stays mine, across closings

The app is worth using only if closing it costs nothing. A habit added, a day marked done, a goal grown — all of it is there again at the next launch, with no gesture asked of the user to make it so. There is no save button, because saving is not something a practitioner should have to think about.

A **first** launch shows a genuine empty daily life and its invitation — never a demo habit. What the user sees is always their own life, from the very first second.

When the stored data cannot be read — a file written by an older version, a truncated write, an edit made by hand — the board simply **starts empty**, and the unreadable data is **set aside** rather than discarded, before anything can overwrite it. A format bug costs a recoverable loss, never a silent one. The user is not asked to repair anything and is never shown an error for it.

When the device offers **no durable place at all** to keep habits, the app **refuses to start** and says so plainly, rather than running as though it were saving. That refusal is deliberate: an app that quietly forgets is worse than an app that admits it cannot remember. This holds on desktop and mobile; on the web the same protection is not yet in place (see *Not available yet*).

**Acceptance (pinned by tests in `app/src/composition.rs`, `app/src/main.rs` and `app/src/views/data_unavailable.rs`, mirrored as [[persistence]]):**

| Given | When | Then |
|---|---|---|
| A habit added, then the app closed | The user opens the app again | The habit is there, with its goal |
| A habit marked done today, then the app closed | The user opens the app again | It still reads as done today |
| A device that has never run the app | The user opens the app | An empty daily life and its invitation — never a demo habit |
| Stored data that cannot be read | The user opens the app | An empty daily life, no error, and the unreadable data set aside |
| A device offering no durable place to store habits | The user opens the app | A calm explanation instead of the board, and nothing written anywhere |

## Not available yet (deliberate — manual development resumes from here)

- **On the web, an unreachable browser store is indistinguishable from an empty one**: in private browsing, or with storage disabled, every save is a silent no-op and the board looks like it works until the page is reloaded. The refusal that protects desktop and mobile (F-10) does not fire there yet, which is why its scenario is scoped to the native platform.
- **No Android build yet** — the device's own data directory has still to be resolved through the platform (issue #35), and no store listing, signing key or account exists (issue #28).
- All six screens act. **Today** (list + mark done + the paused zone + the Ancrées link), **Add**, **Detail** (adjust the goal, read the practice staircase, pause/resume, anchor), **Ancrées** (list + count only — see F-7), **Ritual** ([[ritual]], issue #13) and **Week** ([[week-recap]], issue #22). The Week screen is complete as a **read-only** screen: its weekly reflection (hansei) was dropped from the product on 2026-08-21 by owner ruling (issue #23, closed as won't-do) — the week recap informs, it collects nothing, so no screen writes anything but habits.
- Anchoring is now two-way: the Ancrées screen offers « La remettre dans mon quotidien » on each anchored habit (`[[readmit-habit]]`, slice 7), refusable on a full daily life (« Le quotidien est complet · pour la remettre, ancrez-en une autre d'abord ») or on a title already retaken (« Elle est déjà dans votre quotidien »). The screen's parallel-count footer « Vous suivez N / 5 habitudes en parallèle » is shipped; its per-habit dots remain deferred (see F-7). The recap ([[habit-stats]], slice 8) is shipped — see F-8.
- The multi-step *request* → *create-on-request* flow is **gone** (slice 6): habit creation is one gesture via `AddHabit`, one write, no published events.
