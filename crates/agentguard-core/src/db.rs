//! Local SQLite storage (see `docs/data-model.md`).
//!
//! Only policy *metadata* lives here — never secret values or file contents (D3).

use crate::model::{PolicyRule, Scope};
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeSet, HashMap};

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

// --- Records -----------------------------------------------------------------

/// A row of the `projects` table (also the Home "recent project" shape).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectRecord {
    pub id: String,
    pub path: String,
    pub name: String,
    pub last_opened_at: String,
    pub risk_profile: Option<String>,
    pub risk_score: Option<i64>,
    pub risk_level: Option<String>,
    pub notes: Option<String>,
}

/// A policy rule tagged with the scope it belongs to (as stored in `project_paths`).
#[derive(Debug, Clone)]
pub struct ScopedRuleRow {
    pub scope: Scope,
    pub rule: PolicyRule,
}

/// Serialize a serde enum to its string form (e.g. `Policy::Allow` -> `"allow"`).
fn enum_str<T: Serialize>(v: &T) -> crate::Result<String> {
    match serde_json::to_value(v)? {
        serde_json::Value::String(s) => Ok(s),
        other => Err(crate::Error::Other(format!(
            "expected enum string, got {other}"
        ))),
    }
}

fn enum_from<T: for<'de> Deserialize<'de>>(s: &str) -> crate::Result<T> {
    Ok(serde_json::from_value(serde_json::Value::String(
        s.to_string(),
    ))?)
}

// --- projects ----------------------------------------------------------------

/// Insert or update a project by path; returns the row id (existing id is kept).
pub fn upsert_project(conn: &Connection, p: &ProjectRecord) -> crate::Result<String> {
    conn.execute(
        r#"INSERT INTO projects
             (id, path, name, last_opened_at, risk_profile, risk_score, risk_level, notes)
           VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
           ON CONFLICT(path) DO UPDATE SET
             name = excluded.name,
             last_opened_at = excluded.last_opened_at,
             risk_profile = COALESCE(excluded.risk_profile, projects.risk_profile),
             risk_score = excluded.risk_score,
             risk_level = excluded.risk_level,
             notes = COALESCE(excluded.notes, projects.notes)"#,
        params![
            p.id,
            p.path,
            p.name,
            p.last_opened_at,
            p.risk_profile,
            p.risk_score,
            p.risk_level,
            p.notes,
        ],
    )?;
    // Return the actual stored id (may differ from p.id if the row pre-existed).
    let id: String = conn.query_row(
        "SELECT id FROM projects WHERE path = ?1",
        params![p.path],
        |r| r.get(0),
    )?;
    Ok(id)
}

/// Find a project by its canonical stored path.
pub fn get_project_by_path(conn: &Connection, path: &str) -> crate::Result<Option<ProjectRecord>> {
    let mut stmt = conn.prepare(
        "SELECT id, path, name, last_opened_at, risk_profile, risk_score, risk_level, notes
         FROM projects WHERE path = ?1",
    )?;
    let mut rows = stmt.query_map(params![path], |r| {
        Ok(ProjectRecord {
            id: r.get(0)?,
            path: r.get(1)?,
            name: r.get(2)?,
            last_opened_at: r.get(3)?,
            risk_profile: r.get(4)?,
            risk_score: r.get(5)?,
            risk_level: r.get(6)?,
            notes: r.get(7)?,
        })
    })?;
    Ok(rows.next().transpose()?)
}

/// Persist the profile selected for a project. `None` explicitly clears it.
pub fn set_project_profile(
    conn: &Connection,
    project_id: &str,
    profile: Option<&str>,
) -> crate::Result<bool> {
    Ok(conn.execute(
        "UPDATE projects SET risk_profile = ?1 WHERE id = ?2",
        params![profile, project_id],
    )? > 0)
}

/// Recent projects, most-recently-opened first.
pub fn list_recent_projects(conn: &Connection, limit: i64) -> crate::Result<Vec<ProjectRecord>> {
    let mut stmt = conn.prepare(
        "SELECT id, path, name, last_opened_at, risk_profile, risk_score, risk_level, notes
         FROM projects ORDER BY last_opened_at DESC LIMIT ?1",
    )?;
    let rows = stmt.query_map(params![limit], |r| {
        Ok(ProjectRecord {
            id: r.get(0)?,
            path: r.get(1)?,
            name: r.get(2)?,
            last_opened_at: r.get(3)?,
            risk_profile: r.get(4)?,
            risk_score: r.get(5)?,
            risk_level: r.get(6)?,
            notes: r.get(7)?,
        })
    })?;
    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}

// --- project_paths (policy rule metadata, D3) --------------------------------

