---
id: "adr-0016-snapshot-store-persistence"
type: "technical"
owner: "architect"
# MADR status: accepted. Recorded as `current` because that is this graph's
# vocabulary (draft | current | superseded | deprecated) — see docs/INDEX.md.
status: "current"
updated: "2026-08-23"
relations:
  related:
    - "architecture-overview"
    - "adr-0001-validation-by-construction"
    - "adr-0010-crate-boundary-trust-boundary"
    - "adr-0013-set-based-validation-outside-aggregates"
  depends-on:
    - "adr-0003-two-crate-workspace"
    - "adr-0007-habit-lifecycle-aggregate"
answers:
  - "Where do habits actually live on each platform, and why is it not one substrate everywhere?"
  - "Why is localStorage not the storage of this app, given it is a Dioxus app?"
  - "Why is the store port synchronous, and what breaks the day it becomes async?"
  - "Why does HabitRepository know nothing about persistence, and why was it not changed?"
  - "Where do the serde derives live, and may a domain type carry them?"
  - "A stored payload no longer parses — what happens to it, and what does the user see?"
  - "Why can a save not report its failure, and what would reopen that decision?"
  - "Two writers save at once — who wins, and is that a defect or a decision?"
  - "The platform offers no durable location — does the app fall back or refuse?"
  - "Is an id read back from storage trusted the way a generated one is?"
decided_in:
  - "#34 — 2026-08-23, persistence slices 1 (PR #36) and 2 (PR #37). Substrate reversed by research after the proposal; owner rulings on last-writer-wins, on loud refusal, and on the fixed quarantine name"
---

# ADR 0016 — Habits are persisted through a synchronous single-slot `SnapshotStore`, over a file on native and `localStorage` on web

