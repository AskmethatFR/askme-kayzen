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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_rejects_a_value_below_the_floor() {
        let result = Goal::new(0);

        assert_eq!(result, Err(HabitError::GoalTooSmall { min: Goal::MIN }));
    }

    #[test]
    fn new_accepts_the_floor_and_values_with_no_upper_ceiling() {
        let cases = vec![1, 5, 6, 100];

        for value in cases {
            let result = Goal::new(value).map(|goal| goal.value());

            assert_eq!(result, Ok(value));
        }
    }
}
