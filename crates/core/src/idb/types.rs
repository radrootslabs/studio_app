#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RadrootsClientIdbConfig {
    pub database: &'static str,
    pub store: &'static str,
}

impl RadrootsClientIdbConfig {
    pub const fn new(database: &'static str, store: &'static str) -> Self {
        Self { database, store }
    }
}