/// Replace all stored rules for `(project_id, scope)` with `rules`.
pub fn save_project_paths(
    conn: &mut Connection,
    project_id: &str,
    scope: Scope,
    rules: &[PolicyRule],
    updated_at: &str,
) -> crate::Result<()> {
    let scope_s = enum_str(&scope)?;
    let tx = conn.transaction()?;
    tx.execute(
        "DELETE FROM project_paths WHERE project_id = ?1 AND scope = ?2",
        params![project_id, scope_s],
    )?;
    for rule in rules {
        let tools = match &rule.tools {
            Some(t) => Some(serde_json::to_string(t)?),
            None => None,
        };
        let risk_level = match &rule.risk_level {
            Some(rl) => Some(enum_str(rl)?),
            None => None,
        };
        tx.execute(
            r#"INSERT INTO project_paths
                 (id, project_id, path, policy, scope, applies_to, tools, reason,
                  risk_level, notes, managed_by_ag, updated_at)
               VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, 1, ?11)"#,
            params![
                uuid::Uuid::new_v4().to_string(),
                project_id,
                rule.path,
                enum_str(&rule.policy)?,
                scope_s,
                enum_str(&rule.applies_to)?,
                tools,
                rule.reason,
                risk_level,
                rule.notes,
                updated_at,
            ],
        )?;
    }
    tx.commit()?;
    Ok(())
}

/// Load all stored rules for a project across scopes.
pub fn load_project_paths(
    conn: &Connection,
    project_id: &str,
) -> crate::Result<Vec<ScopedRuleRow>> {
    let mut stmt = conn.prepare(
        "SELECT path, policy, scope, applies_to, tools, reason, risk_level, notes
         FROM project_paths WHERE project_id = ?1",
    )?;
    let rows = stmt.query_map(params![project_id], |r| {
        let tools_json: Option<String> = r.get(4)?;
        let risk_level_s: Option<String> = r.get(6)?;
        Ok((
            r.get::<_, String>(0)?, // path
            r.get::<_, String>(1)?, // policy
            r.get::<_, String>(2)?, // scope
            r.get::<_, String>(3)?, // applies_to
            tools_json,
            r.get::<_, Option<String>>(5)?, // reason
            risk_level_s,
            r.get::<_, Option<String>>(7)?, // notes
        ))
    })?;

    let mut out = Vec::new();
    for row in rows {
        let (path, policy, scope, applies_to, tools_json, reason, risk_level_s, notes) = row?;
        let tools = match tools_json {
            Some(j) => Some(serde_json::from_str(&j)?),
            None => None,
        };
        let risk_level = match risk_level_s {
            Some(s) => Some(enum_from(&s)?),
            None => None,
        };
        out.push(ScopedRuleRow {
            scope: enum_from(&scope)?,
            rule: PolicyRule {
                path,
                policy: enum_from(&policy)?,
                applies_to: enum_from(&applies_to)?,
                tools,
                reason,
                risk_level,
                notes,
            },
        });
    }
    Ok(out)
}

// --- backups ------------------------------------------------------------------

/// A row of the `backups` table.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupRecord {
    pub id: String,
    pub project_id: Option<String>,
    pub scope: String,
    pub original_path: String,
    pub backup_path: String,
    pub created_at: String,
}

/// Record a created backup.
#[allow(clippy::too_many_arguments)]
pub fn record_backup(
    conn: &Connection,
    project_id: Option<&str>,
    scope: Scope,
    original_path: &str,
    backup_path: &str,
    created_at: &str,
) -> crate::Result<()> {
    conn.execute(
        "INSERT INTO backups (id, project_id, scope, original_path, backup_path, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            uuid::Uuid::new_v4().to_string(),
            project_id,
            enum_str(&scope)?,
            original_path,
            backup_path,
            created_at,
        ],
    )?;
    Ok(())
}

/// List backups for a project (most recent first).
pub fn list_backups(conn: &Connection, project_id: &str) -> crate::Result<Vec<BackupRecord>> {
    let mut stmt = conn.prepare(
        "SELECT id, project_id, scope, original_path, backup_path, created_at
         FROM backups WHERE project_id = ?1 ORDER BY created_at DESC",
    )?;
    let rows = stmt.query_map(params![project_id], |r| {
        Ok(BackupRecord {
            id: r.get(0)?,
            project_id: r.get(1)?,
            scope: r.get(2)?,
            original_path: r.get(3)?,
            backup_path: r.get(4)?,
            created_at: r.get(5)?,
        })
    })?;
    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}

/// Resolve one backup by its opaque database id. File paths are never accepted
/// from the webview for preview/restore operations.
pub fn get_backup(conn: &Connection, id: &str) -> crate::Result<Option<BackupRecord>> {
    let mut stmt = conn.prepare(
        "SELECT id, project_id, scope, original_path, backup_path, created_at
         FROM backups WHERE id = ?1",
    )?;
    let mut rows = stmt.query(params![id])?;
    let Some(row) = rows.next()? else {
        return Ok(None);
    };
    Ok(Some(BackupRecord {
        id: row.get(0)?,
        project_id: row.get(1)?,
        scope: row.get(2)?,
        original_path: row.get(3)?,
        backup_path: row.get(4)?,
        created_at: row.get(5)?,
    }))
}

