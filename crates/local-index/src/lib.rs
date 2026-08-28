use std::path::Path;
use std::time::Duration;

use rusqlite::{Connection, OptionalExtension, params};
use thiserror::Error;

use sanket_domain::EventEnvelope;

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

// Schema v2: normalized projection tables for the investment domain. None of
// these tables may ever carry a full PAN, UPI, or proof content — only masked
// PANs, stable ids, and integer paise/basis-point amounts.
const SCHEMA_V2: &str = r#"
CREATE TABLE IF NOT EXISTS members (
    id TEXT PRIMARY KEY,
    display_name TEXT NOT NULL,
    role TEXT NOT NULL,
    masked_pan TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'ACTIVE'
);

CREATE TABLE IF NOT EXISTS friend_accounts (
    id TEXT PRIMARY KEY,
    owner_member_id TEXT NOT NULL,
    label TEXT NOT NULL,
    masked_pan TEXT NOT NULL,
    share_basis_points INTEGER NOT NULL,
    status TEXT NOT NULL DEFAULT 'ACTIVE'
);

CREATE TABLE IF NOT EXISTS ipos (
    id TEXT PRIMARY KEY,
    typed_name TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS investment_sessions (
    id TEXT PRIMARY KEY,
    actor_member_id TEXT NOT NULL,
    declared_capital_paise INTEGER NOT NULL,
    status TEXT NOT NULL DEFAULT 'OPEN',
    recommendation_id TEXT
);

CREATE TABLE IF NOT EXISTS applications (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL,
    ipo_name TEXT NOT NULL,
    planned_amount_paise INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS allocations (
    id TEXT PRIMARY KEY,
    application_id TEXT NOT NULL,
    account_id TEXT NOT NULL,
    amount_paise INTEGER NOT NULL,
    share_basis_points INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS recommendations (
    session_id TEXT NOT NULL,
    algorithm_version TEXT NOT NULL,
    recommendation_id TEXT NOT NULL
);

INSERT OR IGNORE INTO schema_migrations(version) VALUES (2);
"#;

// Schema v3: allotment jobs and per-account attempts. Never store PAN.
const SCHEMA_V3: &str = r#"
CREATE TABLE IF NOT EXISTS allotment_jobs (
    id TEXT PRIMARY KEY,
    application_id TEXT NOT NULL,
    session_id TEXT NOT NULL,
    ipo_name TEXT NOT NULL,
    registrar_id TEXT NOT NULL,
    registrar_name TEXT NOT NULL,
    official_status_url TEXT,
    provider_id TEXT NOT NULL,
    status TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS allotment_attempts (
    id TEXT PRIMARY KEY,
    job_id TEXT NOT NULL,
    account_id TEXT NOT NULL,
    status TEXT NOT NULL,
    attempt_count INTEGER NOT NULL DEFAULT 0,
    allotted_lots INTEGER,
    allotted_shares INTEGER,
    provider_reference TEXT,
    safe_message TEXT,
    source TEXT NOT NULL,
    last_attempt_at TEXT,
    next_retry_at TEXT
);

CREATE INDEX IF NOT EXISTS idx_allotment_attempts_job ON allotment_attempts(job_id);

INSERT OR IGNORE INTO schema_migrations(version) VALUES (3);
"#;

pub struct LocalIndex {
    connection: Connection,
}

pub type FriendProjectionRow = (String, String, String, String, i64);
pub type AllocationProjectionRow = (String, String, String, i64, i64);
pub type SubmittedApplicationRow = (String, String, String, i64, u32);
pub type AllotmentJobRow = (String, String, String, String, String, String);
pub type AllotmentAttemptRow = (String, String, String, Option<i64>, Option<i64>, String);

#[derive(Debug, Error)]
pub enum LocalIndexError {
    #[error("failed to create local index directory: {0}")]
    CreateDirectory(#[source] std::io::Error),
    #[error("failed to restrict local index permissions: {0}")]
    Permissions(#[source] std::io::Error),
    #[error("local SQLite operation failed: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("event serialization failed: {0}")]
    Event(#[from] sanket_domain::EventError),
    #[error("unsupported event payload for projection: {0}")]
    UnsupportedEvent(String),
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
        connection.execute_batch(SCHEMA_V2)?;
        connection.execute_batch(SCHEMA_V3)?;
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

    /// Column names of a table (for security inspection).
    pub fn columns(&self, table: &str) -> Result<Vec<String>, LocalIndexError> {
        let mut stmt = self
            .connection
            .prepare(&format!("PRAGMA table_info({table})"))?;
        let rows = stmt.query_map([], |row| row.get::<_, String>(1))?;
        let mut cols = Vec::new();
        for c in rows {
            cols.push(c?);
        }
        Ok(cols)
    }

    // --- projection: members & friends ---

    pub fn upsert_member(
        &self,
        id: &str,
        display_name: &str,
        role: &str,
        masked_pan: &str,
    ) -> Result<(), LocalIndexError> {
        self.connection.execute(
            "INSERT INTO members(id, display_name, role, masked_pan, status)
             VALUES (?1, ?2, ?3, ?4, 'ACTIVE')
             ON CONFLICT(id) DO UPDATE SET display_name=excluded.display_name,
                                           role=excluded.role,
                                           masked_pan=excluded.masked_pan",
            params![id, display_name, role, masked_pan],
        )?;
        Ok(())
    }

    pub fn upsert_friend(
        &self,
        id: &str,
        owner_member_id: &str,
        label: &str,
        masked_pan: &str,
        share_basis_points: i64,
    ) -> Result<(), LocalIndexError> {
        self.connection.execute(
            "INSERT INTO friend_accounts(id, owner_member_id, label, masked_pan, share_basis_points, status)
             VALUES (?1, ?2, ?3, ?4, ?5, 'ACTIVE')
             ON CONFLICT(id) DO UPDATE SET owner_member_id=excluded.owner_member_id,
                                           label=excluded.label,
                                           masked_pan=excluded.masked_pan,
                                           share_basis_points=excluded.share_basis_points",
            params![id, owner_member_id, label, masked_pan, share_basis_points],
        )?;
        Ok(())
    }

    pub fn archive_friend(&self, id: &str) -> Result<(), LocalIndexError> {
        self.connection.execute(
            "UPDATE friend_accounts SET status='ARCHIVED' WHERE id = ?1",
            params![id],
        )?;
        Ok(())
    }

    pub fn list_members(&self) -> Result<Vec<(String, String, String, String)>, LocalIndexError> {
        let mut stmt = self
            .connection
            .prepare("SELECT id, display_name, role, masked_pan FROM members ORDER BY id")?;
        let rows = stmt.query_map([], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
        })?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r?);
        }
        Ok(out)
    }

    pub fn list_active_friends(&self) -> Result<Vec<FriendProjectionRow>, LocalIndexError> {
        let mut stmt = self.connection.prepare(
            "SELECT id, owner_member_id, label, masked_pan, share_basis_points
             FROM friend_accounts WHERE status = 'ACTIVE' ORDER BY id",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok((
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                row.get(3)?,
                row.get(4)?,
            ))
        })?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r?);
        }
        Ok(out)
    }

    // --- event replay / projection ---

    /// Replay a domain event into the materialized projection, idempotently.
    pub fn apply_event(&self, event: &EventEnvelope) -> Result<(), LocalIndexError> {
        // Idempotency: skip already-applied events.
        let already: Option<u8> = self
            .connection
            .query_row(
                "SELECT 1 FROM projection_events WHERE event_id = ?1",
                [event.event_id()],
                |row| row.get(0),
            )
            .optional()?;
        if already.is_some() {
            return Ok(());
        }

        self.connection.execute(
            "INSERT INTO projection_events(event_id, content_hash) VALUES (?1, ?2)",
            params![event.event_id(), event.content_hash()],
        )?;

        match event.payload() {
            sanket_domain::EventPayload::MemberCreated {
                member_id,
                display_name,
                role,
            } => {
                self.connection.execute(
                    "INSERT INTO members(id, display_name, role, masked_pan, status)
                     VALUES (?1, ?2, ?3, '[MASKED]', 'ACTIVE')
                     ON CONFLICT(id) DO UPDATE SET
                         display_name=excluded.display_name,
                         role=excluded.role",
                    params![member_id, display_name, role_to_str(*role)],
                )?;
            }
            sanket_domain::EventPayload::FriendAdded {
                friend_id,
                owner_member_id,
                share_basis_points,
                label,
            } => {
                self.connection.execute(
                    "INSERT INTO friend_accounts(
                         id, owner_member_id, label, masked_pan, share_basis_points, status
                     ) VALUES (?1, ?2, ?3, '[MASKED]', ?4, 'ACTIVE')
                     ON CONFLICT(id) DO UPDATE SET
                         owner_member_id=excluded.owner_member_id,
                         label=excluded.label,
                         share_basis_points=excluded.share_basis_points,
                         status='ACTIVE'",
                    params![friend_id, owner_member_id, label, share_basis_points],
                )?;
            }
            sanket_domain::EventPayload::FriendArchived { friend_id, .. } => {
                self.archive_friend(friend_id)?;
            }
            sanket_domain::EventPayload::InvestmentSessionCreated {
                session_id,
                actor_member_id,
                declared_capital_paise,
            } => {
                self.connection.execute(
                    "INSERT INTO investment_sessions(id, actor_member_id, declared_capital_paise, status)
                     VALUES (?1, ?2, ?3, 'OPEN')
                     ON CONFLICT(id) DO UPDATE SET declared_capital_paise=excluded.declared_capital_paise",
                    params![session_id, actor_member_id, declared_capital_paise],
                )?;
            }
            sanket_domain::EventPayload::IpoApplicationCreated {
                application_id,
                session_id,
                ipo_name,
                planned_amount_paise,
            } => {
                self.connection.execute(
                    "INSERT INTO applications(id, session_id, ipo_name, planned_amount_paise)
                     VALUES (?1, ?2, ?3, ?4)
                     ON CONFLICT(id) DO UPDATE SET ipo_name=excluded.ipo_name, planned_amount_paise=excluded.planned_amount_paise",
                    params![application_id, session_id, ipo_name, planned_amount_paise],
                )?;
                self.connection.execute(
                    "INSERT OR IGNORE INTO ipos(id, typed_name) VALUES (?1, ?2)",
                    params![application_id, ipo_name],
                )?;
            }
            sanket_domain::EventPayload::AllocationAdded {
                allocation_id,
                application_id,
                account_id,
                amount_paise,
                share_basis_points,
            } => {
                self.connection.execute(
                    "INSERT INTO allocations(id, application_id, account_id, amount_paise, share_basis_points)
                     VALUES (?1, ?2, ?3, ?4, ?5)
                     ON CONFLICT(id) DO UPDATE SET amount_paise=excluded.amount_paise",
                    params![allocation_id, application_id, account_id, amount_paise, share_basis_points],
                )?;
            }
            sanket_domain::EventPayload::InvestmentSessionSubmitted {
                session_id,
                recommendation_id,
            } => {
                self.connection.execute(
                    "UPDATE investment_sessions SET status='SUBMITTED', recommendation_id=?2 WHERE id=?1",
                    params![session_id, recommendation_id],
                )?;
            }
            sanket_domain::EventPayload::InvestmentRecommendationGenerated {
                session_id,
                algorithm_version,
            } => {
                use uuid::Uuid;
                let rec_id = Uuid::now_v7().to_string();
                self.connection.execute(
                    "INSERT INTO recommendations(session_id, algorithm_version, recommendation_id) VALUES (?1, ?2, ?3)",
                    params![session_id, algorithm_version, rec_id],
                )?;
            }
            sanket_domain::EventPayload::AllotmentJobCreated {
                job_id,
                application_id,
                session_id,
                ipo_name,
                registrar_id,
                provider_id,
            } => {
                self.connection.execute(
                    "INSERT INTO allotment_jobs(
                        id, application_id, session_id, ipo_name, registrar_id, registrar_name,
                        provider_id, status
                     ) VALUES (?1, ?2, ?3, ?4, ?5, ?5, ?6, 'CREATED')
                     ON CONFLICT(id) DO UPDATE SET status=excluded.status",
                    params![
                        job_id,
                        application_id,
                        session_id,
                        ipo_name,
                        registrar_id,
                        provider_id
                    ],
                )?;
            }
            sanket_domain::EventPayload::AllotmentJobStatusChanged { job_id, status } => {
                self.connection.execute(
                    "UPDATE allotment_jobs SET status=?2 WHERE id=?1",
                    params![job_id, status],
                )?;
            }
            sanket_domain::EventPayload::AllotmentAttemptRecorded {
                attempt_id,
                job_id,
                account_id,
                status,
                allotted_lots,
                allotted_shares,
                source,
                provider_reference,
            } => {
                self.connection.execute(
                    "INSERT INTO allotment_attempts(
                        id, job_id, account_id, status, attempt_count, allotted_lots, allotted_shares,
                        provider_reference, source
                     ) VALUES (?1, ?2, ?3, ?4, 1, ?5, ?6, ?7, ?8)
                     ON CONFLICT(id) DO UPDATE SET
                        status=excluded.status,
                        allotted_lots=excluded.allotted_lots,
                        allotted_shares=excluded.allotted_shares,
                        provider_reference=excluded.provider_reference,
                        source=excluded.source,
                        attempt_count=allotment_attempts.attempt_count + 1",
                    params![
                        attempt_id,
                        job_id,
                        account_id,
                        status,
                        allotted_lots.map(|v| v as i64),
                        allotted_shares.map(|v| v as i64),
                        provider_reference,
                        source
                    ],
                )?;
            }
            sanket_domain::EventPayload::InvestmentRecommendationApplied { .. }
            | sanket_domain::EventPayload::DeviceRegistered { .. }
            | sanket_domain::EventPayload::SettingsInitialized { .. }
            | sanket_domain::EventPayload::SensitiveIdentityAccessed { .. } => {
                // Projected elsewhere or audit-only; keep idempotent event mark.
            }
        }

        Ok(())
    }

    /// Rebuild the projection from scratch by replaying the given events.
    pub fn rebuild(&self, events: &[EventEnvelope]) -> Result<(), LocalIndexError> {
        self.connection.execute_batch(
            "DELETE FROM projection_events;
             DELETE FROM allotment_attempts;
             DELETE FROM allotment_jobs;
             DELETE FROM allocations;
             DELETE FROM applications;
             DELETE FROM ipos;
             DELETE FROM recommendations;
             DELETE FROM investment_sessions;
             DELETE FROM friend_accounts;
             DELETE FROM members;",
        )?;
        for event in events {
            self.apply_event(event)?;
        }
        Ok(())
    }

    pub fn list_sessions(&self) -> Result<Vec<(String, String, i64, String)>, LocalIndexError> {
        let mut stmt = self.connection.prepare(
            "SELECT id, actor_member_id, declared_capital_paise, status FROM investment_sessions ORDER BY id",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
        })?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r?);
        }
        Ok(out)
    }

    pub fn list_allocations(
        &self,
        session_id: &str,
    ) -> Result<Vec<AllocationProjectionRow>, LocalIndexError> {
        let mut stmt = self.connection.prepare(
            "SELECT a.id, a.application_id, a.account_id, a.amount_paise, a.share_basis_points
             FROM allocations a
             JOIN applications app ON a.application_id = app.id
             WHERE app.session_id = ?1
             ORDER BY a.id",
        )?;
        let rows = stmt.query_map([session_id], |row| {
            Ok((
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                row.get(3)?,
                row.get(4)?,
            ))
        })?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r?);
        }
        Ok(out)
    }

    pub fn list_recommendations(&self) -> Result<Vec<(String, String, String)>, LocalIndexError> {
        let mut stmt = self.connection.prepare(
            "SELECT session_id, algorithm_version, recommendation_id FROM recommendations ORDER BY session_id",
        )?;
        let rows = stmt.query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r?);
        }
        Ok(out)
    }

    /// Submitted IPO applications eligible for allotment checks.
    pub fn list_submitted_applications(
        &self,
    ) -> Result<Vec<SubmittedApplicationRow>, LocalIndexError> {
        let mut stmt = self.connection.prepare(
            "SELECT app.id, app.session_id, app.ipo_name, app.planned_amount_paise,
                    (SELECT COUNT(*) FROM allocations a WHERE a.application_id = app.id)
             FROM applications app
             JOIN investment_sessions s ON s.id = app.session_id
             WHERE s.status = 'SUBMITTED'
             ORDER BY app.id",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok((
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                row.get(3)?,
                row.get::<_, i64>(4)? as u32,
            ))
        })?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r?);
        }
        Ok(out)
    }

    pub fn list_account_ids_for_application(
        &self,
        application_id: &str,
    ) -> Result<Vec<String>, LocalIndexError> {
        let mut stmt = self.connection.prepare(
            "SELECT DISTINCT account_id FROM allocations WHERE application_id = ?1 ORDER BY account_id",
        )?;
        let rows = stmt.query_map([application_id], |row| row.get(0))?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r?);
        }
        Ok(out)
    }

    pub fn list_allotment_jobs(&self) -> Result<Vec<AllotmentJobRow>, LocalIndexError> {
        let mut stmt = self.connection.prepare(
            "SELECT id, application_id, session_id, ipo_name, registrar_id, status
             FROM allotment_jobs ORDER BY id",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok((
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                row.get(3)?,
                row.get(4)?,
                row.get(5)?,
            ))
        })?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r?);
        }
        Ok(out)
    }

    pub fn list_allotment_attempts(
        &self,
        job_id: &str,
    ) -> Result<Vec<AllotmentAttemptRow>, LocalIndexError> {
        let mut stmt = self.connection.prepare(
            "SELECT id, account_id, status, allotted_lots, allotted_shares, source
             FROM allotment_attempts WHERE job_id = ?1 ORDER BY id",
        )?;
        let rows = stmt.query_map([job_id], |row| {
            Ok((
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                row.get(3)?,
                row.get(4)?,
                row.get(5)?,
            ))
        })?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r?);
        }
        Ok(out)
    }

    pub fn masked_pan_for_account(
        &self,
        account_id: &str,
    ) -> Result<Option<String>, LocalIndexError> {
        let m: Option<String> = self
            .connection
            .query_row(
                "SELECT masked_pan FROM members WHERE id = ?1
                 UNION
                 SELECT masked_pan FROM friend_accounts WHERE id = ?1
                 LIMIT 1",
                [account_id],
                |row| row.get(0),
            )
            .optional()?;
        Ok(m)
    }

    pub fn display_label_for_account(
        &self,
        account_id: &str,
    ) -> Result<(String, String), LocalIndexError> {
        if let Some(row) = self
            .connection
            .query_row(
                "SELECT display_name, 'PRIMARY' FROM members WHERE id = ?1",
                [account_id],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
            )
            .optional()?
        {
            return Ok(row);
        }
        if let Some(row) = self
            .connection
            .query_row(
                "SELECT label, 'FRIEND' FROM friend_accounts WHERE id = ?1",
                [account_id],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
            )
            .optional()?
        {
            return Ok(row);
        }
        Ok((account_id.to_owned(), "UNKNOWN".into()))
    }
}

fn role_to_str(role: sanket_domain::Role) -> &'static str {
    match role {
        sanket_domain::Role::Owner => "OWNER",
        sanket_domain::Role::CoreMember => "CORE_MEMBER",
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
