use chrono::{DateTime, Local, NaiveDate, NaiveTime};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::fmt::Formatter;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChecklistItem {
    pub text: String,
    pub done: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetEntry {
    pub reps: u32,
    pub weight_kg: Option<f32>,
    pub completed: bool,
    pub rest_seconds: Option<u32>,
    pub is_warmup: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformedExercise {
    pub name: String,
    pub sets: Vec<SetEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlannedExercise {
    pub name: String,
    pub sets: u32,
    pub target_reps: Option<u32>,
    pub notes: Option<String>,
    pub superset_group: Option<u8>,
    pub is_warmup: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Priority {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Recurrence {
    Daily,
    Weekly,
    Monthly,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum EchoContent {
    PlainEcho {
        markdown: String,
    },
    MeditationEcho {
        markdown: Option<String>,
        duration_minutes: u32,
        mood_before: Option<u8>,
        mood_after: Option<u8>,
    },
    TaskEcho {
        title: String,
        description: Option<String>,
        due_date: Option<NaiveDate>,
        due_time: Option<NaiveTime>,
        completed: bool,
        completed_at: Option<DateTime<Local>>,
        priority: Priority,
        checklist: Vec<ChecklistItem>,
        estimated_minutes: Option<u32>,
        recurrence: Option<Recurrence>,
    },
    WorkoutEcho {
        template_id: Option<Uuid>,
        exercises: Vec<PerformedExercise>,
        duration_minutes: Option<u32>,
        notes: Option<String>,
        perceived_effort: Option<u8>,
    },
}

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
            EchoContent::PlainEcho { markdown } => markdown.len(),
            EchoContent::MeditationEcho { markdown, .. } => markdown.as_deref().unwrap_or("").len(),
            EchoContent::TaskEcho {
                title, description, ..
            } => title.len() + description.as_deref().unwrap_or("").len(),
            EchoContent::WorkoutEcho { notes, .. } => notes.as_deref().unwrap_or("").len(),
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
            EchoContent::PlainEcho { .. } => "Echo",
            EchoContent::MeditationEcho { .. } => "Meditation Echo",
            EchoContent::TaskEcho { .. } => "Task Echo",
            EchoContent::WorkoutEcho { .. } => "Workout Echo",
        }
    }
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
            EchoContent::PlainEcho {
                markdown: "Hello World!".to_string(),
            },
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
    fn test_meditation_echo() {
        let echo = Echo::new(
            Local::now().date_naive(),
            Uuid::new_v4(),
            "Morning Sit".to_string(),
            EchoContent::MeditationEcho {
                markdown: Some("Felt calm.".to_string()),
                duration_minutes: 20,
                mood_before: Some(5),
                mood_after: Some(8),
            },
        );

        assert_eq!(echo.content_type_name(), "Meditation Echo");
        assert_eq!(echo.char_count(), 10);
        println!("{}", echo);
    }

    #[test]
    fn test_task_echo() {
        let echo = Echo::new(
            Local::now().date_naive(),
            Uuid::new_v4(),
            "Buy groceries".to_string(),
            EchoContent::TaskEcho {
                title: "Buy groceries".to_string(),
                description: None,
                due_date: None,
                due_time: None,
                completed: false,
                completed_at: None,
                priority: Priority::Medium,
                checklist: vec![
                    ChecklistItem {
                        text: "Milk".to_string(),
                        done: false,
                    },
                    ChecklistItem {
                        text: "Eggs".to_string(),
                        done: true,
                    },
                ],
                estimated_minutes: Some(30),
                recurrence: None,
            },
        );

        assert_eq!(echo.content_type_name(), "Task Echo");
        println!("{}", echo);
    }

    #[test]
    fn test_workout_echo() {
        let echo = Echo::new(
            Local::now().date_naive(),
            Uuid::new_v4(),
            "Push Day".to_string(),
            EchoContent::WorkoutEcho {
                template_id: None,
                exercises: vec![PerformedExercise {
                    name: "Bench Press".to_string(),
                    sets: vec![SetEntry {
                        reps: 8,
                        weight_kg: Some(80.0),
                        completed: true,
                        rest_seconds: Some(90),
                        is_warmup: false,
                    }],
                }],
                duration_minutes: Some(60),
                notes: Some("Felt strong.".to_string()),
                perceived_effort: Some(7),
            },
        );

        assert_eq!(echo.content_type_name(), "Workout Echo");
        println!("{}", echo);
    }

    #[test]
    fn test_update_content() {
        let mut echo = Echo::new(
            Local::now().date_naive(),
            Uuid::new_v4(),
            "My Echo".to_string(),
            EchoContent::PlainEcho {
                markdown: "Original".to_string(),
            },
        );

        echo.update_content(EchoContent::PlainEcho {
            markdown: "Updated".to_string(),
        });
        assert!(echo.was_modified());
    }

    #[test]
    fn test_echo_content_serde() {
        let content = EchoContent::MeditationEcho {
            markdown: Some("Peaceful.".to_string()),
            duration_minutes: 15,
            mood_before: Some(4),
            mood_after: Some(9),
        };

        let json = serde_json::to_string(&content).expect("serialize failed");
        println!("Serialized: {}", json);

        let restored: EchoContent = serde_json::from_str(&json).expect("deserialize failed");

        match restored {
            EchoContent::MeditationEcho {
                duration_minutes, ..
            } => {
                assert_eq!(duration_minutes, 15);
            }
            _ => panic!("Wrong variant after deserialization"),
        }
    }
}
