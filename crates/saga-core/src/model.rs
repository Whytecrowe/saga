use chrono::{DateTime, Days, Local, Months, NaiveDate, NaiveTime, TimeZone};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
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
    pub weight: Option<f32>,
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
pub struct PlannedSet {
    pub target_reps: Option<u32>,
    pub target_weight: Option<f32>,
    pub target_rest_seconds: Option<u32>,
    pub is_warmup: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlannedExercise {
    pub name: String,
    pub sets: Vec<PlannedSet>,
    pub notes: Option<String>,
    pub superset_group: Option<u8>,
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
pub struct WorkoutProgram {
    pub id: Uuid,
    pub name: String,
    pub notes: Option<String>,
}

#[derive(Debug, Clone)]
pub struct WorkoutTemplate {
    pub id: Uuid,
    pub name: String,
    pub program_id: Option<Uuid>,
    pub sort_order: i32,
    pub exercises: Vec<PlannedExercise>,
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

    pub fn new_workout(day: NaiveDate, section_id: Uuid, title: String) -> Self {
        Echo::new(
            day,
            section_id,
            title,
            EchoContent::WorkoutEcho(WorkoutData::new()),
        )
    }

    pub fn new_workout_from_template(
        day: NaiveDate,
        section_id: Uuid,
        title: String,
        template: &WorkoutTemplate,
        history: &[Echo],
    ) -> Self {
        Echo::new(
            day,
            section_id,
            title,
            EchoContent::WorkoutEcho(WorkoutData::from_template(template, history)),
        )
    }

    pub fn as_workout(&self) -> Option<&WorkoutData> {
        match &self.content {
            EchoContent::WorkoutEcho(data) => Some(data),
            _ => None,
        }
    }

    pub fn as_workout_mut(&mut self) -> Option<&mut WorkoutData> {
        match &mut self.content {
            EchoContent::WorkoutEcho(data) => Some(data),
            _ => None,
        }
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

impl PlannedExercise {
    pub fn new(name: String) -> Self {
        Self {
            name,
            sets: Vec::new(),
            notes: None,
            superset_group: None,
        }
    }

    pub fn add_set(&mut self, set: PlannedSet) {
        self.sets.push(set);
    }

    pub fn remove_set(&mut self, index: usize) {
        if index < self.sets.len() {
            self.sets.remove(index);
        }
    }
}

impl WorkoutProgram {
    pub fn new(name: String, notes: Option<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            name,
            notes,
        }
    }
}

impl WorkoutTemplate {
    pub fn new(name: String, program_id: Option<Uuid>, sort_order: i32) -> Self {
        Self {
            id: Uuid::new_v4(),
            name,
            program_id,
            sort_order,
            exercises: Vec::new(),
        }
    }

    pub fn add_exercise(&mut self, exercise: PlannedExercise) {
        self.exercises.push(exercise);
    }

    pub fn remove_exercise(&mut self, index: usize) {
        if index < self.exercises.len() {
            self.exercises.remove(index);
        }
    }

    pub fn move_exercise(&mut self, from: usize, to: usize) {
        if from < self.exercises.len() && to < self.exercises.len() {
            let item = self.exercises.remove(from);
            self.exercises.insert(to, item);
        }
    }
}

fn last_performed_by_name<'a>(
    history: &'a [Echo],
) -> HashMap<&'a str, &'a PerformedExercise> {
    let mut map: HashMap<&'a str, &'a PerformedExercise> = HashMap::new();

    for echo in history {
        if let Some(workout) = echo.as_workout() {
            for exercise in &workout.exercises {
                map.entry(exercise.name.as_str()).or_insert(exercise);
            }
        }
    }

    map
}

impl WorkoutData {
    pub fn new() -> Self {
        Self {
            template_id: None,
            exercises: Vec::new(),
            duration_minutes: None,
            notes: None,
            perceived_effort: None,
        }
    }

    pub fn from_template(template: &WorkoutTemplate, history: &[Echo]) -> Self {
        let last_by_name = last_performed_by_name(history);

        let exercises = template
            .exercises
            .iter()
            .map(|planned| {
                let previous = last_by_name.get(planned.name.as_str());

                let sets = planned
                    .sets
                    .iter()
                    .enumerate()
                    .map(|(index, planned_set)| {
                        let previous_set =
                            previous.and_then(|exercise| exercise.sets.get(index));

                        SetEntry {
                            reps: previous_set
                                .map(|set| set.reps)
                                .or(planned_set.target_reps)
                                .unwrap_or(0),
                            weight: previous_set
                                .and_then(|set| set.weight)
                                .or(planned_set.target_weight),
                            completed: false,
                            rest_seconds: previous_set
                                .and_then(|set| set.rest_seconds)
                                .or(planned_set.target_rest_seconds),
                            is_warmup: planned_set.is_warmup,
                        }
                    })
                    .collect();

                PerformedExercise {
                    name: planned.name.clone(),
                    sets,
                }
            })
            .collect();

        Self {
            template_id: Some(template.id),
            exercises,
            duration_minutes: None,
            notes: None,
            perceived_effort: None,
        }
    }

    pub fn add_exercise(&mut self, name: String) {
        self.exercises.push(PerformedExercise {
            name,
            sets: Vec::new(),
        });
    }

    pub fn remove_exercise(&mut self, index: usize) {
        if index < self.exercises.len() {
            self.exercises.remove(index);
        }
    }

    pub fn log_set(&mut self, exercise_index: usize, set: SetEntry) {
        if let Some(exercise) = self.exercises.get_mut(exercise_index) {
            exercise.sets.push(set);
        }
    }

    pub fn is_freeform(&self) -> bool {
        self.template_id.is_none()
    }

    pub fn working_sets(&self) -> impl Iterator<Item = &SetEntry> {
        self.exercises
            .iter()
            .flat_map(|exercise| exercise.sets.iter())
            .filter(|set| set.completed && !set.is_warmup)
    }

    pub fn total_volume(&self) -> f32 {
        self.working_sets()
            .map(|set| set.reps as f32 * set.weight.unwrap_or(0.0))
            .sum()
    }

    pub fn total_sets(&self) -> usize {
        self.exercises
            .iter()
            .map(|exercise| exercise.sets.len())
            .sum()
    }

    pub fn completed_sets(&self) -> usize {
        self.exercises
            .iter()
            .flat_map(|exercise| exercise.sets.iter())
            .filter(|set| set.completed)
            .count()
    }
}

impl Default for WorkoutData {
    fn default() -> Self {
        Self::new()
    }
}

pub fn open_tasks(echoes: &[Echo]) -> Vec<&Echo> {
    echoes
        .iter()
        .filter(|echo| echo.as_task().is_some_and(|task| !task.completed))
        .collect()
}

pub fn overdue_tasks(echoes: &[Echo], now: DateTime<Local>) -> Vec<&Echo> {
    echoes
        .iter()
        .filter(|echo| echo.as_task().is_some_and(|task| task.is_overdue(now)))
        .collect()
}

pub fn tasks_by_priority(echoes: &[Echo]) -> Vec<&Echo> {
    let mut tasks: Vec<(&Echo, &Priority)> = echoes
        .iter()
        .filter_map(|echo| echo.as_task().map(|task| (echo, &task.priority)))
        .collect();
    tasks.sort_by(|a, b| b.1.cmp(a.1));
    tasks.into_iter().map(|(echo, _)| echo).collect()
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
                        weight: Some(80.0),
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
    fn test_workout_program_new() {
        let program = WorkoutProgram::new(
            "Push/Pull/Legs".to_string(),
            Some("6-day split".to_string()),
        );
        assert_eq!(program.name, "Push/Pull/Legs");
        assert_eq!(program.notes.as_deref(), Some("6-day split"));
    }

    #[test]
    fn test_planned_exercise_build() {
        let mut bench = PlannedExercise::new("Bench Press".to_string());
        assert!(bench.sets.is_empty());

        bench.add_set(PlannedSet {
            target_reps: Some(8),
            target_weight: Some(60.0),
            target_rest_seconds: Some(120),
            is_warmup: false,
        });
        bench.add_set(PlannedSet {
            target_reps: Some(8),
            target_weight: Some(60.0),
            target_rest_seconds: Some(120),
            is_warmup: false,
        });
        assert_eq!(bench.sets.len(), 2);

        bench.remove_set(5);
        assert_eq!(bench.sets.len(), 2);

        bench.remove_set(0);
        assert_eq!(bench.sets.len(), 1);
    }

    #[test]
    fn test_workout_template_build() {
        let mut template = WorkoutTemplate::new("Push Day".to_string(), None, 0);
        assert!(template.exercises.is_empty());
        assert!(template.program_id.is_none());

        template.add_exercise(PlannedExercise::new("Bench".to_string()));
        template.add_exercise(PlannedExercise::new("Overhead Press".to_string()));
        template.add_exercise(PlannedExercise::new("Dips".to_string()));
        assert_eq!(template.exercises.len(), 3);

        template.remove_exercise(10);
        assert_eq!(template.exercises.len(), 3);

        template.move_exercise(0, 2);
        assert_eq!(template.exercises[0].name, "Overhead Press");
        assert_eq!(template.exercises[2].name, "Bench");

        template.remove_exercise(1);
        assert_eq!(template.exercises.len(), 2);
    }

    fn push_day_template() -> WorkoutTemplate {
        let mut bench = PlannedExercise::new("Bench".to_string());
        for _ in 0..3 {
            bench.add_set(PlannedSet {
                target_reps: Some(8),
                target_weight: Some(60.0),
                target_rest_seconds: Some(120),
                is_warmup: false,
            });
        }

        let mut template = WorkoutTemplate::new("Push Day".to_string(), None, 0);
        template.add_exercise(bench);
        template
    }

    #[test]
    fn test_workout_freeform_new() {
        let echo = Echo::new_workout(
            NaiveDate::from_ymd_opt(2026, 6, 3).unwrap(),
            Uuid::new_v4(),
            "Freeform lift".to_string(),
        );
        let workout = echo.as_workout().unwrap();
        assert!(workout.is_freeform());
        assert!(workout.exercises.is_empty());
        assert!(workout.template_id.is_none());
    }

    #[test]
    fn test_from_template_empty_history_uses_plan() {
        let template = push_day_template();
        let workout = WorkoutData::from_template(&template, &[]);

        assert_eq!(workout.template_id, Some(template.id));
        assert!(!workout.is_freeform());
        assert_eq!(workout.exercises.len(), 1);

        let bench = &workout.exercises[0];
        assert_eq!(bench.name, "Bench");
        assert_eq!(bench.sets.len(), 3);
        for set in &bench.sets {
            assert_eq!(set.weight, Some(60.0));
            assert_eq!(set.reps, 8);
            assert_eq!(set.rest_seconds, Some(120));
            assert!(!set.completed);
        }
    }

    #[test]
    fn test_from_template_no_plan_weight_is_blank() {
        let mut squat = PlannedExercise::new("Squat".to_string());
        squat.add_set(PlannedSet {
            target_reps: None,
            target_weight: None,
            target_rest_seconds: None,
            is_warmup: false,
        });
        let mut template = WorkoutTemplate::new("Leg Day".to_string(), None, 0);
        template.add_exercise(squat);

        let workout = WorkoutData::from_template(&template, &[]);
        let set = &workout.exercises[0].sets[0];
        assert_eq!(set.weight, None);
        assert_eq!(set.reps, 0);
        assert_eq!(set.rest_seconds, None);
    }

    #[test]
    fn test_from_template_uses_history_ramp() {
        let section = Uuid::new_v4();
        let mut last = Echo::new_workout(
            NaiveDate::from_ymd_opt(2026, 6, 1).unwrap(),
            section,
            "Last Push".to_string(),
        );
        {
            let workout = last.as_workout_mut().unwrap();
            workout.add_exercise("Bench".to_string());
            workout.log_set(
                0,
                SetEntry {
                    reps: 5,
                    weight: Some(60.0),
                    completed: true,
                    rest_seconds: Some(90),
                    is_warmup: true,
                },
            );
            workout.log_set(
                0,
                SetEntry {
                    reps: 5,
                    weight: Some(100.0),
                    completed: true,
                    rest_seconds: Some(120),
                    is_warmup: false,
                },
            );
            workout.log_set(
                0,
                SetEntry {
                    reps: 5,
                    weight: Some(140.0),
                    completed: true,
                    rest_seconds: Some(180),
                    is_warmup: false,
                },
            );
        }

        let history = vec![last];
        let template = push_day_template();
        let workout = WorkoutData::from_template(&template, &history);

        let bench = &workout.exercises[0];
        assert_eq!(bench.sets[0].weight, Some(60.0));
        assert_eq!(bench.sets[1].weight, Some(100.0));
        assert_eq!(bench.sets[2].weight, Some(140.0));
        assert_eq!(bench.sets[1].reps, 5);
    }

    #[test]
    fn test_from_template_history_shorter_than_plan() {
        let mut last = Echo::new_workout(
            NaiveDate::from_ymd_opt(2026, 6, 1).unwrap(),
            Uuid::new_v4(),
            "Last".to_string(),
        );
        {
            let workout = last.as_workout_mut().unwrap();
            workout.add_exercise("Bench".to_string());
            workout.log_set(
                0,
                SetEntry {
                    reps: 3,
                    weight: Some(100.0),
                    completed: true,
                    rest_seconds: None,
                    is_warmup: false,
                },
            );
        }

        let history = vec![last];
        let template = push_day_template();
        let workout = WorkoutData::from_template(&template, &history);

        let bench = &workout.exercises[0];
        assert_eq!(bench.sets[0].weight, Some(100.0));
        assert_eq!(bench.sets[1].weight, Some(60.0));
        assert_eq!(bench.sets[2].weight, Some(60.0));
    }

    #[test]
    fn test_from_template_independence() {
        let template = push_day_template();
        let mut workout = WorkoutData::from_template(&template, &[]);

        workout.add_exercise("Bonus Curls".to_string());
        workout.exercises[0].sets[0].weight = Some(999.0);

        assert_eq!(template.exercises.len(), 1);
        assert_eq!(template.exercises[0].sets.len(), 3);
        assert_eq!(template.exercises[0].sets[0].target_weight, Some(60.0));
    }

    #[test]
    fn test_total_volume_excludes_warmup_and_uncompleted() {
        let mut workout = WorkoutData::new();
        workout.add_exercise("Bench".to_string());
        workout.log_set(
            0,
            SetEntry {
                reps: 10,
                weight: Some(40.0),
                completed: true,
                rest_seconds: None,
                is_warmup: true,
            },
        );
        workout.log_set(
            0,
            SetEntry {
                reps: 8,
                weight: Some(100.0),
                completed: true,
                rest_seconds: None,
                is_warmup: false,
            },
        );
        workout.log_set(
            0,
            SetEntry {
                reps: 8,
                weight: Some(100.0),
                completed: false,
                rest_seconds: None,
                is_warmup: false,
            },
        );

        assert_eq!(workout.total_volume(), 800.0);
        assert_eq!(workout.total_sets(), 3);
        assert_eq!(workout.completed_sets(), 2);
    }

    #[test]
    fn test_as_workout_on_non_workout() {
        let echo = Echo::new(
            Local::now().date_naive(),
            Uuid::new_v4(),
            "Plain".to_string(),
            EchoContent::PlainEcho(PlainData {
                markdown: "hi".to_string(),
            }),
        );
        assert!(echo.as_workout().is_none());
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

    #[test]
    fn test_task_query_helpers() {
        let section = Uuid::new_v4();
        let day = NaiveDate::from_ymd_opt(2026, 6, 1).unwrap();

        let mut low_open = Echo::new_task(day, section, "Low open".to_string());
        low_open.as_task_mut().unwrap().set_priority(Priority::Low);

        let mut critical_open = Echo::new_task(day, section, "Critical open".to_string());
        critical_open
            .as_task_mut()
            .unwrap()
            .set_priority(Priority::Critical);

        let mut done = Echo::new_task(day, section, "Done".to_string());
        done.as_task_mut().unwrap().complete();

        let mut overdue = Echo::new_task(day, section, "Overdue".to_string());
        overdue
            .as_task_mut()
            .unwrap()
            .set_due(Some(NaiveDate::from_ymd_opt(2020, 1, 1).unwrap()), None);

        let plain = Echo::new(
            day,
            section,
            "Note".to_string(),
            EchoContent::PlainEcho(PlainData {
                markdown: "x".to_string(),
            }),
        );

        let all = vec![low_open, critical_open, done, overdue, plain];

        let open = open_tasks(&all);
        assert_eq!(open.len(), 3);

        let now = Local::now();
        let late = overdue_tasks(&all, now);
        assert_eq!(late.len(), 1);
        assert_eq!(late[0].title, "Overdue");

        let ranked = tasks_by_priority(&all);
        assert_eq!(ranked.len(), 4);
        assert_eq!(ranked[0].title, "Critical open");
    }
}
