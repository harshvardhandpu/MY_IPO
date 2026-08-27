use std::path::Path;
use std::time::Duration;

use rusqlite::{Connection, OptionalExtension};
use thiserror::Error;

const SCHEMA_V1: &str = r#"
CREATE TABLE IF NOT EXISTS schema_migrations (
    version INTEGER PRIMARY KEY,
    applied_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS projection_events (
    event_id TEXT PRIMARY KEY,
    content_hash TEXT NOT NULL,
    applied_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

INSERT OR IGNORE INTO schema_migrations(version) VALUES (1);
"#;

pub struct LocalIndex {
    connection: Connection,
}

#[derive(Debug, Error)]
pub enum LocalIndexError {
    #[error("failed to create local index directory: {0}")]
    CreateDirectory(#[source] std::io::Error),
    #[error("failed to restrict local index permissions: {0}")]
    Permissions(#[source] std::io::Error),
    #[error("local SQLite operation failed: {0}")]
    Sqlite(#[from] rusqlite::Error),
}

impl LocalIndex {
    pub fn open(path: &Path) -> Result<Self, LocalIndexError> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(LocalIndexError::CreateDirectory)?;
        }
        let connection = Connection::open(path)?;
        restrict_database_permissions(path)?;
        connection.busy_timeout(Duration::from_secs(5))?;
        connection.pragma_update(None, "foreign_keys", true)?;
        connection.pragma_update(None, "journal_mode", "WAL")?;
        connection.execute_batch(SCHEMA_V1)?;
        Ok(Self { connection })
    }

    pub fn schema_version(&self) -> Result<u32, LocalIndexError> {
        let version = self.connection.query_row(
            "SELECT COALESCE(MAX(version), 0) FROM schema_migrations",
            [],
            |row| row.get(0),
        )?;
        Ok(version)
    }

    pub fn has_table(&self, table: &str) -> Result<bool, LocalIndexError> {
        let exists = self
            .connection
            .query_row(
                "SELECT 1 FROM sqlite_schema WHERE type = 'table' AND name = ?1",
                [table],
                |row| row.get::<_, u8>(0),
            )
            .optional()?
            .is_some();
        Ok(exists)
    }
}

#[cfg(unix)]
fn restrict_database_permissions(path: &Path) -> Result<(), LocalIndexError> {
    use std::os::unix::fs::PermissionsExt;

    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))
        .map_err(LocalIndexError::Permissions)
}

#[cfg(not(unix))]
fn restrict_database_permissions(_path: &Path) -> Result<(), LocalIndexError> {
    Ok(())
}
