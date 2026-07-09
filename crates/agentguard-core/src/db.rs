//! Local SQLite storage (see `docs/data-model.md`).
//!
//! Only policy *metadata* lives here — never secret values or file contents (D3).

use rusqlite::Connection;

/// Current schema version.
pub const SCHEMA_VERSION: i64 = 1;

/// Open (creating if needed) the database at `path` and ensure the schema is current.
pub fn open(path: &std::path::Path) -> crate::Result<Connection> {
    let conn = Connection::open(path)?;
    init(&conn)?;
    Ok(conn)
}

/// Initialize / migrate the schema on an open connection.
pub fn init(conn: &Connection) -> crate::Result<()> {
    conn.execute_batch(
        r#"
        PRAGMA foreign_keys = ON;

        CREATE TABLE IF NOT EXISTS projects (
            id             TEXT PRIMARY KEY,
            path           TEXT NOT NULL UNIQUE,
            name           TEXT NOT NULL,
            last_opened_at TEXT NOT NULL,
            risk_profile   TEXT,
            risk_score     INTEGER,
            risk_level     TEXT,
            notes          TEXT
        );

        CREATE TABLE IF NOT EXISTS project_paths (
            id            TEXT PRIMARY KEY,
            project_id    TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
            path          TEXT NOT NULL,
            policy        TEXT NOT NULL,
            scope         TEXT NOT NULL,
            applies_to    TEXT NOT NULL,
            tools         TEXT,
            reason        TEXT,
            risk_level    TEXT,
            notes         TEXT,
            managed_by_ag INTEGER NOT NULL DEFAULT 1,
            updated_at    TEXT NOT NULL,
            UNIQUE(project_id, scope, path, policy)
        );

        CREATE TABLE IF NOT EXISTS backups (
            id            TEXT PRIMARY KEY,
            project_id    TEXT REFERENCES projects(id) ON DELETE CASCADE,
            scope         TEXT NOT NULL,
            original_path TEXT NOT NULL,
            backup_path   TEXT NOT NULL,
            created_at    TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS known_sensitive_paths (
            id         TEXT PRIMARY KEY,
            project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
            path       TEXT NOT NULL,
            source     TEXT NOT NULL,
            dismissed  INTEGER NOT NULL DEFAULT 0
        );
        "#,
    )?;
    conn.pragma_update(None, "user_version", SCHEMA_VERSION)?;
    Ok(())
}

/// Read the schema version stored in the database.
pub fn schema_version(conn: &Connection) -> crate::Result<i64> {
    let v: i64 = conn.query_row("PRAGMA user_version", [], |r| r.get(0))?;
    Ok(v)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn init_creates_all_tables_and_sets_version() {
        let conn = Connection::open_in_memory().unwrap();
        init(&conn).unwrap();

        assert_eq!(schema_version(&conn).unwrap(), SCHEMA_VERSION);

        let count: i64 = conn
            .query_row(
                "SELECT count(*) FROM sqlite_master WHERE type='table' AND name IN
                 ('projects','project_paths','backups','known_sensitive_paths')",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(count, 4);
    }

    #[test]
    fn init_is_idempotent() {
        let conn = Connection::open_in_memory().unwrap();
        init(&conn).unwrap();
        init(&conn).unwrap(); // running twice must not error
        assert_eq!(schema_version(&conn).unwrap(), SCHEMA_VERSION);
    }
}
