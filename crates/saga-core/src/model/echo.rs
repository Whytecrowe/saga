use super::*;
use chrono::{DateTime, Local, NaiveDate};
use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum EchoContent {
    PlainEcho(PlainData),
    MeditationEcho(MeditationData),
    TaskEcho(TaskData),
    WorkoutEcho(WorkoutData),
}

#[derive(Debug, Clone)]
pub struct Echo {
    pub id: Uuid,
    pub day: NaiveDate,
    pub section_id: Uuid,
    pub title: String,
    pub content: EchoContent,
    pub mood: Option<u8>,
    pub energy: Option<u8>,
    pub pinned: bool,
    pub tags: Vec<String>,
    pub linked_echo_id: Option<Uuid>,
    pub created_at: DateTime<Local>,
    pub updated_at: DateTime<Local>,
}

impl Echo {
    pub fn new(day: NaiveDate, section_id: Uuid, title: String, content: EchoContent) -> Self {
        let now = Local::now();

        Self {
            id: Uuid::new_v4(),
            day,
            section_id,
            title,
            content,
            mood: None,
            energy: None,
            pinned: false,
            tags: Vec::new(),
            linked_echo_id: None,
            created_at: now,
            updated_at: now,
        }
    }

    pub fn char_count(&self) -> usize {
        match &self.content {
            EchoContent::PlainEcho(data) => data.markdown.len(),
            EchoContent::MeditationEcho(data) => data.markdown.as_deref().unwrap_or("").len(),
            EchoContent::TaskEcho(data) => data.description.as_deref().unwrap_or("").len(),
            EchoContent::WorkoutEcho(data) => data.notes.as_deref().unwrap_or("").len(),
        }
    }

    pub fn update_content(&mut self, new_content: EchoContent) {
        self.content = new_content;
        self.updated_at = Local::now();
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

    pub fn content_type_name(&self) -> &str {
        match &self.content {
            EchoContent::PlainEcho(_) => ECHO_TYPE_PLAIN,
            EchoContent::MeditationEcho(_) => ECHO_TYPE_MEDITATION,
            EchoContent::TaskEcho(_) => ECHO_TYPE_TASK,
            EchoContent::WorkoutEcho(_) => ECHO_TYPE_WORKOUT,
        }
    }
}

impl fmt::Display for Echo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} {}\n  Day: {}\n  Section: {}\n  Created: {}",
            self.content_type_name(),
            self.id,
            self.day,
            self.section_id,
            self.created_at.format("%b %e, %Y at %l:%M %p"),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Local;
    use uuid::Uuid;

    #[test]
    fn test_plain_echo() {
        let echo = Echo::new(
            Local::now().date_naive(),
            Uuid::new_v4(),
            "First Echo".to_string(),
            EchoContent::PlainEcho(PlainData {
                markdown: "Hello World!".to_string(),
            }),
        );

        assert_eq!(echo.content_type_name(), "Echo");
        assert_eq!(echo.char_count(), 12);
        assert!(!echo.pinned);
        assert!(echo.tags.is_empty());
        assert!(echo.mood.is_none());
        assert!(echo.energy.is_none());
        assert!(echo.linked_echo_id.is_none());
        assert!(!echo.was_modified());

        println!("{}", echo);
    }

    #[test]
    fn test_update_content() {
        let mut echo = Echo::new(
            Local::now().date_naive(),
            Uuid::new_v4(),
            "My Echo".to_string(),
            EchoContent::PlainEcho(PlainData {
                markdown: "Original".to_string(),
            }),
        );

        echo.update_content(EchoContent::PlainEcho(PlainData {
            markdown: "Updated".to_string(),
        }));
        assert!(echo.was_modified());
    }

    #[test]
    fn test_echo_content_serde() {
        let content = EchoContent::MeditationEcho(MeditationData {
            markdown: Some("Peaceful.".to_string()),
            duration_minutes: 15,
            mood_before: Some(4),
            mood_after: Some(9),
        });

        let json = serde_json::to_string(&content).expect("serialize failed");
        println!("Serialized: {}", json);

        let restored: EchoContent = serde_json::from_str(&json).expect("deserialize failed");

        match restored {
            EchoContent::MeditationEcho(data) => {
                assert_eq!(data.duration_minutes, 15);
            }
            _ => panic!("Wrong variant after deserialization"),
        }
    }
}
