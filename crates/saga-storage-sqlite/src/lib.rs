use thiserror::Error;
use uuid::Uuid;
use rusqlite::Connection;
use std::path::Path;
use saga_core::model::Section;

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
            markdown TEXT NOT NULL,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            FOREIGN KEY (section_id) REFERENCES sections(id)
        );

        CREATE INDEX IF NOT EXISTS idx_echoes_day ON echoes(day);"
        )?;
        Ok(())
    }

    pub fn create_section(&self, section: &Section) -> Result<()> {
        self.conn.execute(
            "INSERT INTO sections (id, name, sort_order) VALUES (?1, ?2, ?3)",
            rusqlite::params![
                section.id,
                section.name,
                section.sort_order,
            ]
        )?;

        Ok(())
    }

    pub fn get_section(&self, section_id: &Uuid) -> Result<Option<Section>> {
        let result = self.conn.query_row(
            "SELECT id, name, sort_order FROM sections WHERE id = ?1",
            rusqlite::params![section_id],
            |row| {
                Ok(Section {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    sort_order: row.get(2)?,
                })
            }
        );

        match result {
            Ok(section) => Ok(Some(section)),  // Found it!
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),  // Not found
            Err(e) => Err(StorageError::Database(e)),  // Real error
        }
    }

    pub fn get_all_sections(&self) -> Result<Vec<Section>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, sort_order FROM sections ORDER BY sort_order"
        )?;

        let sections = stmt.query_map(
            [],
            |row| {
                Ok(Section {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    sort_order: row.get(2)?,
                })
            }
        )?;

        let sections = sections.collect::<rusqlite::Result<Vec<_>>>()?;

        Ok(sections)
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::Section;
    use uuid::Uuid;

    #[test]
    fn test_create_section() {
        let storage = Storage::new(":memory:")
            .expect("Memory storage creation failed!");

        let section = Section {
            id: Uuid::new_v4(),
            name: "Meditation".to_string(),
            sort_order: 0,
        };

        storage.create_section(&section)
            .expect("Creating Section failed!");
    }

    #[test]
    fn test_get_section() {
        let storage = Storage::new(":memory:")
            .expect("Failed to create storage");

        let section = Section {
            id: Uuid::new_v4(),
            name: "Meditation".to_string(),
            sort_order: 0,
        };

        storage.create_section(&section)
            .expect("Failed to create section");

        // Test found case
        let found = storage.get_section(&section.id)
            .expect("Query failed");
        assert!(found.is_some());
        assert_eq!(found.unwrap().name, "Meditation");

        // Test not found case
        let not_found = storage.get_section(&Uuid::new_v4())
            .expect("Query failed");
        assert!(not_found.is_none());
    }

    #[test]
    fn test_get_all_sections() {
        let storage = Storage::new(":memory:")
            .expect("Failed to create storage");

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

        storage.create_section(&section1).expect("Failed to create section 1");
        storage.create_section(&section2).expect("Failed to create section 2");

        // Get all
        let sections = storage.get_all_sections()
            .expect("Failed to get sections");

        assert_eq!(sections.len(), 2);
        assert_eq!(sections[0].name, "Meditation");  // sort_order 0 comes first
        assert_eq!(sections[1].name, "Work");        // sort_order 1 comes second
    }
}