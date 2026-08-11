/// Where a habit stands in its own lifecycle. Two states only — no `Anchored`
/// yet (slice 6) — modelled as an enum rather than a `paused: bool` so that an
/// impossible combination (e.g. paused-and-anchored) is never representable
/// (adr-0007).
///
/// No `Default`: `Habit::new` names `Active` explicitly rather than falling
/// back to an implicit one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LifecycleState {
    Active,
    Paused,
}
