use chrono::NaiveDate;
use rusqlite::Connection;
use saga_core::model::{Echo, EchoContent, Section};
use std::path::Path;
use std::str::FromStr;
use thiserror::Error;
use uuid::Uuid;

#[derive(Error, Debug)]
pub enum StorageError {
    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),
    #[error("Echo not found: {0}")]
    EchoNotFound(Uuid),
    #[error("Section not found: {0}")]
    SectionNotFound(Uuid),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}

pub type Result<T> = std::result::Result<T, StorageError>;

pub struct Storage {
    conn: Connection,
}
impl Storage {
    pub fn new<P: AsRef<Path>>(db_path: P) -> Result<Self> {
        let conn = Connection::open(db_path)?;
        conn.execute_batch("PRAGMA journal_mode=WAL;")?;
        let storage = Self { conn };
        storage.init_schema()?;
        Ok(storage)
    }

    fn init_schema(&self) -> Result<()> {
        self.conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS sections (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                sort_order INTEGER NOT NULL
            );

            CREATE TABLE IF NOT EXISTS echoes (
                id TEXT PRIMARY KEY,
                day TEXT NOT NULL,
                section_id TEXT NOT NULL,
                title TEXT NOT NULL,
                content_type TEXT NOT NULL,
                content_json TEXT NOT NULL,
                mood INTEGER,
                energy INTEGER,
                pinned INTEGER NOT NULL DEFAULT 0,
                tags TEXT NOT NULL DEFAULT '[]',
                linked_echo_id TEXT,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                FOREIGN KEY (section_id) REFERENCES sections(id)
            );

            CREATE INDEX IF NOT EXISTS idx_echoes_day ON echoes(day);"
        )?;
        Ok(())
    }
    pub fn save_section(&self, section: &Section) -> Result<()> {
        self.conn.execute(
            "INSERT INTO sections (id, name, sort_order) VALUES (?1, ?2, ?3)",
            rusqlite::params![section.id.to_string(), section.name, section.sort_order],
        )?;
        Ok(())
    }

    pub fn get_section(&self, section_id: &Uuid) -> Result<Option<Section>> {
        let result = self.conn.query_row(
            "SELECT id, name, sort_order FROM sections WHERE id = ?1",
            rusqlite::params![section_id.to_string()],
            |row| {
                Ok(Section {
                    id: parse_from_text(row, 0)?,
                    name: row.get(1)?,
                    sort_order: row.get(2)?,
                })
            },
        );
        match result {
            Ok(section) => Ok(Some(section)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(StorageError::Database(e)),
        }
    }

    pub fn get_all_sections(&self) -> Result<Vec<Section>> {
        let mut stmt = self.conn.prepare("SELECT id, name, sort_order FROM sections ORDER BY sort_order")?;
        let sections = stmt.query_map([], |row| {
            Ok(Section {
                id: parse_from_text(row, 0)?,
                name: row.get(1)?,
                sort_order: row.get(2)?,
            })
        })?;
        Ok(sections.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    pub fn update_section(&self, section: &Section) -> Result<()> {
        let rows_affected = self.conn.execute(
            "UPDATE sections SET name = ?1, sort_order = ?2 WHERE id = ?3",
            rusqlite::params![section.name, section.sort_order, section.id.to_string()],
        )?;
        if rows_affected == 0 { return Err(StorageError::SectionNotFound(section.id)); }
        Ok(())
    }

    pub fn delete_section(&self, section_id: &Uuid) -> Result<()> {
        let rows_affected = self.conn.execute(
            "DELETE FROM sections WHERE id = ?1",
            rusqlite::params![section_id.to_string()],
        )?;
        if rows_affected == 0 { return Err(StorageError::SectionNotFound(*section_id)); }
        Ok(())
    }

    pub fn get_next_sort_order(&self) -> Result<i32> {
        let sections = self.get_all_sections()?;
        let max = sections.iter().map(|s| s.sort_order).max().unwrap_or(-1);
        Ok(max + 1)
    }
    pub fn save_echo(&self, echo: &Echo) -> Result<()> {
        let content_type = echo.content_type_name().to_string();
        let content_json = serde_json::to_string(&echo.content)?;
        let tags_json = serde_json::to_string(&echo.tags)?;
        let linked_echo_id = echo.linked_echo_id.map(|id| id.to_string());
        self.conn.execute(
            "INSERT INTO echoes (id, day, section_id, title, content_type, content_json, mood, energy, pinned, tags, linked_echo_id, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
            rusqlite::params![
                echo.id.to_string(), echo.day.to_string(), echo.section_id.to_string(), echo.title,
                content_type, content_json,
                echo.mood.map(|v| v as i64), echo.energy.map(|v| v as i64),
                echo.pinned as i64, tags_json, linked_echo_id,
                echo.created_at.to_rfc3339(), echo.updated_at.to_rfc3339(),
            ],
        )?;
        Ok(())
    }

    pub fn get_echo(&self, echo_id: &Uuid) -> Result<Option<Echo>> {
        let result = self.conn.query_row(
            "SELECT id, day, section_id, title, content_json, mood, energy, pinned, tags, linked_echo_id, created_at, updated_at FROM echoes WHERE id = ?1",
            rusqlite::params![echo_id.to_string()],
            map_echo_row,
        );
        match result {
            Ok(inner) => Ok(Some(inner?)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(StorageError::Database(e)),
        }
    }

    pub fn update_echo(&self, echo: &Echo) -> Result<()> {
        let content_type = echo.content_type_name().to_string();
        let content_json = serde_json::to_string(&echo.content)?;
        let tags_json = serde_json::to_string(&echo.tags)?;
        let linked_echo_id = echo.linked_echo_id.map(|id| id.to_string());
        let rows_affected = self.conn.execute(
            "UPDATE echoes SET day = ?1, section_id = ?2, title = ?3, content_type = ?4, content_json = ?5, mood = ?6, energy = ?7, pinned = ?8, tags = ?9, linked_echo_id = ?10, updated_at = ?11 WHERE id = ?12",
            rusqlite::params![
                echo.day.to_string(), echo.section_id.to_string(), echo.title,
                content_type, content_json,
                echo.mood.map(|v| v as i64), echo.energy.map(|v| v as i64),
                echo.pinned as i64, tags_json, linked_echo_id,
                echo.updated_at.to_rfc3339(), echo.id.to_string(),
            ],
        )?;
        if rows_affected == 0 { return Err(StorageError::EchoNotFound(echo.id)); }
        Ok(())
    }

    pub fn delete_echo(&self, echo_id: &Uuid) -> Result<()> {
        let rows_affected = self.conn.execute(
            "DELETE FROM echoes WHERE id = ?1",
            rusqlite::params![echo_id.to_string()],
        )?;
        if rows_affected == 0 { return Err(StorageError::EchoNotFound(*echo_id)); }
        Ok(())
    }
    pub fn get_echoes_for_day(&self, date: NaiveDate) -> Result<Vec<Echo>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, day, section_id, title, content_json, mood, energy, pinned, tags, linked_echo_id, created_at, updated_at FROM echoes WHERE day = ?1 ORDER BY created_at",
        )?;
        let echoes = stmt.query_map(rusqlite::params![date.to_string()], map_echo_row)?;
        echoes.map(|r| r.map_err(StorageError::Database)?.map_err(Into::into)).collect()
    }

    pub fn get_all_echoes(&self) -> Result<Vec<Echo>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, day, section_id, title, content_json, mood, energy, pinned, tags, linked_echo_id, created_at, updated_at FROM echoes ORDER BY day DESC, created_at DESC",
        )?;
        let echoes = stmt.query_map([], map_echo_row)?;
        echoes.map(|r| r.map_err(StorageError::Database)?.map_err(Into::into)).collect()
    }
}

