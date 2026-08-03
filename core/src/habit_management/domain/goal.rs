use crate::habit_management::domain::habit::HabitError;

#[derive(Debug, Clone, PartialEq)]
pub struct Goal(u32);

impl Goal {
    pub const MIN: u32 = 1;

    pub fn new(value: u32) -> Result<Goal, HabitError> {
        if value < Self::MIN {
            return Err(HabitError::GoalTooSmall { min: Self::MIN });
        }

        Ok(Goal(value))
    }

    pub fn value(&self) -> u32 {
        self.0
    }

    pub fn grown(&self) -> Goal {
        Goal(self.0.saturating_add(1))
    }

    pub fn lightened(&self) -> Goal {
        Goal(self.0.saturating_sub(1).max(Self::MIN))
    }
}
