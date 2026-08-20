---
id: "adr-0015-ritual-countdown-monotonic-animation-clock"
type: "technical"
owner: "architect"
# MADR status: accepted. Recorded as `current` because that is this graph's
# vocabulary (draft | current | superseded | deprecated) — see docs/INDEX.md.
status: "current"
updated: "2026-08-20"
relations:
  supersedes:
    # FACET SUPERSESSION — [[adr-0010-crate-boundary-trust-boundary]] stays `current`.
    # Its DECISION (the crate boundary is the trust boundary; one fallible door;
    # the seven escalation triggers) is untouched, and every other node still
    # depends on it. Exactly ONE accepted consequence is dead: « the `Ritual` route
    # is not covered — its view re-injects the raw route parameter into a `Link` ».
    # Scope also recorded in that node's docs/INDEX.md row.
    - "adr-0010-crate-boundary-trust-boundary"
  related:
    # EXTENSION, not supersession — [[adr-0014-view-wiring-click-dispatch-harness]]
    # established the click-dispatch harness and left its extension to synthetic
    # non-click events open. This node closes that opening and generalises the
    # one-dispatch refusal. Nothing there is annulled; its aria-label convention
    # and its three refusals stand.
    - "adr-0014-view-wiring-click-dispatch-harness"
    - "architecture-overview"
  depends-on:
    - "adr-0003-two-crate-workspace"
    - "adr-0006-cqrs-light"
    - "adr-0007-habit-lifecycle-aggregate"
answers:
  - "How long does the ritual count down, and what happens when it reaches zero?"
  - "Why is the remaining time a subtraction over a monotonic timestamp rather than a tally of ticks?"
  - "What drives the countdown's heartbeat, and why is it not a timer crate or a JS setTimeout loop?"
  - "Why does a throttled, hidden or backgrounded screen come back correct with no code written for that case?"
  - "Is the `Ritual` route still outside the single fallible door?"
  - "A view hands a habit id to a `Link` — is that a trust-boundary bypass?"
  - "How does a test drive a non-click, non-visible event through a Dioxus view?"
  - "Why is the tick sensor located by listener name instead of by `aria-label`?"
  - "Why does the harness build its animation payload with `serde_json::from_value` instead of a local trait impl?"
  - "Why is the countdown a component of its own, separate from the screen that owns the clock?"
  - "Did the ritual grow the dependency graph?"
decided_in:
  - "#13 — 2026-08-20, the ritual times the habit's own goal (GATE 1.5 human approval; owner rulings on the dose and on zero)"
---

# ADR 0015 — The ritual's countdown is a monotonic subtraction, beaten by the platform's animation clock

> **⚠️ Annuls one accepted consequence of [[adr-0010-crate-boundary-trust-boundary]]** — « the
> `Ritual` route is not covered ». That node stays `current`; its decision, its single door and
> its seven escalation triggers are untouched. Only that one consequence is dead, and it died by
> being *strengthened*: the route now goes through the door like every other.
>
> **Extends [[adr-0014-view-wiring-click-dispatch-harness]]** — which left the harness's extension
> to synthetic non-click events open. Nothing there is annulled.

> **One-liner**: a countdown that *tallies* beats is wrong the moment the platform stops beating;
> a countdown that *subtracts* two timestamps is correct at every instant, whatever happened in
> between — so the beat is demoted to a repaint cue, and the cheapest repaint cue on the platform
> is the animation the screen already runs.
> **Links**: [[adr-0010-crate-boundary-trust-boundary]] (the door it now goes through),
> [[adr-0014-view-wiring-click-dispatch-harness]] (the harness it extends),
> [[architecture-overview]] (current shape).

## Context

Every screen before this one was inert: it rendered what a query returned and changed only when
the user acted. The ritual is the first screen that must change **while nobody touches it**, and
that single property is what forced every decision below — a clock source, a correctness model
for elapsed time, and a way to test a view that drives itself.

## Decision

### 1. What is counted, and what happens at zero

The ritual counts **the habit's own current goal**, read from the core through the existing detail
query — never a fixed duration. The dose is the habit's business ([[adr-0007-habit-lifecycle-aggregate]],
[[adr-0008-goal-based-dose-user-paced-progression]]); the screen renders it, it does not choose it.

At zero the timer **stops and waits**. Reaching zero validates nothing: completion stays a gesture
the user makes, never a consequence of elapsed time. This is the Kaizen no-reproach rule applied to
time — a countdown that marked the habit done would turn a soft target into a verdict.

### 2. Remaining time is a subtraction, never a tally

```
remaining = total − (now − started_at)          # saturating at zero
```