/// Replace the known sensitive paths for a project.
pub fn save_sensitive_paths(
    conn: &mut Connection,
    project_id: &str,
    paths: &[String],
    source: &str,
) -> crate::Result<()> {
    let tx = conn.transaction()?;
    let existing: HashMap<String, (String, bool)> = {
        let mut stmt = tx.prepare(
            "SELECT path, id, dismissed FROM known_sensitive_paths
             WHERE project_id = ?1 AND source = ?2",
        )?;
        let rows = stmt.query_map(params![project_id, source], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, bool>(2)?,
            ))
        })?;
        rows.collect::<rusqlite::Result<Vec<_>>>()?
            .into_iter()
            .map(|(path, id, dismissed)| (path, (id, dismissed)))
            .collect()
    };
    tx.execute(
        "DELETE FROM known_sensitive_paths WHERE project_id = ?1 AND source = ?2",
        params![project_id, source],
    )?;
    // A scanner can report the same path through more than one detector. Keep a
    // single stable row and, importantly, carry its dismissed bit across rescans.
    for p in paths.iter().collect::<BTreeSet<_>>() {
        let (id, dismissed) = existing
            .get(p.as_str())
            .cloned()
            .unwrap_or_else(|| (uuid::Uuid::new_v4().to_string(), false));
        tx.execute(
            "INSERT INTO known_sensitive_paths (id, project_id, path, source, dismissed)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![id, project_id, p, source, dismissed],
        )?;
    }
    tx.commit()?;
    Ok(())
}

/// A scanner-discovered path and its per-project dismissal state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SensitivePathRecord {
    pub id: String,
    pub project_id: String,
    pub path: String,
    pub source: String,
    pub dismissed: bool,
}

/// List all known sensitive paths for a project, including dismissed entries.
pub fn list_sensitive_paths(
    conn: &Connection,
    project_id: &str,
) -> crate::Result<Vec<SensitivePathRecord>> {
    let mut stmt = conn.prepare(
        "SELECT id, project_id, path, source, dismissed
         FROM known_sensitive_paths WHERE project_id = ?1 ORDER BY source, path",
    )?;
    let rows = stmt.query_map(params![project_id], |r| {
        Ok(SensitivePathRecord {
            id: r.get(0)?,
            project_id: r.get(1)?,
            path: r.get(2)?,
            source: r.get(3)?,
            dismissed: r.get(4)?,
        })
    })?;
    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}

