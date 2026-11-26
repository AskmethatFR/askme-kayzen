---
id: "adr-0026-older-snapshot-upgraded-on-read"
type: "technical"
owner: "architect"
status: "current"
updated: "2026-11-26"
relations:
  related:
    - "adr-0016-snapshot-store-persistence"
    - "adr-0025-goal-times-per-day-daily-attempt-count"
answers:
  - "What happens to a snapshot saved under the minutes model after the update?"
  - "Is an older snapshot version quarantined?"
  - "When is an upgraded snapshot written back?"
decided_in:
  - "goal as times per day, slice S11 — 2026-11-26"
---

# ADR 0026 — An older snapshot version is upgraded on read, never set aside

> **One-liner**: A version 1 snapshot (the minutes model) is read and upgraded in memory. Its habits keep their title, completion days and state; the goal becomes the default (3 times a day) and the steps are re-seeded. The next normal save writes version 2. Nothing is quarantined for being an older version.
> **Links**: [[adr-0016-snapshot-store-persistence]] (the store, its all-or-nothing decode and its quarantine), [[adr-0025-goal-times-per-day-daily-attempt-count]] (the times-per-day goal this upgrade lands on).

## Context

[[adr-0025-goal-times-per-day-daily-attempt-count]] changed what a goal counts: times per day, not minutes. Snapshots on users' devices still hold minute goals under version 1. The codec quarantined any version other than its own ([[adr-0016-snapshot-store-persistence]]), so raising the version alone would have emptied every board on the first launch after the update.

## Decision

| Facet | Decision |
|---|---|
| Versions | The codec writes version **2**. It reads versions **1 and 2**; any other version is unreadable and quarantined as before (`core/src/habit_management/infrastructure/habit_snapshot_codec.rs`) |
| Version 1 habit | Title, state and completion days are kept. Each stored day is one attempt; a day stored twice counts two |
| Version 1 goal | **`Goal::DEFAULT`**. The old minute goals are not converted |
| Version 1 steps | Still validated as before (non-empty, parseable dates, non-decreasing, goal at least 1), so a corrupt version 1 payload is still quarantined. Then dropped: the history is `StepHistory::seeded(first step's date, Goal::DEFAULT)`, so the creation date stands |
| Write-back | **Never during load.** The stored text stays as it was until the next normal save, which writes version 2. A crash after load leaves the version 1 payload intact |
| Where it is decided | One place: the codec's decode of a version 1 record |

## Options considered

- **Upgrade on read with the default goal.** Keeps everything the user can still read honestly, and loses no data on disk until a save.
- **A minutes-to-times formula.** Rejected: there is no honest mapping from minutes of practice to times per day.
- **Quarantine version 1.** Rejected: the board would empty on the first launch after the update.
- **A new storage key for version 2.** Rejected: it would orphan the existing data under the old key.

## Consequences

- Good: every habit survives the update with its history of practised days.
- Good: corrupt or unknown payloads take the same quarantine path as before.
- Bad: each upgraded habit restarts at the default goal; a user who had grown a goal must grow it again.
- Bad: the step history before the update is gone after the first save.