`started_at` is a **monotonic** instant captured once when the screen opens; `now` is another
monotonic instant. Nothing accumulates, nothing increments, no beat is ever counted.

**This is the load-bearing decision of the node, and its value is the case nobody has to write
code for.** A tab that is throttled, hidden, backgrounded or simply starved of frames stops
producing beats — a tally would come back *frozen* or *drifted* by exactly the missing beats, and
the fix would be a visibility listener, a catch-up loop, and a reconciliation rule to test. A
subtraction comes back **correct**, because the two timestamps never stopped being true. The
degenerate cases collapse into the nominal one instead of becoming branches.

The monotonic clock is used deliberately, not as an implementation detail: a wall-clock reading
can jump backwards (NTP correction, a user changing the system time), which would make a remaining
duration *grow*. The instant type is the one that reads the same monotonic source under both the
browser and native targets, so the screen has no `cfg`-split notion of time.

**Consequence for the beat**: its rate is a **refresh cadence, not a unit of account**. Changing it
changes how smoothly the dial moves and nothing else; no rate is load-bearing, and no test may
assert on one.

### 3. The heartbeat is the platform's own animation clock

The screen renders a zero-sized, `aria-hidden`, pointer-inert element carrying an infinite CSS
animation, and listens for one animation-iteration event per cycle. Each event refreshes `now`.

Three properties come free, and each is a mechanism that would otherwise have had to be built:

| Property | Why it holds |
|---|---|
| The beat stops when the page is not rendering | The compositor suspends the animation; no callback fires, no work happens, and §2 makes the gap harmless |
| The beat dies with the screen | The listener is part of the render tree, so unmounting removes it. There is no handle to leak and no cleanup to forget |
| The beat needs no runtime | It is the same declarative CSS the rest of this app's motion already uses |

**The sensor is rendered only while time remains.** Stopping the countdown is therefore *removing
the clock*, not ignoring it — the at-zero state has no live machinery at all rather than a
suppressed one.

### 4. The `Ritual` route now crosses the single door — adr-0010's accepted non-coverage is dead

[[adr-0010-crate-boundary-trust-boundary]] accepted, in writing, that this one route never reached
the core: its view forwarded the raw URL segment straight into a `Link`, and nothing parsed it.
Both halves of that sentence are now false. The screen needs the habit's goal, so it calls the
core; the raw segment reaches exactly one place — the query call — and every id the view hands
onward afterwards is a value the **core returned**.

**The property is provenance, not identity.** The id forwarded to a `Link` is byte-for-byte the one
that arrived in the URL, and that is not a bypass: it came back through `HabitId`'s single fallible
constructor, so it is a *laundered* value. This makes the rule stated in [[adr-0010-crate-boundary-trust-boundary]]
sharper rather than weaker — **every route that names a habit is now covered by the door**, with no
exception left to remember, and the seven escalation triggers are unchanged (SSR stays a
dev-dependency, so trigger 4 has not fired).

### 5. Pinning a view that drives itself

The review barrier measured the gap by hand: four mutations that **destroy** the countdown —
deleting the sensor, no-oping its handler, freezing the clock, forcing elapsed time to zero — each
left the whole suite green. The cause was not a missing assertion but a missing *state*: every
behavioural render was taken at t = 0, where « reads the clock » and « ignores the clock » produce
identical HTML. **A self-driving view is unpinned until a test renders it at a non-zero instant and
delivers it a beat.**

Two decisions follow.

**a. The harness dispatches synthetic events, and locates them by listener name.**
[[adr-0014-view-wiring-click-dispatch-harness]]'s `aria-label` convention is a locator for elements
a *user* drives; it does not apply here, and forcing it would be a defect — the sensor is
`aria-hidden` with no visible text, so any label invented for it would violate the house
`"<visible text> · <subject>"` rule and put a test-only handle in the accessibility tree. A
synthetic event is instead located by the **listener registration** the render already emits, which
is the artifact that actually exists for a non-visible target. The three refusals of adr-0014 are
reproduced for it verbatim — unknown name, ambiguous name, second dispatch — and the *one-dispatch*
guard now lives in the shared dispatch path rather than in the click entry point, so it covers
click, synthetic event and any cross sequence by construction rather than by repetition.

**b. The payload is deserialised, and that is forced, not preferred.** The platform's event
converter hard-downcasts to its own concrete serialized-event type, so a locally-written trait
impl compiles and then fails at run time; the only alternative is re-implementing the converter's
entire surface for one event. The payload is therefore built by deserialising a literal into that
concrete type. The failure mode is deliberately loud: the type derives its deserialisation with no
field defaults, so an upstream field change **fails the test suite** instead of silently degrading
into a default-valued event.