> **One-liner**: Durability is an **adapter concern**. A `SnapshotStore` port owns **one slot**, not a key space, and is **synchronous by hard constraint**; a **decorator** over the in-memory repository hydrates from it once and rewrites the whole snapshot on each write, so `HabitRepository`, every use case, every query and the whole domain stay unaware that a file exists. The native substrate is a **file** — not `localStorage`, which is unreachable *and* non-durable there.
> **Links**: [[architecture-overview]] (where this sits in the layering), [[adr-0013-set-based-validation-outside-aggregates]] (condition (a) — the reason the port may not be async), [[adr-0001-validation-by-construction]] (the single door the wire format must not become a second one of), [[adr-0010-crate-boundary-trust-boundary]] (its trigger #7 — persistence — fired in this cycle).

## Context

The proposal on the table assumed `localStorage` on every platform, on the reading that a Dioxus app is a web app everywhere. Investigation reversed it, and the finding is worth more than the decision it produced because it is expensive to re-derive:

- **Dioxus `mobile` compiles native**, not wasm — `aarch64-linux-android`, through wry/tao/jni/ndk. `web-sys` therefore does not exist on that target, and the only bridge to the WebView's JS is `document::eval`, which is **asynchronous**. A store port that must be synchronous cannot be built on it.
- **`localStorage` inside a WebView is not durable storage.** Android clears it together with the app's cache; iOS drops it across relaunches. Even where it is reachable, it does not satisfy the requirement.

So the native substrate is a file, and `localStorage` survives only as the web arm, where it is the platform's own answer and nothing else is available.

## Decision

| Facet | Decision |
|---|---|
| Substrate | **A file** on native (desktop and mobile alike), **`localStorage`** on web. Selected at **compile time** (`#[cfg(target_arch)]`), never at runtime, so neither arm's dependency enters the other platform's build ([[adr-0003-two-crate-workspace]]) |
| The port | `SnapshotStore` — **one durable slot**, not a key space: load the whole payload or nothing, save the whole payload. Both methods **synchronous and total** (no error channel — see *Accepted costs*). It lives in `infrastructure/`, **not** in the domain: no domain type and no use case may name it, and a port only two adapters share would otherwise make the domain announce a concern it must not know |
| Where durability attaches | A **decorator** over the in-memory repository: hydrate once at construction, serve reads from memory, rewrite the snapshot on each write. **`HabitRepository` is not touched** — durability adds an adapter, never a port method, and every existing test double keeps working untouched |
| Write granularity | The **whole** snapshot per write. The daily life is capped ([[adr-0007-habit-lifecycle-aggregate]]), so a full rewrite is kilobytes; differential writes would buy complexity and no measurable gain |
| Wire format | A **persistence DTO confined to the infrastructure module** carries the `serde` derives. Domain types are rebuilt through their own validating constructors, so nothing under `domain/` is coupled to a wire format and no second construction door opens ([[adr-0001-validation-by-construction]]) |
| Decoding | **All-or-nothing.** One unparsable field discards the whole payload. A partially rebuilt habit is not a lesser failure than an empty board — it is a worse one, because it looks like data |
| Unreadable payload | **Quarantined at a fixed, never-timestamped sibling slot, before anything can overwrite it.** The invariant, stated once: *the primary is never destroyed without a copy existing first.* The board then simply starts empty |
| No durable location | **Refuse at startup, with an explicit screen.** Never fall back to somewhere writable |

## Rejected alternatives

| Alternative | Why rejected |
|---|---|
| `localStorage` on every platform (the original proposal) | Unreachable on the native Android target, and not durable in a WebView even where reachable — it fails the requirement on both counts |
| Reaching the WebView's `localStorage` through `document::eval` | Asynchronous. An `await` between reading the set and writing it reopens condition (a) of [[adr-0013-set-based-validation-outside-aggregates]] — the daily-life cap stops being enforceable by the type system |
| Persistence methods on `HabitRepository` (or a second, "persistent" port) | Durability is not a domain concern and no caller has a persistence question to ask. Leaving the port alone is what kept the diff at zero lines there and every test double valid |
| `serde` derives on the domain types | A second construction path around the validating constructors — precisely [[adr-0010-crate-boundary-trust-boundary]]'s trigger #6 |
| Best-effort decoding (keep the habits that parse) | Admits a half-rebuilt board that the user cannot distinguish from a whole one. All-or-nothing plus a quarantined copy loses nothing and lies about nothing |
| One file (or key) per habit | Invents partial states nothing needs: the snapshot is one consistent whole, and a per-habit key space would have to reconcile them |
| Timestamped quarantine copies | An unreadable payload is re-copied on every launch; a growing set of copies fills a disk. A fixed name makes the copy idempotent (owner ruling) |
| Falling back to a temp or world-writable directory when no data directory resolves | Measured **reachable by default on Android**: `dirs` routes that platform through its Linux path and returns `None` without `HOME`/`XDG_DATA_HOME`. The fallback would therefore have shipped, by default, an app that persists nothing on every launch — with no signal to anyone |

## Accepted costs, and what would reopen them

### `save()` has no error channel — the root of most of this cycle's review findings

The port's `save` cannot report failure. This is deliberate (a total, synchronous port is what keeps the write path free of a failure mode the domain has no answer to), and it has a standing price that must be understood before anyone reuses the shape: **with no way to report a failure, every failure must be absorbed — and each absorption is a place data can die quietly.** Three separate instances were found by review in this cycle, all fixed: a failed write that truncated the snapshot *and* the quarantine to empty; a directory occupying the temp name, which made saving permanently impossible with no signal; an existing but unopenable file that read as "no data" and was then overwritten. The individual defects are gone; the **class is open by design**.

Its first corollary is already visible: a bound the caller cannot be *told* about must be enforced **before** the value materialises. That is why the payload size cap is restated at the file adapter rather than left to the codec's parse — by the time a `load()` returns a string, an unbounded read has already happened.

**Escalation trigger** — the day a save failure must reach the user (a sync target, a store shared with another writer, or a product decision to say "not saved"), the answer is a **fallible `save`**, not one more absorption site. Whoever meets that day re-reads this section before adding a fourth.

### Last-writer-wins, for v1 (owner ruling, knowingly)

No revision, no etag, no compare-and-save, no re-read inside `save`. **Scope matters, and the cycle had to learn it**: this acceptance covers **concurrent writers**. It does **not** cover a single writer losing data to an I/O failure — those are defects, and were fixed as defects, not filed under this ruling.

### Degrade loudly, or not at all

When the platform cannot provide what a feature requires, the app says so at startup and stops, rather than substituting a location the OS can clear or another app can read. The rejected fallback above is what this rule is worth in practice: a silent substitute is not a lesser outcome than a refusal, it is a worse one, because nobody learns that the promise is not being kept.

## Consequences / Constraints

- **MUST**: keep `SnapshotStore` synchronous. Making either method `async` reopens [[adr-0013-set-based-validation-outside-aggregates]] condition (a); the fix at that point is to move the cap into the write, not to re-read.
- **MUST**: keep durability in the adapter layer — no use case, query or domain type may learn that a file exists.
- **MUST**: keep the wire format's derives out of `domain/`, and rebuild every domain value through its own constructor on the way back in.
- **MUST**: copy an unreadable payload to its quarantine slot **before** the primary can be overwritten.
- **MUST**: treat a value read back from the store as **user-supplied input**, never as "already validated" — this is [[adr-0010-crate-boundary-trust-boundary]] trigger #7, fired in this cycle.
- **MUST NOT**: add a method to `HabitRepository`, or a second repository port, to serve persistence.
- **MUST NOT**: treat the store as a key space, or make the quarantine name vary per attempt.
- **Deferred work** arising from this decision lives in **one place** — [[architecture-overview]]'s *Open questions / Gaps*. It is not restated here.