fn map_echo_row(row: &rusqlite::Row) -> rusqlite::Result<Result<Echo>> {
    let id: Uuid = parse_from_text(row, 0)?;
    let day: NaiveDate = parse_from_text(row, 1)?;
    let section_id: Uuid = parse_from_text(row, 2)?;
    let title: String = row.get(3)?;
    let content_json: String = row.get(4)?;
    let mood: Option<i64> = row.get(5)?;
    let energy: Option<i64> = row.get(6)?;
    let pinned: i64 = row.get(7)?;
    let tags_json: String = row.get(8)?;
    let linked_echo_id_str: Option<String> = row.get(9)?;
    let created_at = parse_from_text(row, 10)?;
    let updated_at = parse_from_text(row, 11)?;

    let content: EchoContent = match serde_json::from_str(&content_json) {
        Ok(c) => c,
        Err(e) => return Ok(Err(StorageError::Json(e))),
    };
    let tags: Vec<String> = match serde_json::from_str(&tags_json) {
        Ok(t) => t,
        Err(e) => return Ok(Err(StorageError::Json(e))),
    };
    let linked_echo_id = match linked_echo_id_str {
        Some(s) => match s.parse::<Uuid>() {
            Ok(id) => Some(id),
            Err(e) => return Ok(Err(StorageError::Database(rusqlite::Error::FromSqlConversionFailure(9, rusqlite::types::Type::Text, Box::new(e))))),
        },
        None => None,
    };

    Ok(Ok(Echo {
        id, day, section_id, title, content,
        mood: mood.map(|v| v as u8),
        energy: energy.map(|v| v as u8),
        pinned: pinned != 0,
        tags,
        linked_echo_id,
        created_at,
        updated_at,
    }))
}

