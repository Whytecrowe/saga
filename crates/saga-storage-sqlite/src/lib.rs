use chrono::{NaiveDate, Utc};
use rusqlite::Connection;
use saga_core::model::{Echo, Section};
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
}

pub type Result<T> = std::result::Result<T, StorageError>;

pub struct Storage {
    conn: Connection,
}

impl Storage {
    pub fn new<P: AsRef<Path>>(db_path: P) -> Result<Self> {
        let conn = Connection::open(db_path)?;

        // Enable WAL mode - allows concurrent readers
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
                markdown TEXT NOT NULL,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                FOREIGN KEY (section_id) REFERENCES sections(id)
            );

            CREATE INDEX IF NOT EXISTS idx_echoes_day ON echoes(day);",
        )?;
        Ok(())
    }

    // SECTIONS

    pub fn save_section(&self, section: &Section) -> Result<()> {
        self.conn.execute(
            "INSERT INTO sections (id, name, sort_order) VALUES (?1, ?2, ?3)",
            rusqlite::params![section.id.to_string(), section.name, section.sort_order,],
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
            Ok(section) => Ok(Some(section)),                      // Found it
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None), // Not found
            Err(e) => Err(StorageError::Database(e)),              // Real error
        }
    }

    pub fn get_all_sections(&self) -> Result<Vec<Section>> {
        let mut stmt = self
            .conn
            .prepare("SELECT id, name, sort_order FROM sections ORDER BY sort_order")?;

        let sections = stmt.query_map([], |row| {
            Ok(Section {
                id: parse_from_text(row, 0)?,
                name: row.get(1)?,
                sort_order: row.get(2)?,
            })
        })?;

        let sections = sections.collect::<rusqlite::Result<Vec<_>>>()?;

        Ok(sections)
    }

    pub fn update_section(&self, section: &Section) -> Result<()> {
        let rows_affected = self.conn.execute(
            "UPDATE sections SET name =?1, sort_order = ?2 WHERE id = ?3",
            rusqlite::params![
                section.name.to_string(),
                section.sort_order,
                section.id.to_string(),
            ],
        )?;

        if rows_affected == 0 {
            return Err(StorageError::SectionNotFound(section.id));
        }

        Ok(())
    }

    pub fn delete_section(&self, section_id: &Uuid) -> Result<()> {
        let rows_affected = self.conn.execute(
            "DELETE FROM sections WHERE id = ?1",
            rusqlite::params![section_id.to_string()],
        )?;

        if rows_affected == 0 {
            return Err(StorageError::SectionNotFound(section_id.clone()));
        }

        Ok(())
    }

    pub fn get_next_sort_order(&self) -> Result<i32> {
        let sections = self.get_all_sections()?;
        let max = sections.iter().map(|s| s.sort_order).max().unwrap_or(-1); // If no sections, start at 0
        Ok(max + 1)
    }

    // ECHOES

    pub fn save_echo(&self, echo: &Echo) -> Result<()> {
        self.conn.execute(
            "INSERT INTO echoes (id, day, section_id, title, markdown, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            rusqlite::params![
                echo.id.to_string(),
                echo.day.to_string(),
                echo.section_id.to_string(),
                echo.title,
                echo.markdown,
                echo.created_at.to_string(),
                echo.updated_at.to_string(),
            ]
        )?;

        Ok(())
    }

    pub fn get_echo(&self, echo_id: &Uuid) -> Result<Option<Echo>> {
        let result = self.conn.query_row(
            "SELECT id, day, section_id, title, markdown, created_at, updated_at FROM echoes WHERE id = ?1",
            rusqlite::params![echo_id.to_string()],
            |row| {
                Ok(Echo {
                    id: parse_from_text(row, 0)?,
                    day: parse_from_text(row, 1)?,
                    section_id: parse_from_text(row, 2)?,
                    title: parse_from_text(row, 3)?,
                    markdown: row.get(4)?,
                    created_at: parse_from_text(row, 5)?,
                    updated_at: parse_from_text(row, 6)?,
                })
            }
        );

        match result {
            Ok(echo) => Ok(Some(echo)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(StorageError::Database(e)),
        }
    }

    // TODO: add more optimal methods where only markdown is updated
    pub fn update_echo(&self, echo: &Echo) -> Result<()> {
        let rows_affected = self.conn.execute(
            "UPDATE echoes SET day = ?1, section_id = ?2, title = ?3, markdown = ?4, updated_at = ?5 WHERE id = ?6",
            rusqlite::params![
                echo.day.to_string(),
                echo.section_id.to_string(),
                echo.title,
                echo.markdown,
                Utc::now().to_rfc3339(),
                echo.id.to_string(),
            ]
        )?;

        if rows_affected == 0 {
            return Err(StorageError::EchoNotFound(echo.id));
        }

        Ok(())
    }

    pub fn delete_echo(&self, echo_id: &Uuid) -> Result<()> {
        let rows_affected = self.conn.execute(
            "DELETE FROM echoes WHERE id = ?1",
            rusqlite::params![echo_id.to_string()],
        )?;

        if rows_affected == 0 {
            return Err(StorageError::EchoNotFound(echo_id.clone()));
        }

        Ok(())
    }

    pub fn get_echoes_for_day(&self, date: NaiveDate) -> Result<Vec<Echo>> {
        let mut res = self.conn.prepare(
            "SELECT id, day, section_id, title, markdown, created_at, updated_at
            FROM echoes
            WHERE day = ?1
            ORDER BY created_at",
        )?;

        let echoes = res.query_map(rusqlite::params![date.to_string()], |row| {
            Ok(Echo {
                id: parse_from_text(row, 0)?,
                day: parse_from_text(row, 1)?,
                section_id: parse_from_text(row, 2)?,
                title: parse_from_text(row, 3)?,
                markdown: parse_from_text(row, 4)?,
                created_at: parse_from_text(row, 5)?,
                updated_at: parse_from_text(row, 6)?,
            })
        })?;

        let echoes = echoes.collect::<rusqlite::Result<Vec<_>>>()?;

        Ok(echoes)
    }

    pub fn get_all_echoes(&self) -> Result<Vec<Echo>> {
        let mut res = self.conn.prepare(
            "SELECT id, day, section_id, title, markdown, created_at, updated_at
            FROM echoes
            ORDER BY day DESC, created_at DESC",
        )?;

        let echoes = res
            .query_map([], |row| {
                Ok(Echo {
                    id: parse_from_text(row, 0)?,
                    day: parse_from_text(row, 1)?,
                    section_id: parse_from_text(row, 2)?,
                    title: parse_from_text(row, 3)?,
                    markdown: parse_from_text(row, 4)?,
                    created_at: parse_from_text(row, 5)?,
                    updated_at: parse_from_text(row, 6)?,
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;

        Ok(echoes)
    }
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
    use crate::Section;
    use chrono::Local;
    use uuid::Uuid;

    #[test]
    fn test_save_section() {
        let storage = Storage::new(":memory:").expect("Memory storage creation failed!");

        let section = Section {
            id: Uuid::new_v4(),
            name: "Meditation".to_string(),
            sort_order: 0,
        };

        storage
            .save_section(&section)
            .expect("Creating Section failed!");
    }

    #[test]
    fn test_get_section() {
        let storage = Storage::new(":memory:").expect("Failed to create storage");

        let section = Section {
            id: Uuid::new_v4(),
            name: "Meditation".to_string(),
            sort_order: 0,
        };

        storage
            .save_section(&section)
            .expect("Failed to create section");

        // Test found case
        let found = storage.get_section(&section.id).expect("Query failed");
        assert!(found.is_some());
        assert_eq!(found.unwrap().name, "Meditation");

        // Test not found case
        let not_found = storage.get_section(&Uuid::new_v4()).expect("Query failed");
        assert!(not_found.is_none());
    }

    #[test]
    fn test_get_all_sections() {
        let storage = Storage::new(":memory:").expect("Failed to create storage");

        // Create multiple sections
        let section1 = Section {
            id: Uuid::new_v4(),
            name: "Meditation".to_string(),
            sort_order: 0,
        };

        let section2 = Section {
            id: Uuid::new_v4(),
            name: "Work".to_string(),
            sort_order: 1,
        };

        storage
            .save_section(&section1)
            .expect("Failed to create section 1");
        storage
            .save_section(&section2)
            .expect("Failed to create section 2");

        // Get all
        let sections = storage.get_all_sections().expect("Failed to get sections");

        assert_eq!(sections.len(), 2);
        assert_eq!(sections[0].name, "Meditation"); // sort_order 0 comes first
        assert_eq!(sections[1].name, "Work"); // sort_order 1 comes second
    }

    #[test]
    fn test_update_section() {
        let storage = Storage::new(":memory:").expect("Failed to create storage");

        let mut section = Section {
            id: Uuid::new_v4(),
            name: "Meditation".to_string(),
            sort_order: 0,
        };

        storage.save_section(&section).expect("Failed to create");

        // Update it
        section.name = "Mindfulness".to_string();
        section.sort_order = 5;
        storage.update_section(&section).expect("Failed to update");

        // Verify
        let updated = storage
            .get_section(&section.id)
            .expect("Query failed")
            .unwrap();
        assert_eq!(updated.name, "Mindfulness");
        assert_eq!(updated.sort_order, 5);
    }

    #[test]
    fn test_delete_section() {
        let storage = Storage::new(":memory:").expect("Failed to create storage");

        let section = Section {
            id: Uuid::new_v4(),
            name: "Meditation".to_string(),
            sort_order: 0,
        };

        storage.save_section(&section).expect("Failed to create");
        storage
            .delete_section(&section.id)
            .expect("Failed to delete");

        // Verify it's gone
        let result = storage.get_section(&section.id).expect("Query failed");
        assert!(result.is_none());
    }

    #[test]
    fn test_create_echo() {
        let storage = Storage::new(":memory:").expect("Failed to create storage");

        let section = Section {
            id: Uuid::new_v4(),
            name: "Meditation".to_string(),
            sort_order: 0,
        };

        storage
            .save_section(&section)
            .expect("Creating Section failed!");

        let echo = Echo::new(
            Local::now().date_naive(),
            section.id,
            "Echo title".to_string(),
            "Echo texttttt".to_string(),
        );

        storage.save_echo(&echo).expect("Failed to create echo");

        let found_echo = storage.get_echo(&echo.id).expect("Failed to get echo");
        assert!(found_echo.is_some());

        let found_unwrapped = found_echo.unwrap();
        assert_eq!(found_unwrapped.id, echo.id);
        assert_eq!(found_unwrapped.markdown, echo.markdown);
    }

    #[test]
    fn test_update_echo() {
        let storage = Storage::new(":memory:").expect("Failed to create storage");

        let section = Section {
            id: Uuid::new_v4(),
            name: "Meditation".to_string(),
            sort_order: 0,
        };

        storage
            .save_section(&section)
            .expect("Creating Section failed!");

        let mut echo = Echo::new(
            Local::now().date_naive(),
            section.id,
            "Echo title".to_string(),
            "Echo texttttt".to_string(),
        );

        storage.save_echo(&echo).expect("Failed to create echo");

        echo.markdown = "New updated text!".to_string();

        storage.update_echo(&echo).expect("Failed to update");

        let found_echo = storage.get_echo(&echo.id).expect("Failed to get echo");
        assert!(found_echo.is_some());
        let found_unwrapped = found_echo.unwrap();
        assert_eq!(found_unwrapped.markdown, echo.markdown);
    }

    #[test]
    fn test_get_echoes_for_day() {
        let storage = Storage::new(":memory:").expect("Failed to create storage");

        // Create section
        let section = Section {
            id: Uuid::new_v4(),
            name: "Meditation".to_string(),
            sort_order: 0,
        };
        storage
            .save_section(&section)
            .expect("Failed to create section");

        // Create echoes for different days
        let today = Local::now().date_naive();
        let yesterday = today - chrono::Days::new(1);

        let echo1 = Echo::new(
            today,
            section.id,
            "Title 1".to_string(),
            "Today's first echo".to_string(),
        );
        let echo2 = Echo::new(
            today,
            section.id,
            "Title 2".to_string(),
            "Today's second echo".to_string(),
        );
        let echo3 = Echo::new(
            yesterday,
            section.id,
            "Title 3".to_string(),
            "Yesterday's echo".to_string(),
        );

        storage.save_echo(&echo1).expect("Failed to save echo1");
        storage.save_echo(&echo2).expect("Failed to save echo2");
        storage.save_echo(&echo3).expect("Failed to save echo3");

        // Get echoes for today
        let today_echoes = storage
            .get_echoes_for_day(today)
            .expect("Failed to get echoes for today");

        assert_eq!(today_echoes.len(), 2);
        assert!(
            today_echoes
                .iter()
                .any(|e| e.markdown == "Today's first echo")
        );
        assert!(
            today_echoes
                .iter()
                .any(|e| e.markdown == "Today's second echo")
        );

        // Get echoes for yesterday
        let yesterday_echoes = storage
            .get_echoes_for_day(yesterday)
            .expect("Failed to get echoes for yesterday");

        assert_eq!(yesterday_echoes.len(), 1);
        assert_eq!(yesterday_echoes[0].markdown, "Yesterday's echo");
    }
}
