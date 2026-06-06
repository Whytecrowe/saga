use crate::Result;
use rusqlite::{Connection, Transaction};

type Migration = fn(&Transaction) -> rusqlite::Result<()>;

const MIGRATIONS: &[Migration] = &[
    migration_001_initial,
];

pub(crate) fn run_migrations(conn: &mut Connection) -> Result<()> {
    let current: i32 = conn.query_row(
        "PRAGMA user_version",
        [],
        |row| row.get(0),
    )?;

    for (index, migration) in MIGRATIONS.iter().enumerate() {
        let version = (index + 1) as i32;

        if version > current {
            let tx = conn.transaction()?;
            migration(&tx)?;
            tx.execute_batch(&format!("PRAGMA user_version = {version};"))?;
            tx.commit()?;
        }
    }

    Ok(())
}

fn migration_001_initial(tx: &Transaction) -> rusqlite::Result<()> {
    tx.execute_batch(
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

        CREATE INDEX IF NOT EXISTS idx_echoes_day ON echoes(day);

        CREATE TABLE IF NOT EXISTS workout_programs (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            notes TEXT
        );

        CREATE TABLE IF NOT EXISTS workout_templates (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            program_id TEXT,
            sort_order INTEGER NOT NULL,
            exercises_json TEXT NOT NULL DEFAULT '[]',
            FOREIGN KEY (program_id) REFERENCES workout_programs(id)
        );",
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Storage;

    #[test]
    fn test_fresh_db_is_at_latest_version() {
        let storage = Storage::new(":memory:").expect("Failed to create storage");
        let version: i32 = storage
            .conn
            .query_row("PRAGMA user_version", [], |row| row.get(0))
            .expect("Failed to read user_version");
        assert_eq!(version, MIGRATIONS.len() as i32);
    }

    #[test]
    fn test_migrations_are_idempotent() {
        let mut conn = Connection::open_in_memory().expect("Failed to open memory db");
        run_migrations(&mut conn).expect("First migration run failed");
        run_migrations(&mut conn).expect("Second migration run failed");
        let version: i32 = conn
            .query_row("PRAGMA user_version", [], |row| row.get(0))
            .expect("Failed to read user_version");
        assert_eq!(version, MIGRATIONS.len() as i32);
    }

    #[test]
    fn test_adopts_existing_database() {
        let mut conn = Connection::open_in_memory().expect("Failed to open memory db");
        conn.execute_batch(
            "CREATE TABLE sections (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                sort_order INTEGER NOT NULL
            );",
        )
        .expect("Failed to pre-create legacy table");
        conn.execute(
            "INSERT INTO sections (id, name, sort_order) VALUES (?1, ?2, ?3)",
            rusqlite::params!["legacy-id", "Legacy", 0],
        )
        .expect("Failed to insert legacy row");

        run_migrations(&mut conn).expect("Adoption migration failed");

        let version: i32 = conn
            .query_row("PRAGMA user_version", [], |row| row.get(0))
            .expect("Failed to read user_version");
        assert_eq!(version, MIGRATIONS.len() as i32);

        let count: i32 = conn
            .query_row("SELECT COUNT(*) FROM sections", [], |row| row.get(0))
            .expect("Failed to count sections");
        assert_eq!(count, 1);
    }
}
