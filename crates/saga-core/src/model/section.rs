use std::fmt::{self, Formatter};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct Section {
    pub id: Uuid,
    pub name: String,
    pub sort_order: i32,
}

#[derive(Debug, Clone)]
pub struct Template {
    pub id: Uuid,
    pub name: String,
    pub markdown_seed: String,
    pub section_id: Option<Uuid>,
}

impl Section {
    pub fn new(name: String, sort_order: i32) -> Self {
        Self {
            id: Uuid::new_v4(),
            name,
            sort_order,
        }
    }

    pub fn rename(&mut self, new_name: String) {
        self.name = new_name;
    }
}

impl fmt::Display for Section {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}\n  ID: {}\n  Sort Order: {}",
            self.name, self.id, self.sort_order,
        )
    }
}
