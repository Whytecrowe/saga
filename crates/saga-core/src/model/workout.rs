use super::*;
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkoutData {
    pub template_id: Option<Uuid>,
    pub exercises: Vec<PerformedExercise>,
    pub duration_minutes: Option<u32>,
    pub notes: Option<String>,
    pub perceived_effort: Option<u8>,
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

impl Echo {
    pub fn new_workout(day: NaiveDate, title: String) -> Self {
        Echo::new(day, title, EchoContent::WorkoutEcho(WorkoutData::new()))
    }

    pub fn new_workout_from_template(
        day: NaiveDate,
        title: String,
        template: &WorkoutTemplate,
        history: &[Echo],
    ) -> Self {
        Echo::new(
            day,
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

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Local, NaiveDate};

    #[test]
    fn test_workout_echo() {
        let echo = Echo::new(
            Local::now().date_naive(),
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
        let mut last = Echo::new_workout(
            NaiveDate::from_ymd_opt(2026, 6, 1).unwrap(),
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
            "Plain".to_string(),
            EchoContent::PlainEcho(PlainData {
                markdown: "hi".to_string(),
            }),
        );
        assert!(echo.as_workout().is_none());
    }
}
