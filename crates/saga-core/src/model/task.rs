use super::*;
use chrono::{DateTime, Days, Local, Months, NaiveDate, NaiveTime, TimeZone};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChecklistItem {
    pub text: String,
    pub done: bool,
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

impl Echo {
    pub fn new_task(day: NaiveDate, title: String) -> Self {
        Echo::new(day, title, EchoContent::TaskEcho(TaskData::new()))
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
    }

    pub fn remove_item(&mut self, index: usize) {
        if index < self.checklist.len() {
            self.checklist.remove(index);
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
        }
    }

    pub fn set_item_done(&mut self, index: usize, done: bool) {
        if let Some(item) = self.checklist.get_mut(index) {
            item.done = done;
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
            Some(_) => self.due_datetime().is_some_and(|due| due < now),
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
}

impl Default for TaskData {
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

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Local, NaiveDate, NaiveTime};

    #[test]
    fn test_task_echo() {
        let echo = Echo::new(
            Local::now().date_naive(),
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
    fn test_checklist_independent_of_completion() {
        let mut task = TaskData::new();
        task.add_item("Milk".to_string());
        task.add_item("Eggs".to_string());

        assert!(task.is_list());
        assert_eq!(task.progress(), (0, 2));
        assert!(!task.completed);

        // checking every item must NOT auto-complete the task
        task.toggle_item(0);
        task.toggle_item(1);
        assert_eq!(task.progress(), (2, 2));
        assert!(!task.completed);

        // completion is manual and stays put when items change
        task.complete();
        assert!(task.completed);
        task.toggle_item(0);
        assert_eq!(task.progress(), (1, 2));
        assert!(task.completed, "unchecking an item must not reopen the task");
    }

    #[test]
    fn test_manual_complete_uncomplete() {
        let mut task = TaskData::new();
        assert!(!task.completed);

        task.complete();
        assert!(task.completed);
        assert!(task.completed_at.is_some());

        task.uncomplete();
        assert!(!task.completed);
        assert!(task.completed_at.is_none());
    }

    #[test]
    fn test_complete_with_partial_checklist() {
        let mut task = TaskData::new();
        task.add_item("A".to_string());
        task.add_item("B".to_string());
        task.set_item_done(0, true);

        task.complete();
        assert!(task.completed);
        assert_eq!(task.progress(), (1, 2));

        task.remove_item(1);
        assert!(task.completed);
        assert_eq!(task.progress(), (1, 1));
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
            "Weekly groceries".to_string(),
        );
        {
            let task = echo.as_task_mut().unwrap();
            task.set_due(Some(NaiveDate::from_ymd_opt(2026, 1, 1).unwrap()), None);
            task.recurrence = Some(Recurrence::Weekly);
            task.add_item("Milk".to_string());
            task.set_item_done(0, true);
            task.complete();
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
            "Plain".to_string(),
            EchoContent::PlainEcho(PlainData {
                markdown: "hi".to_string(),
            }),
        );
        assert!(echo.as_task().is_none());
    }

    #[test]
    fn test_task_query_helpers() {
        let day = NaiveDate::from_ymd_opt(2026, 6, 1).unwrap();

        let mut low_open = Echo::new_task(day, "Low open".to_string());
        low_open.as_task_mut().unwrap().set_priority(Priority::Low);

        let mut critical_open = Echo::new_task(day, "Critical open".to_string());
        critical_open
            .as_task_mut()
            .unwrap()
            .set_priority(Priority::Critical);

        let mut done = Echo::new_task(day, "Done".to_string());
        done.as_task_mut().unwrap().complete();

        let mut overdue = Echo::new_task(day, "Overdue".to_string());
        overdue
            .as_task_mut()
            .unwrap()
            .set_due(Some(NaiveDate::from_ymd_opt(2020, 1, 1).unwrap()), None);

        let plain = Echo::new(
            day,
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
