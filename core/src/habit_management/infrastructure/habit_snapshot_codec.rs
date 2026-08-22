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

/// Read only far enough to decide whether the rest of the payload is worth
/// parsing at all — an unknown version is refused before a single
/// `HabitRecord` is touched, matching "takes the same path, without being
/// parsed" for the versioning behaviour.
#[derive(Deserialize)]
struct VersionProbe {
    v: u32,
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
/// habit.
pub struct HabitSnapshotCodec;

impl HabitSnapshotCodec {
    const VERSION: u32 = 1;

    pub fn encode(habits: &[Habit]) -> String {
        let snapshot = SnapshotV1 {
            v: Self::VERSION,
            habits: habits.iter().map(Self::encode_habit).collect(),
        };
        serde_json::to_string(&snapshot).expect(
            "a snapshot built entirely from already-validated domain values always serializes",
        )
    }

    /// `None` covers every unreadable shape alike: invalid JSON, a missing
    /// field, an unknown `v`, or a stored value that no longer parses through
    /// its VO constructor — the caller cannot tell which, and does not need
    /// to; every case reads as "start from an empty board".
    pub fn decode(payload: &str) -> Option<Vec<Habit>> {
        let probe: VersionProbe = serde_json::from_str(payload).ok()?;
        if probe.v != Self::VERSION {
            return None;
        }
        let snapshot: SnapshotV1 = serde_json::from_str(payload).ok()?;
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

    fn decode_habit(record: HabitRecord) -> Option<Habit> {
        let id = HabitId::new(&record.id).ok()?;
        let title = HabitTitle::new(record.title).ok()?;

        let mut steps = record.steps.into_iter();
        let first = steps.next()?;
        let first_goal = Goal::new(first.goal).ok()?;
        let mut rest = Vec::new();
        for step in steps {
            let goal = Goal::new(step.goal).ok()?;
            rest.push((LocalDate::from_epoch_day(step.on), goal));
        }
        let step_history =
            StepHistory::rehydrate(LocalDate::from_epoch_day(first.on), first_goal, rest);

        let completion_history = CompletionHistory::rehydrate(
            record
                .completions
                .into_iter()
                .map(LocalDate::from_epoch_day),
        );

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