/// Change one sensitive-path dismissal, scoped to its owning project.
pub fn set_sensitive_path_dismissed(
    conn: &Connection,
    project_id: &str,
    id: &str,
    dismissed: bool,
) -> crate::Result<bool> {
    Ok(conn.execute(
        "UPDATE known_sensitive_paths SET dismissed = ?1
         WHERE project_id = ?2 AND id = ?3",
        params![dismissed, project_id, id],
    )? > 0)
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

    fn sample_project() -> ProjectRecord {
        ProjectRecord {
            id: "p1".into(),
            path: "/work/proj".into(),
            name: "proj".into(),
            last_opened_at: "2026-07-09T17:30:00+09:00".into(),
            risk_profile: Some("Conservative".into()),
            risk_score: Some(100),
            risk_level: Some("high".into()),
            notes: None,
        }
    }

    #[test]
    fn upsert_and_list_recent() {
        let mut conn = Connection::open_in_memory().unwrap();
        init(&conn).unwrap();
        let id = upsert_project(&conn, &sample_project()).unwrap();
        assert_eq!(id, "p1");

        // Upsert again (same path) keeps id, updates fields.
        let mut p = sample_project();
        p.id = "different".into();
        p.risk_score = Some(50);
        let id2 = upsert_project(&conn, &p).unwrap();
        assert_eq!(id2, "p1"); // existing id kept

        let recent = list_recent_projects(&conn, 10).unwrap();
        assert_eq!(recent.len(), 1);
        assert_eq!(recent[0].risk_score, Some(50));

        // Silence unused-mut on some toolchains.
        let _ = &mut conn;
    }

    #[test]
    fn reopening_preserves_profile_and_notes_when_new_values_are_absent() {
        let conn = Connection::open_in_memory().unwrap();
        init(&conn).unwrap();
        let mut original = sample_project();
        original.notes = Some("keep this".into());
        upsert_project(&conn, &original).unwrap();

        let mut reopened = sample_project();
        reopened.id = "replacement-id".into();
        reopened.last_opened_at = "2026-07-10T00:00:00Z".into();
        reopened.risk_profile = None;
        reopened.notes = None;
        reopened.risk_score = Some(25);
        upsert_project(&conn, &reopened).unwrap();

        let stored = get_project_by_path(&conn, "/work/proj").unwrap().unwrap();
        assert_eq!(stored.id, "p1");
        assert_eq!(stored.risk_profile.as_deref(), Some("Conservative"));
        assert_eq!(stored.notes.as_deref(), Some("keep this"));
        assert_eq!(stored.risk_score, Some(25));
        assert_eq!(stored.last_opened_at, "2026-07-10T00:00:00Z");

        assert!(set_project_profile(&conn, "p1", Some("balanced")).unwrap());
        assert_eq!(
            get_project_by_path(&conn, "/work/proj")
                .unwrap()
                .unwrap()
                .risk_profile
                .as_deref(),
            Some("balanced")
        );
        assert!(!set_project_profile(&conn, "missing", Some("custom")).unwrap());
    }

    #[test]
    fn project_paths_round_trip() {
        use crate::model::{AppliesTo, Policy, RiskLevel, Tool};
        let mut conn = Connection::open_in_memory().unwrap();
        init(&conn).unwrap();
        upsert_project(&conn, &sample_project()).unwrap();

        let mut r1 = PolicyRule::new("src", Policy::Allow, AppliesTo::FolderAndChildren);
        r1.reason = Some("source".into());
        r1.risk_level = Some(RiskLevel::Low);
        let mut r2 = PolicyRule::new("notes", Policy::Deny, AppliesTo::Folder);
        r2.tools = Some(vec![Tool::Read]);

        save_project_paths(&mut conn, "p1", Scope::Project, &[r1, r2], "t0").unwrap();
        let loaded = load_project_paths(&conn, "p1").unwrap();
        assert_eq!(loaded.len(), 2);

        let src = loaded.iter().find(|x| x.rule.path == "src").unwrap();
        assert_eq!(src.scope, Scope::Project);
        assert_eq!(src.rule.policy, Policy::Allow);
        assert_eq!(src.rule.reason.as_deref(), Some("source"));
        assert_eq!(src.rule.risk_level, Some(RiskLevel::Low));

        let notes = loaded.iter().find(|x| x.rule.path == "notes").unwrap();
        assert_eq!(notes.rule.tools, Some(vec![Tool::Read]));

        // Re-save replaces (not duplicates).
        save_project_paths(&mut conn, "p1", Scope::Project, &[], "t1").unwrap();
        assert_eq!(load_project_paths(&conn, "p1").unwrap().len(), 0);
    }

    #[test]
    fn sensitive_path_rescan_preserves_dismissal_and_drops_stale_rows() {
        let mut conn = Connection::open_in_memory().unwrap();
        init(&conn).unwrap();
        upsert_project(&conn, &sample_project()).unwrap();

        save_sensitive_paths(
            &mut conn,
            "p1",
            &[".env".into(), "secret.pem".into(), ".env".into()],
            "scanner",
        )
        .unwrap();
        let first = list_sensitive_paths(&conn, "p1").unwrap();
        assert_eq!(first.len(), 2, "duplicate scanner hits are folded");
        let env = first.iter().find(|r| r.path == ".env").unwrap();
        let env_id = env.id.clone();
        assert!(set_sensitive_path_dismissed(&conn, "p1", &env_id, true).unwrap());
        assert!(!set_sensitive_path_dismissed(&conn, "other", &env_id, false).unwrap());

        save_sensitive_paths(
            &mut conn,
            "p1",
            &[".env".into(), "new.key".into()],
            "scanner",
        )
        .unwrap();
        let rescanned = list_sensitive_paths(&conn, "p1").unwrap();
        assert_eq!(rescanned.len(), 2);
        let env = rescanned.iter().find(|r| r.path == ".env").unwrap();
        assert_eq!(env.id, env_id);
        assert!(env.dismissed);
        assert!(rescanned.iter().all(|r| r.path != "secret.pem"));
        assert!(
            !rescanned
                .iter()
                .find(|r| r.path == "new.key")
                .unwrap()
                .dismissed
        );
    }

    #[test]
    fn backup_lookup_uses_opaque_id() {
        let conn = Connection::open_in_memory().unwrap();
        init(&conn).unwrap();
        upsert_project(&conn, &sample_project()).unwrap();
        record_backup(
            &conn,
            Some("p1"),
            Scope::Project,
            "/work/proj/.claude/settings.json",
            "/backups/project.json",
            "t0",
        )
        .unwrap();

        let listed = list_backups(&conn, "p1").unwrap();
        let resolved = get_backup(&conn, &listed[0].id).unwrap().unwrap();
        assert_eq!(resolved.backup_path, "/backups/project.json");
        assert!(get_backup(&conn, "missing").unwrap().is_none());
    }
}
