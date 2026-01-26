use chrono::{ DateTime, NaiveDate, Utc };
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

#[derive(Debug, Clone)]
pub struct Echo {
    pub id: Uuid,
    pub day: NaiveDate,
    pub section_id: Uuid,
    pub markdown: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Echo {
    pub fn new(day: NaiveDate, section_id: Uuid, markdown: String) -> Self {
        let now = Utc::now();

        Self {
            id: Uuid::new_v4(),
            day,
            section_id,
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
        self.updated_at = Utc::now();
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
        self.updated_at = Utc::now();
    }
}