#[derive(Debug, Clone, PartialEq)]
pub struct HabitId(String);

impl HabitId {
    pub fn new(value: String) -> HabitId {
        HabitId(value)
    }

    pub fn value(&self) -> &str {
        &self.0
    }
}
impl From<&str> for HabitId {
    fn from(value: &str) -> HabitId {
        HabitId(value.to_string())
    }
}
