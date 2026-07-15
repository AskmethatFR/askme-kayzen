pub trait GuidGenerator {
    fn generate(&self) -> String;
}

pub struct UuidGenerator;

impl GuidGenerator for UuidGenerator {
    fn generate(&self) -> String {
        uuid::Uuid::new_v4().to_string()
    }
}
