use chrono::{DateTime, Local, NaiveDate};
use uuid::Uuid;
use std::fmt;
use std::fmt::Formatter;

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

#[derive(Debug, Clone)]
pub struct Echo {
    pub id: Uuid,
    pub day: NaiveDate,
    pub section_id: Uuid,
    pub title: String,
    pub markdown: String,
    pub created_at: DateTime<Local>,
    pub updated_at: DateTime<Local>,
}

impl Echo {
    pub fn new(day: NaiveDate, section_id: Uuid, title: String, markdown: String) -> Self {
        let now = Local::now();

        Self {
            id: Uuid::new_v4(),
            day,
            section_id,
            title,
            markdown,
            created_at: now,
            updated_at: now,
        }
    }

    pub fn char_count(&self) -> usize {
        self.markdown.len()
    }

    pub fn update_markdown(&mut self, new_markdown: String) {
        self.markdown = new_markdown;
        self.updated_at = Local::now();
    }

    // This method takes ownership and the Echo is gone after calling it
    pub fn into_markdown(self) -> String {
        self.markdown
    }

    pub fn display_day(&self) -> String {
        self.day.format("%A, %B %e, %Y").to_string()
    }

    pub fn was_modified(&self) -> bool {
        self.updated_at != self.created_at
    }

    pub fn set_day(&mut self, new_day: NaiveDate) {
        self.day = new_day;
        self.updated_at = Local::now();
    }
}

// TODO: do we need any other features here? what are they? add?
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
            self.name,
            self.id,
            self.sort_order,
        )
    }
}

impl fmt::Display for Echo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Echo {}\n  Day: {}\n  Section: {}\n  Created: {}\n\n{}",
            self.id,
            self.day,
            self.section_id,
            self.created_at.format("%b %e, %Y at %l:%M %p"),
            self.markdown
        )
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Local;
    use uuid::Uuid;

    #[test]
    fn test_echo_base() {
        // create an Echo
        let echo = Echo::new(
            Local::now().date_naive(),
            Uuid::new_v4(),
            "First Echo".to_string(),
            "Hello World!".to_string(),
        );

        println!("Echo created: {:#?}", echo);
        println!("Echo char count: {}", echo.char_count());

        // Test mutable borrow
        let mut echo2 = echo.clone();
        println!("Echo2 text initial: {:?}", echo2.markdown);
        echo2.update_markdown("Second Echo: Updated!".to_string());
        println!("Echo2 new text: {:?}", echo2.markdown);

        // Test consumption with conversion
        let markdown = echo2.into_markdown();
        println!("Echo2 consumed: {:?}", markdown);

        // date display
        println!("Echo display day: {}", echo.display_day());
    }
}