fn parse_from_text<T>(row: &rusqlite::Row, idx: usize) -> rusqlite::Result<T>
where
    T: FromStr,
    T::Err: std::error::Error + Send + Sync + 'static,
{
    let text: String = row.get(idx)?;
    text.parse::<T>().map_err(|e| {
        rusqlite::Error::FromSqlConversionFailure(idx, rusqlite::types::Type::Text, Box::new(e))
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Local;
    use saga_core::model::{
        ChecklistItem, EchoContent, MeditationData, PerformedExercise, PlainData, Priority,
        SetEntry, TaskData, WorkoutData,
    };
    use uuid::Uuid;

    fn make_section(storage: &Storage) -> Section {
        let section = Section { id: Uuid::new_v4(), name: "Test Section".to_string(), sort_order: 0 };
        storage.save_section(&section).expect("Failed to save section");
        section
    }

    #[test]
    fn test_save_section() {
        let storage = Storage::new(":memory:").expect("Memory storage creation failed!");
        let section = Section { id: Uuid::new_v4(), name: "Meditation".to_string(), sort_order: 0 };
        storage.save_section(&section).expect("Creating Section failed!");
    }

    #[test]
    fn test_get_section() {
        let storage = Storage::new(":memory:").expect("Failed to create storage");
        let section = Section { id: Uuid::new_v4(), name: "Meditation".to_string(), sort_order: 0 };
        storage.save_section(&section).expect("Failed to create section");
        let found = storage.get_section(&section.id).expect("Query failed");
        assert!(found.is_some());
        assert_eq!(found.unwrap().name, "Meditation");
        let not_found = storage.get_section(&Uuid::new_v4()).expect("Query failed");
        assert!(not_found.is_none());
    }

    #[test]
    fn test_get_all_sections() {
        let storage = Storage::new(":memory:").expect("Failed to create storage");
        let s1 = Section { id: Uuid::new_v4(), name: "Meditation".to_string(), sort_order: 0 };
        let s2 = Section { id: Uuid::new_v4(), name: "Work".to_string(), sort_order: 1 };
        storage.save_section(&s1).expect("Failed to create section 1");
        storage.save_section(&s2).expect("Failed to create section 2");
        let sections = storage.get_all_sections().expect("Failed to get sections");
        assert_eq!(sections.len(), 2);
        assert_eq!(sections[0].name, "Meditation");
        assert_eq!(sections[1].name, "Work");
    }

    #[test]
    fn test_update_section() {
        let storage = Storage::new(":memory:").expect("Failed to create storage");
        let mut section = Section { id: Uuid::new_v4(), name: "Meditation".to_string(), sort_order: 0 };
        storage.save_section(&section).expect("Failed to create");
        section.name = "Mindfulness".to_string();
        section.sort_order = 5;
        storage.update_section(&section).expect("Failed to update");
        let updated = storage.get_section(&section.id).expect("Query failed").unwrap();
        assert_eq!(updated.name, "Mindfulness");
        assert_eq!(updated.sort_order, 5);
    }

    #[test]
    fn test_delete_section() {
        let storage = Storage::new(":memory:").expect("Failed to create storage");
        let section = Section { id: Uuid::new_v4(), name: "Meditation".to_string(), sort_order: 0 };
        storage.save_section(&section).expect("Failed to create");
        storage.delete_section(&section.id).expect("Failed to delete");
        let result = storage.get_section(&section.id).expect("Query failed");
        assert!(result.is_none());
    }
    #[test]
    fn test_create_plain_echo() {
        let storage = Storage::new(":memory:").expect("Failed to create storage");
        let section = make_section(&storage);
        let echo = Echo::new(Local::now().date_naive(), section.id, "Echo title".to_string(), EchoContent::PlainEcho(PlainData { markdown: "Hello world".to_string() }));
        storage.save_echo(&echo).expect("Failed to save echo");
        let found = storage.get_echo(&echo.id).expect("Failed to get echo").unwrap();
        assert_eq!(found.id, echo.id);
        match found.content {
            EchoContent::PlainEcho(data) => assert_eq!(data.markdown, "Hello world"),
            _ => panic!("Wrong content type"),
        }
    }

    #[test]
    fn test_update_echo() {
        let storage = Storage::new(":memory:").expect("Failed to create storage");
        let section = make_section(&storage);
        let mut echo = Echo::new(Local::now().date_naive(), section.id, "Echo title".to_string(), EchoContent::PlainEcho(PlainData { markdown: "Original".to_string() }));
        storage.save_echo(&echo).expect("Failed to save echo");
        echo.update_content(EchoContent::PlainEcho(PlainData { markdown: "Updated".to_string() }));
        storage.update_echo(&echo).expect("Failed to update");
        let found = storage.get_echo(&echo.id).expect("Failed to get echo").unwrap();
        match found.content {
            EchoContent::PlainEcho(data) => assert_eq!(data.markdown, "Updated"),
            _ => panic!("Wrong content type"),
        }
    }

    #[test]
    fn test_get_echoes_for_day() {
        let storage = Storage::new(":memory:").expect("Failed to create storage");
        let section = make_section(&storage);
        let today = Local::now().date_naive();
        let yesterday = today - chrono::Days::new(1);
        let echo1 = Echo::new(today, section.id, "Title 1".to_string(), EchoContent::PlainEcho(PlainData { markdown: "Today first".to_string() }));
        let echo2 = Echo::new(today, section.id, "Title 2".to_string(), EchoContent::PlainEcho(PlainData { markdown: "Today second".to_string() }));
        let echo3 = Echo::new(yesterday, section.id, "Title 3".to_string(), EchoContent::PlainEcho(PlainData { markdown: "Yesterday".to_string() }));
        storage.save_echo(&echo1).expect("Failed to save echo1");
        storage.save_echo(&echo2).expect("Failed to save echo2");
        storage.save_echo(&echo3).expect("Failed to save echo3");
        let today_echoes = storage.get_echoes_for_day(today).expect("Failed to get echoes");
        assert_eq!(today_echoes.len(), 2);
        let yesterday_echoes = storage.get_echoes_for_day(yesterday).expect("Failed to get echoes");
        assert_eq!(yesterday_echoes.len(), 1);
    }

    #[test]
    fn test_meditation_echo_roundtrip() {
        let storage = Storage::new(":memory:").expect("Failed to create storage");
        let section = make_section(&storage);
        let echo = Echo::new(Local::now().date_naive(), section.id, "Morning Sit".to_string(), EchoContent::MeditationEcho(MeditationData { markdown: Some("Felt calm.".to_string()), duration_minutes: 20, mood_before: Some(5), mood_after: Some(8) }));
        storage.save_echo(&echo).expect("Failed to save");
        let found = storage.get_echo(&echo.id).expect("Failed to get").unwrap();
        match found.content {
            EchoContent::MeditationEcho(data) => {
                assert_eq!(data.duration_minutes, 20);
                assert_eq!(data.mood_before, Some(5));
                assert_eq!(data.mood_after, Some(8));
            }
            _ => panic!("Wrong content type"),
        }
    }

    #[test]
    fn test_task_echo_roundtrip() {
        let storage = Storage::new(":memory:").expect("Failed to create storage");
        let section = make_section(&storage);
        let echo = Echo::new(Local::now().date_naive(), section.id, "Buy groceries".to_string(), EchoContent::TaskEcho(TaskData { title: "Buy groceries".to_string(), description: None, due_date: None, due_time: None, completed: false, completed_at: None, priority: Priority::High, checklist: vec![ChecklistItem { text: "Milk".to_string(), done: false }, ChecklistItem { text: "Eggs".to_string(), done: true }], estimated_minutes: Some(30), recurrence: None }));
        storage.save_echo(&echo).expect("Failed to save");
        let found = storage.get_echo(&echo.id).expect("Failed to get").unwrap();
        match found.content {
            EchoContent::TaskEcho(data) => {
                assert_eq!(data.checklist.len(), 2);
                assert_eq!(data.checklist[0].text, "Milk");
                assert!(data.checklist[1].done);
            }
            _ => panic!("Wrong content type"),
        }
    }

    #[test]
    fn test_workout_echo_roundtrip() {
        let storage = Storage::new(":memory:").expect("Failed to create storage");
        let section = make_section(&storage);
        let echo = Echo::new(Local::now().date_naive(), section.id, "Push Day".to_string(), EchoContent::WorkoutEcho(WorkoutData { template_id: None, exercises: vec![PerformedExercise { name: "Bench Press".to_string(), sets: vec![SetEntry { reps: 8, weight_kg: Some(80.0), completed: true, rest_seconds: Some(90), is_warmup: false }] }], duration_minutes: Some(60), notes: Some("Felt strong.".to_string()), perceived_effort: Some(7) }));
        storage.save_echo(&echo).expect("Failed to save");
        let found = storage.get_echo(&echo.id).expect("Failed to get").unwrap();
        match found.content {
            EchoContent::WorkoutEcho(data) => {
                assert_eq!(data.exercises.len(), 1);
                assert_eq!(data.exercises[0].name, "Bench Press");
                assert_eq!(data.exercises[0].sets[0].reps, 8);
                assert_eq!(data.duration_minutes, Some(60));
            }
            _ => panic!("Wrong content type"),
        }
    }

    #[test]
    fn test_echo_shared_fields_roundtrip() {
        let storage = Storage::new(":memory:").expect("Failed to create storage");
        let section = make_section(&storage);
        let mut echo = Echo::new(Local::now().date_naive(), section.id, "Tagged Echo".to_string(), EchoContent::PlainEcho(PlainData { markdown: "With metadata".to_string() }));
        echo.mood = Some(7);
        echo.energy = Some(9);
        echo.pinned = true;
        echo.tags = vec!["rust".to_string(), "learning".to_string()];
        storage.save_echo(&echo).expect("Failed to save");
        let found = storage.get_echo(&echo.id).expect("Failed to get").unwrap();
        assert_eq!(found.mood, Some(7));
        assert_eq!(found.energy, Some(9));
        assert!(found.pinned);
        assert_eq!(found.tags, vec!["rust", "learning"]);
    }
}