**c. The countdown is a component of its own.** The screen that owns the clock also owns the
navigation away from it, and that link cannot render without a mounted router — which is not
constructible from a test here. Splitting the countdown out yields a component that depends on
*remaining* and *total* alone, so a test renders it at any instant it likes.

The split is kept because it is right, not because it is convenient: stateful container /
presentational child, with the tick surfaced as an **event the child raises rather than a mechanism
it owns**. The child reports « a beat happened »; how a beat is produced, and what refreshing means,
stay the parent's business — which is exactly what lets §3's clock source be replaced later without
touching the dial.

### 6. Dependencies

The cycle added **zero resolved packages**: the monotonic-instant crate and the deserialiser were
already in the graph transitively, and the deserialiser is dev-only. Verified on the lockfile
diff, which contains no new package entry. The `app → core` edge and the crate split of
[[adr-0003-two-crate-workspace]] are untouched.

## Rejected alternatives

| Rejected | Why |
|---|---|
| **Count beats** (`remaining -= 1` per tick) | Wrong by construction the moment the platform skips a beat — and it *will*, on every hidden tab. Buys a visibility listener, a catch-up rule and their tests, to reach the accuracy a subtraction has for free |
| **Wall-clock instead of monotonic** | An NTP correction or a manual clock change can make remaining time grow. The bug would be unreproducible and would look like a rendering fault |
| A **fixed one-minute** ritual | Contradicts the habit's own dose ([[adr-0008-goal-based-dose-user-paced-progression]]). The screen would be telling the user a duration the product does not believe in |
| **Validate the habit at zero** | Turns a soft target into a verdict, and makes elapsed time the author of a completion the user never claimed. Owner ruling |
| An **async runtime** timer | Absent from this app's host graph; pulling one in to schedule a repaint is a runtime and a dependency for a job CSS already does |
| A **wasm-only timer crate** | Splits the notion of time by target, in a workspace whose whole point is that the app compiles for several |
| A **new general-purpose timer crate** | An unvetted dependency, weighed against zero new packages for the chosen path |
| A JS **`setTimeout` loop** through the eval bridge | Leaks an orphaned callback chain per visit — the loop outlives the screen that started it, with no handle to cancel. Re-creates the lifetime problem §3 gets structurally |
| Locate the sensor by **`aria-label`** | Would put a test-only handle in the accessibility tree of an `aria-hidden` element, and break the `"<visible text> · <subject>"` convention that keeps [[adr-0014-view-wiring-click-dispatch-harness]]'s handles honest |
| A **local event-data trait impl** instead of deserialising | Compiles, then fails at run time on the platform's hard downcast. The honest alternative is re-implementing the converter's whole surface for one event |
| **Keep one component** and test the countdown through a router | The router context is not constructible from a test here; the seam would have to be faked rather than obtained |

## Consequences / Constraints

- **MUST**: derive remaining time by subtracting two monotonic instants. **MUST NOT** accumulate,
  decrement or count beats — a value that depends on how many beats arrived is a defect, not an
  approximation.
- **MUST**: treat the beat's rate as a repaint cadence. No test asserts on it; changing it is a
  visual choice.
- **MUST**: keep the at-zero state free of live machinery — the sensor is *absent*, not muted.
- **MUST**: pin a self-driving view at a **non-zero** instant. A render taken at t = 0 cannot
  distinguish reading the clock from ignoring it, and a suite of such renders survives the deletion
  of the whole mechanism.
- **MUST**: locate a user-driven element by its `aria-label` ([[adr-0014-view-wiring-click-dispatch-harness]])
  and a non-visible synthetic target by its listener registration. **MUST NOT** invent an
  `aria-label` for an element no user can reach.
- **MUST**: keep one dispatch per screen instance — the guard is in the shared path, so this holds
  across event kinds without being restated per entry point.
- **MUST NOT**: read « the ritual screen refuses a paused or anchored habit » as a domain guard. It
  is a screen refusal, the same shape as every other lifecycle gating here
  ([[adr-0007-habit-lifecycle-aggregate]] AD-4); the use cases still accept the id, and the fix
  lands at the entry point with the ownership check when [[adr-0010-crate-boundary-trust-boundary]]
  trigger 2 fires.
- **Out of scope**: persisting or resuming a ritual across a reload (nothing survives a restart);
  what a completed ritual should offer next; the week screen.

## Open questions / Gaps

- [ ] **A ritual does not survive a reload.** The starting instant lives in the screen; reopening
      restarts the count. Harmless while nothing persists at all, and a decision the persistence
      cycle inherits rather than one to pre-empt here.
