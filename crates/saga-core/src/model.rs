use chrono::{DateTime, Days, Local, Months, NaiveDate, NaiveTime, TimeZone};
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

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Priority {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum Recurrence {
    Daily,
    Weekly,
    Monthly,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlainData {
    pub markdown: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeditationData {
    pub markdown: Option<String>,
    pub duration_minutes: u32,
    pub mood_before: Option<u8>,
    pub mood_after: Option<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskData {
    pub description: Option<String>,
    pub due_date: Option<NaiveDate>,
    pub due_time: Option<NaiveTime>,
    pub completed: bool,
    pub completed_at: Option<DateTime<Local>>,
    pub priority: Priority,
    pub checklist: Vec<ChecklistItem>,
    pub estimated_minutes: Option<u32>,
    pub recurrence: Option<Recurrence>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkoutData {
    pub template_id: Option<Uuid>,
    pub exercises: Vec<PerformedExercise>,
    pub duration_minutes: Option<u32>,
    pub notes: Option<String>,
    pub perceived_effort: Option<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum EchoContent {
    PlainEcho(PlainData),
    MeditationEcho(MeditationData),
    TaskEcho(TaskData),
    WorkoutEcho(WorkoutData),
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
            EchoContent::PlainEcho(_) => "Echo",
            EchoContent::MeditationEcho(_) => "Meditation Echo",
            EchoContent::TaskEcho(_) => "Task Echo",
            EchoContent::WorkoutEcho(_) => "Workout Echo",
        }
    }

    pub fn new_task(day: NaiveDate, section_id: Uuid, title: String) -> Self {
        Echo::new(day, section_id, title, EchoContent::TaskEcho(TaskData::new()))
    }

    pub fn as_task(&self) -> Option<&TaskData> {
        match &self.content {
            EchoContent::TaskEcho(data) => Some(data),
            _ => None,
        }
    }

    pub fn as_task_mut(&mut self) -> Option<&mut TaskData> {
        match &mut self.content {
            EchoContent::TaskEcho(data) => Some(data),
            _ => None,
        }
    }

    pub fn spawn_next_occurrence(&self) -> Option<Echo> {
        let task = self.as_task()?;
        let next_date = task.next_due_date()?;

        let mut next_task = task.clone();
        next_task.completed = false;
        next_task.completed_at = None;
        next_task.due_date = Some(next_date);
        for item in next_task.checklist.iter_mut() {
            item.done = false;
        }

        let mut next_echo = Echo::new(
            next_date,
            self.section_id,
            self.title.clone(),
            EchoContent::TaskEcho(next_task),
        );
        next_echo.tags = self.tags.clone();

        Some(next_echo)
    }
}

impl TaskData {
    pub fn new() -> Self {
        Self {
            description: None,
            due_date: None,
            due_time: None,
            completed: false,
            completed_at: None,
            priority: Priority::Medium,
            checklist: Vec::new(),
            estimated_minutes: None,
            recurrence: None,
        }
    }

    pub fn complete(&mut self) {
        self.completed = true;
        self.completed_at = Some(Local::now());
    }

    pub fn uncomplete(&mut self) {
        self.completed = false;
        self.completed_at = None;
    }

    pub fn is_complete(&self) -> bool {
        self.completed
    }

    pub fn add_item(&mut self, text: String) {
        self.checklist.push(ChecklistItem {
            text,
            done: false,
        });
        self.recompute_completion();
    }

    pub fn remove_item(&mut self, index: usize) {
        if index < self.checklist.len() {
            self.checklist.remove(index);
            self.recompute_completion();
        }
    }

    pub fn edit_item(&mut self, index: usize, text: String) {
        if let Some(item) = self.checklist.get_mut(index) {
            item.text = text;
        }
    }

    pub fn toggle_item(&mut self, index: usize) {
        if let Some(item) = self.checklist.get_mut(index) {
            item.done = !item.done;
            self.recompute_completion();
        }
    }

    pub fn set_item_done(&mut self, index: usize, done: bool) {
        if let Some(item) = self.checklist.get_mut(index) {
            item.done = done;
            self.recompute_completion();
        }
    }

    pub fn progress(&self) -> (usize, usize) {
        let done = self.checklist.iter().filter(|item| item.done).count();
        (done, self.checklist.len())
    }

    pub fn all_items_done(&self) -> bool {
        !self.checklist.is_empty() && self.checklist.iter().all(|item| item.done)
    }

    pub fn is_list(&self) -> bool {
        !self.checklist.is_empty()
    }

    pub fn clear_checklist(&mut self) {
        self.checklist.clear();
    }

    pub fn set_priority(&mut self, priority: Priority) {
        self.priority = priority;
    }

    pub fn set_due(&mut self, date: Option<NaiveDate>, time: Option<NaiveTime>) {
        self.due_date = date;
        self.due_time = time;
    }

    pub fn clear_due(&mut self) {
        self.due_date = None;
        self.due_time = None;
    }

    pub fn due_datetime(&self) -> Option<DateTime<Local>> {
        let date = self.due_date?;
        let time = self
            .due_time
            .unwrap_or_else(|| NaiveTime::from_hms_opt(0, 0, 0).unwrap());
        Local.from_local_datetime(&date.and_time(time)).single()
    }

    pub fn is_overdue(&self, now: DateTime<Local>) -> bool {
        if self.completed {
            return false;
        }
        let Some(due_date) = self.due_date else {
            return false;
        };
        match self.due_time {
            Some(_) => self.due_datetime().map_or(false, |due| due < now),
            None => due_date < now.date_naive(),
        }
    }

    pub fn next_due_date(&self) -> Option<NaiveDate> {
        let date = self.due_date?;
        Some(match self.recurrence? {
            Recurrence::Daily => date + Days::new(1),
            Recurrence::Weekly => date + Days::new(7),
            Recurrence::Monthly => date.checked_add_months(Months::new(1))?,
        })
    }

    pub fn set_estimated_minutes(&mut self, minutes: Option<u32>) {
        self.estimated_minutes = minutes;
    }

    fn recompute_completion(&mut self) {
        if self.checklist.is_empty() {
            return;
        }
        if self.checklist.iter().all(|item| item.done) {
            if !self.completed {
                self.completed = true;
                self.completed_at = Some(Local::now());
            }
        } else if self.completed {
            self.completed = false;
            self.completed_at = None;
        }
    }
}

impl Default for TaskData {
    fn default() -> Self {
        Self::new()
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
    use chrono::{Local, NaiveDate, NaiveTime};
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
    fn test_meditation_echo() {
        let echo = Echo::new(
            Local::now().date_naive(),
            Uuid::new_v4(),
            "Morning Sit".to_string(),
            EchoContent::MeditationEcho(MeditationData {
                markdown: Some("Felt calm.".to_string()),
                duration_minutes: 20,
                mood_before: Some(5),
                mood_after: Some(8),
            }),
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
            EchoContent::TaskEcho(TaskData {
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
            }),
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
            EchoContent::WorkoutEcho(WorkoutData {
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
            }),
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

    #[test]
    fn test_task_new_defaults() {
        let task = TaskData::new();
        assert!(!task.completed);
        assert!(task.completed_at.is_none());
        assert!(task.checklist.is_empty());
        assert!(!task.is_list());
        assert_eq!(task.priority, Priority::Medium);
    }

    #[test]
    fn test_complete_uncomplete() {
        let mut task = TaskData::new();
        task.complete();
        assert!(task.is_complete());
        assert!(task.completed_at.is_some());

        task.uncomplete();
        assert!(!task.is_complete());
        assert!(task.completed_at.is_none());
    }

    #[test]
    fn test_checklist_autocomplete() {
        let mut task = TaskData::new();
        task.add_item("Milk".to_string());
        task.add_item("Eggs".to_string());

        assert!(task.is_list());
        assert_eq!(task.progress(), (0, 2));
        assert!(!task.completed);

        task.toggle_item(0);
        assert_eq!(task.progress(), (1, 2));
        assert!(!task.completed);

        task.toggle_item(1);
        assert_eq!(task.progress(), (2, 2));
        assert!(task.completed);
        assert!(task.completed_at.is_some());

        task.toggle_item(1);
        assert!(!task.completed);
        assert!(task.completed_at.is_none());
    }

    #[test]
    fn test_empty_checklist_not_autocompleted() {
        let mut task = TaskData::new();
        assert!(!task.completed);

        task.complete();
        assert!(task.completed);
    }

    #[test]
    fn test_remove_item_triggers_autocomplete() {
        let mut task = TaskData::new();
        task.add_item("A".to_string());
        task.add_item("B".to_string());
        task.set_item_done(0, true);
        assert!(!task.completed);

        task.remove_item(1);
        assert!(task.completed);
    }

    #[test]
    fn test_priority_ordering() {
        assert!(Priority::Low < Priority::Medium);
        assert!(Priority::Medium < Priority::High);
        assert!(Priority::High < Priority::Critical);

        let mut prios = vec![
            Priority::High,
            Priority::Low,
            Priority::Critical,
            Priority::Medium,
        ];
        prios.sort();
        assert_eq!(
            prios,
            vec![
                Priority::Low,
                Priority::Medium,
                Priority::High,
                Priority::Critical,
            ]
        );
    }

    #[test]
    fn test_due_and_overdue() {
        let now = Local::now();
        let today = now.date_naive();

        let mut task = TaskData::new();
        assert!(!task.is_overdue(now));

        task.set_due(Some(today - chrono::Days::new(1)), None);
        assert!(task.is_overdue(now));

        task.set_due(Some(today), None);
        assert!(!task.is_overdue(now));

        task.set_due(Some(today + chrono::Days::new(1)), None);
        assert!(!task.is_overdue(now));

        task.set_due(Some(today - chrono::Days::new(5)), None);
        task.complete();
        assert!(!task.is_overdue(now));
    }

    #[test]
    fn test_due_datetime_combines() {
        let mut task = TaskData::new();
        task.set_due(
            Some(NaiveDate::from_ymd_opt(2026, 6, 1).unwrap()),
            Some(NaiveTime::from_hms_opt(9, 30, 0).unwrap()),
        );
        let dt = task.due_datetime().expect("should combine into a datetime");
        assert_eq!(dt.date_naive(), NaiveDate::from_ymd_opt(2026, 6, 1).unwrap());
    }

    #[test]
    fn test_recurrence_next_due() {
        let mut task = TaskData::new();
        task.set_due(Some(NaiveDate::from_ymd_opt(2026, 1, 31).unwrap()), None);

        task.recurrence = Some(Recurrence::Daily);
        assert_eq!(
            task.next_due_date(),
            Some(NaiveDate::from_ymd_opt(2026, 2, 1).unwrap())
        );

        task.recurrence = Some(Recurrence::Weekly);
        assert_eq!(
            task.next_due_date(),
            Some(NaiveDate::from_ymd_opt(2026, 2, 7).unwrap())
        );

        task.recurrence = Some(Recurrence::Monthly);
        assert_eq!(
            task.next_due_date(),
            Some(NaiveDate::from_ymd_opt(2026, 2, 28).unwrap())
        );
    }

    #[test]
    fn test_next_due_requires_due_and_recurrence() {
        let mut task = TaskData::new();
        task.recurrence = Some(Recurrence::Daily);
        assert_eq!(task.next_due_date(), None);

        let mut task2 = TaskData::new();
        task2.set_due(Some(NaiveDate::from_ymd_opt(2026, 1, 1).unwrap()), None);
        assert_eq!(task2.next_due_date(), None);
    }

    #[test]
    fn test_spawn_next_occurrence() {
        let mut echo = Echo::new_task(
            NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
            Uuid::new_v4(),
            "Weekly groceries".to_string(),
        );
        {
            let task = echo.as_task_mut().unwrap();
            task.set_due(Some(NaiveDate::from_ymd_opt(2026, 1, 1).unwrap()), None);
            task.recurrence = Some(Recurrence::Weekly);
            task.add_item("Milk".to_string());
            task.set_item_done(0, true);
        }
        assert!(echo.as_task().unwrap().completed);

        let next = echo.spawn_next_occurrence().expect("should spawn next");
        assert_ne!(next.id, echo.id);

        let next_task = next.as_task().unwrap();
        assert_eq!(
            next_task.due_date,
            Some(NaiveDate::from_ymd_opt(2026, 1, 8).unwrap())
        );
        assert!(!next_task.completed);
        assert!(next_task.completed_at.is_none());
        assert!(!next_task.checklist[0].done);
        assert_eq!(next.day, NaiveDate::from_ymd_opt(2026, 1, 8).unwrap());
    }

    #[test]
    fn test_as_task_on_non_task() {
        let echo = Echo::new(
            Local::now().date_naive(),
            Uuid::new_v4(),
            "Plain".to_string(),
            EchoContent::PlainEcho(PlainData {
                markdown: "hi".to_string(),
            }),
        );
        assert!(echo.as_task().is_none());
    }
}
