use serde::{Deserialize, Serialize};

use crate::habit_management::domain::completion_history::CompletionHistory;
use crate::habit_management::domain::goal::Goal;
use crate::habit_management::domain::habit::Habit;
use crate::habit_management::domain::habit_id::HabitId;
use crate::habit_management::domain::habit_title::HabitTitle;
use crate::habit_management::domain::lifecycle_state::LifecycleState;
use crate::habit_management::domain::step_history::StepHistory;
use crate::shared::local_date::LocalDate;

/// The persistence DTO for a habit snapshot — `serde` derives live here and
/// nowhere under `domain/`, so nothing in the aggregate is coupled to a wire
/// format (the tech spec's "dedicated persistence DTO" facet).
#[derive(Serialize, Deserialize)]
struct SnapshotV1 {
    v: u32,
    habits: Vec<HabitRecord>,
}

#[derive(Serialize, Deserialize)]
struct HabitRecord {
    id: String,
    title: String,
    state: StateRecord,
    steps: Vec<StepRecord>,
    completions: Vec<i64>,
}

#[derive(Serialize, Deserialize)]
struct StepRecord {
    on: i64,
    goal: u32,
}

#[derive(Serialize, Deserialize)]
enum StateRecord {
    Active,
    Paused,
    Anchored,
}

/// Explicit, both-ways mapping between `Habit` and its `{"v":1,"habits":[…]}`
/// wire form. `encode` never fails (it only ever serializes data this crate
/// already validated); `decode` is all-or-nothing — one unparsable field
/// discards the whole payload rather than admitting a partially-rebuilt
/// habit. `pub(crate)`-visible module only (see `mod.rs`): this being a
/// public constructor of `Vec<Habit>` from an arbitrary string would have
/// made every `pub(crate)` on the domain rehydration constructors it calls
/// pointless — a second, wider door next to `AddHabit` (adr-0010).
pub struct HabitSnapshotCodec;

impl HabitSnapshotCodec {
    const VERSION: u32 = 1;

    /// Above this, a stored payload is refused before a single byte is
    /// handed to `serde_json` — Security's F-4: `load()` gives an S2 file
    /// adapter no way to say "too big", so nothing bounded a `fs::read_to_string`
    /// of an arbitrary size (measured: 14.9 MB / 2,000,000 completions parsed
    /// whole). 4 MB leaves two orders of magnitude above adr-0007 FUT-1's
    /// modelled *organic* growth (~24 bytes per grow/lighten round trip) —
    /// that figure never answered a payload that *arrives* already holding a
    /// million steps.
    pub(crate) const MAX_PAYLOAD_BYTES: usize = 4 * 1024 * 1024;

    pub fn encode(habits: &[Habit]) -> String {
        let snapshot = SnapshotV1 {
            v: Self::VERSION,
            habits: habits.iter().map(Self::encode_habit).collect(),
        };
        serde_json::to_string(&snapshot).expect(
            "a snapshot built entirely from already-validated domain values always serializes",
        )
    }

    /// `None` covers every unreadable shape alike: oversized, invalid JSON, a
    /// missing field, an unknown `v`, or a stored value that no longer
    /// parses through its VO constructor — the caller cannot tell which, and
    /// does not need to; every case reads as "start from an empty board".
    ///
    /// Parses the wire form exactly once: `serde_json` still has to walk
    /// every byte to build `SnapshotV1` regardless of `v`, so a second,
    /// version-only pass bought no early exit — only a doubled cost on a
    /// large payload. What an unknown version *does* skip is the next,
    /// domain-level pass: `decode_habit` never runs, so no stored field is
    /// pushed through a VO constructor.
    pub fn decode(payload: &str) -> Option<Vec<Habit>> {
        if payload.len() > Self::MAX_PAYLOAD_BYTES {
            return None;
        }
        let snapshot: SnapshotV1 = serde_json::from_str(payload).ok()?;
        if snapshot.v != Self::VERSION {
            return None;
        }
        snapshot
            .habits
            .into_iter()
            .map(Self::decode_habit)
            .collect()
    }

    fn encode_habit(habit: &Habit) -> HabitRecord {
        let steps = habit
            .step_history()
            .changes()
            .into_iter()
            .map(|change| StepRecord {
                on: change.on().epoch_day(),
                goal: change.goal().value(),
            })
            .collect();
        let completions = habit
            .completion_history()
            .dates()
            .map(|date| date.epoch_day())
            .collect();

        HabitRecord {
            id: habit.id().value().to_string(),
            title: habit.title().value().to_string(),
            state: Self::encode_state(habit.state()),
            steps,
            completions,
        }
    }

    /// Rejects a non-monotone `steps` order (Security F-6): `StepHistory::goal_on`
    /// scans from the end for the last step on-or-before a queried day, which
    /// only answers correctly if the steps are chronologically ordered. A
    /// stored payload could carry them out of order (hand-edited, or an older
    /// writer bug); rejecting it here — rather than silently sorting — keeps
    /// the same all-or-nothing stance already taken for every other
    /// structurally-broken field, instead of rewriting what was stored into
    /// an order it never held.
    fn decode_habit(record: HabitRecord) -> Option<Habit> {
        let id = HabitId::new(&record.id).ok()?;
        let title = HabitTitle::new(record.title).ok()?;

        let mut steps = record.steps.into_iter();
        let first = steps.next()?;
        let first_goal = Goal::new(first.goal).ok()?;
        let first_on = LocalDate::parse_stored(first.on)?;
        let mut rest = Vec::new();
        let mut previous_on = first_on;
        for step in steps {
            let goal = Goal::new(step.goal).ok()?;
            let on = LocalDate::parse_stored(step.on)?;
            if on <= previous_on {
                return None;
            }
            previous_on = on;
            rest.push((on, goal));
        }
        let step_history = StepHistory::rehydrate(first_on, first_goal, rest);

        let mut completion_dates = Vec::with_capacity(record.completions.len());
        for day in record.completions {
            completion_dates.push(LocalDate::parse_stored(day)?);
        }
        let completion_history = CompletionHistory::rehydrate(completion_dates);

        Some(Habit::rehydrate(
            id,
            title,
            step_history,
            completion_history,
            Self::decode_state(record.state),
        ))
    }

    fn encode_state(state: LifecycleState) -> StateRecord {
        match state {
            LifecycleState::Active => StateRecord::Active,
            LifecycleState::Paused => StateRecord::Paused,
            LifecycleState::Anchored => StateRecord::Anchored,
        }
    }

    fn decode_state(state: StateRecord) -> LifecycleState {
        match state {
            StateRecord::Active => LifecycleState::Active,
            StateRecord::Paused => LifecycleState::Paused,
            StateRecord::Anchored => LifecycleState::Anchored,
        }
    }
}
