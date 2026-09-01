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

// Schema v4: durable worker ownership/retry state plus safe public provider and
// estimated-profit projections. Sensitive identity values are never stored.
const SCHEMA_V4: &str = r#"
BEGIN;
ALTER TABLE allotment_jobs ADD COLUMN actor_member_id TEXT NOT NULL DEFAULT '';
ALTER TABLE allotment_jobs ADD COLUMN lease_owner_device_id TEXT;
ALTER TABLE allotment_jobs ADD COLUMN lease_token TEXT;
ALTER TABLE allotment_jobs ADD COLUMN lease_expires_at INTEGER;
ALTER TABLE allotment_jobs ADD COLUMN cancel_requested INTEGER NOT NULL DEFAULT 0;
ALTER TABLE allotment_jobs ADD COLUMN created_at TEXT NOT NULL DEFAULT '';
ALTER TABLE allotment_jobs ADD COLUMN updated_at TEXT NOT NULL DEFAULT '';

CREATE TABLE IF NOT EXISTS provider_issue_mappings (
    application_id TEXT NOT NULL,
    provider_id TEXT NOT NULL,
    registrar_id TEXT NOT NULL,
    provider_issue_id TEXT NOT NULL,
    ipo_name TEXT NOT NULL,
    official_status_url TEXT NOT NULL,
    last_verified_at TEXT NOT NULL,
    PRIMARY KEY(application_id, provider_id)
);

CREATE TABLE IF NOT EXISTS provider_health (
    provider_id TEXT PRIMARY KEY,
    status TEXT NOT NULL,
    checked_at TEXT NOT NULL,
    safe_message TEXT
);

CREATE TABLE IF NOT EXISTS estimated_profit_bases (
    application_id TEXT NOT NULL,
    account_id TEXT NOT NULL,
    basis TEXT NOT NULL,
    reference_price_paise INTEGER,
    issue_price_paise INTEGER,
    allotted_shares INTEGER NOT NULL,
    estimated_profit_paise INTEGER,
    provenance TEXT,
    observed_at TEXT NOT NULL,
    PRIMARY KEY(application_id, account_id)
);

INSERT OR IGNORE INTO schema_migrations(version) VALUES (4);
COMMIT;
"#;

// Schema v5: safe human-verification continuation metadata. Provider session state stays ephemeral.
const SCHEMA_V5: &str = r#"
BEGIN;
CREATE TABLE IF NOT EXISTS provider_challenges (
    id TEXT PRIMARY KEY,
    job_id TEXT NOT NULL,
    attempt_id TEXT NOT NULL,
    account_id TEXT NOT NULL,
    provider_id TEXT NOT NULL,
    challenge_type TEXT NOT NULL,
    status TEXT NOT NULL,
    endpoint_id TEXT NOT NULL,
    continuation_reference TEXT,
    created_at TEXT NOT NULL,
    expires_at TEXT
);
CREATE INDEX IF NOT EXISTS idx_provider_challenges_job ON provider_challenges(job_id);
INSERT OR IGNORE INTO schema_migrations(version) VALUES (5);
COMMIT;
"#;

// Schema v6: public registrar metadata captured with the submitted application.
const SCHEMA_V6: &str = r#"
BEGIN;
ALTER TABLE applications ADD COLUMN registrar_id TEXT NOT NULL DEFAULT '';
ALTER TABLE applications ADD COLUMN registrar_name TEXT NOT NULL DEFAULT '';
ALTER TABLE applications ADD COLUMN official_status_url TEXT;
ALTER TABLE applications ADD COLUMN expected_allotment_date TEXT;
INSERT OR IGNORE INTO schema_migrations(version) VALUES (6);
COMMIT;
"#;

const SCHEMA_V7: &str = r#"
BEGIN;
ALTER TABLE applications ADD COLUMN source TEXT NOT NULL DEFAULT 'OWNER_CURRENT_ENTRY';
ALTER TABLE applications ADD COLUMN application_date TEXT;
ALTER TABLE applications ADD COLUMN created_at TEXT NOT NULL DEFAULT '';
INSERT OR IGNORE INTO schema_migrations(version) VALUES (7);
COMMIT;
"#;

// Schema v8: safe public Upstox metadata captured with current applications.
const SCHEMA_V8: &str = r#"
BEGIN;
ALTER TABLE applications ADD COLUMN metadata_json TEXT;
INSERT OR IGNORE INTO schema_migrations(version) VALUES (8);
COMMIT;
"#;

pub struct LocalIndex {
    connection: Connection,
}

pub type FriendProjectionRow = (String, String, String, String, i64);
pub type AllocationProjectionRow = (String, String, String, i64, i64);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SubmittedApplicationState {
    pub application_id: String,
    pub session_id: String,
    pub ipo_name: String,
    pub planned_amount_paise: i64,
    pub account_count: u32,
    pub registrar_id: String,
    pub registrar_name: String,
    pub official_status_url: Option<String>,
    pub expected_allotment_date: Option<String>,
}
pub type AllotmentJobRow = (String, String, String, String, String, String);
pub type AllotmentAttemptRow = (String, String, String, Option<i64>, Option<i64>, String);
pub type EstimatedProfitBase = (String, Option<i64>, Option<String>);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AllotmentJobExecutionRow {
    pub id: String,
    pub application_id: String,
    pub session_id: String,
    pub ipo_name: String,
    pub registrar_id: String,
    pub registrar_name: String,
    pub official_status_url: Option<String>,
    pub provider_id: String,
    pub status: String,
    pub actor_member_id: String,
    pub cancel_requested: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AllotmentAttemptState {
    pub id: String,
    pub job_id: String,
    pub account_id: String,
    pub status: String,
    pub attempt_count: u32,
    pub allotted_lots: Option<i64>,
    pub allotted_shares: Option<i64>,
    pub provider_reference: Option<String>,
    pub safe_message: Option<String>,
    pub source: String,
    pub last_attempt_at: Option<String>,
    pub next_retry_at: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProviderChallengeState {
    pub id: String,
    pub job_id: String,
    pub attempt_id: String,
    pub account_id: String,
    pub provider_id: String,
    pub challenge_type: String,
    pub status: String,
    pub endpoint_id: String,
    pub continuation_reference: Option<String>,
    pub created_at: String,
    pub expires_at: Option<String>,
}

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
    #[error("metadata serialization failed: {0}")]
    Metadata(String),
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
        let version: u32 = connection.query_row(
            "SELECT COALESCE(MAX(version), 0) FROM schema_migrations",
            [],
            |row| row.get(0),
        )?;
        if version < 4 {
            connection.execute_batch(SCHEMA_V4)?;
        }
        let version: u32 = connection.query_row(
            "SELECT COALESCE(MAX(version), 0) FROM schema_migrations",
            [],
            |row| row.get(0),
        )?;
        if version < 5 {
            connection.execute_batch(SCHEMA_V5)?;
        }
        let version: u32 = connection.query_row(
            "SELECT COALESCE(MAX(version), 0) FROM schema_migrations",
            [],
            |row| row.get(0),
        )?;
        if version < 6 {
            connection.execute_batch(SCHEMA_V6)?;
        }
        let version: u32 = connection.query_row(
            "SELECT COALESCE(MAX(version), 0) FROM schema_migrations",
            [],
            |row| row.get(0),
        )?;
        if version < 7 {
            connection.execute_batch(SCHEMA_V7)?;
        }
        let version: u32 = connection.query_row(
            "SELECT COALESCE(MAX(version), 0) FROM schema_migrations",
            [],
            |row| row.get(0),
        )?;
        if version < 8 {
            connection.execute_batch(SCHEMA_V8)?;
        }
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
                registrar_id,
                registrar_name,
                official_status_url,
                expected_allotment_date,
                source,
                application_date,
                metadata,
            } => {
                let metadata_json = metadata
                    .as_ref()
                    .map(serde_json::to_string)
                    .transpose()
                    .map_err(|error| LocalIndexError::Metadata(error.to_string()))?;
                self.connection.execute(
                    "INSERT INTO applications(
                         id, session_id, ipo_name, planned_amount_paise, registrar_id,
                         registrar_name, official_status_url, expected_allotment_date,
                         source, application_date, created_at, metadata_json
                     ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
                     ON CONFLICT(id) DO UPDATE SET
                         ipo_name=excluded.ipo_name,
                         planned_amount_paise=excluded.planned_amount_paise,
                         registrar_id=excluded.registrar_id,
                         registrar_name=excluded.registrar_name,
                         official_status_url=excluded.official_status_url,
                         expected_allotment_date=excluded.expected_allotment_date,
                         source=excluded.source,
                         application_date=excluded.application_date,
                         metadata_json=excluded.metadata_json",
                    params![
                        application_id,
                        session_id,
                        ipo_name,
                        planned_amount_paise,
                        registrar_id,
                        registrar_name,
                        official_status_url,
                        expected_allotment_date,
                        source,
                        application_date,
                        event.occurred_at(),
                        metadata_json
                    ],
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
            sanket_domain::EventPayload::InvestmentSessionVoided { session_id, .. } => {
                // Only submitted sessions move to VOIDED; already-voided stays voided (idempotent).
                self.connection.execute(
                    "UPDATE investment_sessions SET status='VOIDED'
                     WHERE id=?1 AND status IN ('SUBMITTED', 'VOIDED')",
                    params![session_id],
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
                registrar_name,
                official_status_url,
                provider_id,
            } => {
                self.connection.execute(
                    "INSERT INTO allotment_jobs(
                        id, application_id, session_id, ipo_name, registrar_id, registrar_name,
                        official_status_url, provider_id, status, actor_member_id, created_at, updated_at
                     ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 'CREATED', ?9, ?10, ?10)
                     ON CONFLICT(id) DO UPDATE SET status=excluded.status",
                    params![
                        job_id,
                        application_id,
                        session_id,
                        ipo_name,
                        registrar_id,
                        registrar_name,
                        official_status_url,
                        provider_id,
                        event.actor_member_id(),
                        event.occurred_at()
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
            sanket_domain::EventPayload::AllotmentAttemptStateUpdated {
                attempt_id,
                job_id,
                account_id,
                status,
                attempt_count,
                allotted_lots,
                allotted_shares,
                source,
                provider_reference,
                safe_message,
                last_attempt_at,
                next_retry_at,
            } => {
                self.connection.execute(
                    "INSERT INTO allotment_attempts(
                        id, job_id, account_id, status, attempt_count, allotted_lots,
                        allotted_shares, provider_reference, safe_message, source,
                        last_attempt_at, next_retry_at
                     ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
                     ON CONFLICT(id) DO UPDATE SET
                        status=excluded.status,
                        attempt_count=excluded.attempt_count,
                        allotted_lots=excluded.allotted_lots,
                        allotted_shares=excluded.allotted_shares,
                        provider_reference=excluded.provider_reference,
                        safe_message=excluded.safe_message,
                        source=excluded.source,
                        last_attempt_at=excluded.last_attempt_at,
                        next_retry_at=excluded.next_retry_at",
                    params![
                        attempt_id,
                        job_id,
                        account_id,
                        status,
                        *attempt_count as i64,
                        allotted_lots.map(|v| v as i64),
                        allotted_shares.map(|v| v as i64),
                        provider_reference,
                        safe_message,
                        source,
                        last_attempt_at,
                        next_retry_at
                    ],
                )?;
            }
            sanket_domain::EventPayload::AllotmentProviderChallengeUpdated {
                challenge_id,
                job_id,
                attempt_id,
                account_id,
                provider_id,
                challenge_type,
                status,
                endpoint_id,
                continuation_reference,
                created_at,
                expires_at,
            } => {
                self.connection.execute(
                    "INSERT INTO provider_challenges(
                        id, job_id, attempt_id, account_id, provider_id, challenge_type,
                        status, endpoint_id, continuation_reference, created_at, expires_at
                     ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
                     ON CONFLICT(id) DO UPDATE SET
                        status=excluded.status,
                        endpoint_id=excluded.endpoint_id,
                        continuation_reference=excluded.continuation_reference,
                        expires_at=excluded.expires_at",
                    params![
                        challenge_id,
                        job_id,
                        attempt_id,
                        account_id,
                        provider_id,
                        challenge_type,
                        status,
                        endpoint_id,
                        continuation_reference,
                        created_at,
                        expires_at
                    ],
                )?;
            }
            sanket_domain::EventPayload::AllotmentProviderDiscovered {
                application_id,
                registrar_id,
                provider_id,
                provider_issue_id,
                ipo_name,
                official_status_url,
                last_verified_at,
            } => {
                self.connection.execute(
                    "INSERT INTO provider_issue_mappings(
                        application_id, provider_id, registrar_id, provider_issue_id,
                        ipo_name, official_status_url, last_verified_at
                     ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
                     ON CONFLICT(application_id, provider_id) DO UPDATE SET
                        registrar_id=excluded.registrar_id,
                        provider_issue_id=excluded.provider_issue_id,
                        ipo_name=excluded.ipo_name,
                        official_status_url=excluded.official_status_url,
                        last_verified_at=excluded.last_verified_at",
                    params![
                        application_id,
                        provider_id,
                        registrar_id,
                        provider_issue_id,
                        ipo_name,
                        official_status_url,
                        last_verified_at
                    ],
                )?;
            }
            sanket_domain::EventPayload::EstimatedProfitUpdated {
                application_id,
                account_id,
                basis,
                reference_price_paise,
                issue_price_paise,
                allotted_shares,
                estimated_profit_paise,
                provenance,
                observed_at,
            } => {
                self.connection.execute(
                    "INSERT INTO estimated_profit_bases(
                        application_id, account_id, basis, reference_price_paise,
                        issue_price_paise, allotted_shares, estimated_profit_paise,
                        provenance, observed_at
                     ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
                     ON CONFLICT(application_id, account_id) DO UPDATE SET
                        basis=excluded.basis,
                        reference_price_paise=excluded.reference_price_paise,
                        issue_price_paise=excluded.issue_price_paise,
                        allotted_shares=excluded.allotted_shares,
                        estimated_profit_paise=excluded.estimated_profit_paise,
                        provenance=excluded.provenance,
                        observed_at=excluded.observed_at",
                    params![
                        application_id,
                        account_id,
                        basis,
                        reference_price_paise,
                        issue_price_paise,
                        *allotted_shares as i64,
                        estimated_profit_paise,
                        provenance,
                        observed_at
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
             DELETE FROM estimated_profit_bases;
             DELETE FROM provider_health;
             DELETE FROM provider_issue_mappings;
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
    ) -> Result<Vec<SubmittedApplicationState>, LocalIndexError> {
        let mut stmt = self.connection.prepare(
            "SELECT app.id, app.session_id, app.ipo_name, app.planned_amount_paise,
                    (SELECT COUNT(*) FROM allocations a WHERE a.application_id = app.id),
                    app.registrar_id, app.registrar_name, app.official_status_url,
                    app.expected_allotment_date
             FROM applications app
             JOIN investment_sessions s ON s.id = app.session_id
             WHERE s.status = 'SUBMITTED'
             ORDER BY app.id",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(SubmittedApplicationState {
                application_id: row.get(0)?,
                session_id: row.get(1)?,
                ipo_name: row.get(2)?,
                planned_amount_paise: row.get(3)?,
                account_count: row.get::<_, i64>(4)? as u32,
                registrar_id: row.get(5)?,
                registrar_name: row.get(6)?,
                official_status_url: row.get(7)?,
                expected_allotment_date: row.get(8)?,
            })
        })?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r?);
        }
        Ok(out)
    }

    pub fn historical_application_exists(
        &self,
        actor_member_id: &str,
        ipo_name: &str,
        provider_id: &str,
        provider_issue_id: &str,
    ) -> Result<bool, LocalIndexError> {
        let exists = self
            .connection
            .query_row(
                "SELECT 1
                 FROM applications a
                 JOIN investment_sessions s ON s.id = a.session_id
                 JOIN provider_issue_mappings p ON p.application_id = a.id
                 WHERE s.actor_member_id = ?1
                   AND lower(trim(a.ipo_name)) = lower(trim(?2))
                   AND p.provider_id = ?3
                   AND p.provider_issue_id = ?4
                 LIMIT 1",
                params![actor_member_id, ipo_name, provider_id, provider_issue_id],
                |row| row.get::<_, u8>(0),
            )
            .optional()?
            .is_some();
        Ok(exists)
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

    pub fn allocation_amount_for_account(
        &self,
        application_id: &str,
        account_id: &str,
    ) -> Result<Option<i64>, LocalIndexError> {
        self.connection
            .query_row(
                "SELECT amount_paise FROM allocations WHERE application_id=?1 AND account_id=?2 LIMIT 1",
                params![application_id, account_id],
                |row| row.get(0),
            )
            .optional()
            .map_err(LocalIndexError::from)
    }

    /// `(basis, estimated_profit_paise, provenance)`.
    pub fn estimated_profit_for_account(
        &self,
        application_id: &str,
        account_id: &str,
    ) -> Result<Option<EstimatedProfitBase>, LocalIndexError> {
        self.connection
            .query_row(
                "SELECT basis, estimated_profit_paise, provenance
                 FROM estimated_profit_bases WHERE application_id=?1 AND account_id=?2",
                params![application_id, account_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .optional()
            .map_err(LocalIndexError::from)
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

    pub fn allotment_attempt(
        &self,
        job_id: &str,
        account_id: &str,
    ) -> Result<Option<AllotmentAttemptState>, LocalIndexError> {
        self.connection
            .query_row(
                "SELECT id, job_id, account_id, status, attempt_count, allotted_lots,
                        allotted_shares, provider_reference, safe_message, source,
                        last_attempt_at, next_retry_at
                 FROM allotment_attempts WHERE job_id=?1 AND account_id=?2
                 ORDER BY id LIMIT 1",
                params![job_id, account_id],
                |row| {
                    Ok(AllotmentAttemptState {
                        id: row.get(0)?,
                        job_id: row.get(1)?,
                        account_id: row.get(2)?,
                        status: row.get(3)?,
                        attempt_count: row.get::<_, i64>(4)? as u32,
                        allotted_lots: row.get(5)?,
                        allotted_shares: row.get(6)?,
                        provider_reference: row.get(7)?,
                        safe_message: row.get(8)?,
                        source: row.get(9)?,
                        last_attempt_at: row.get(10)?,
                        next_retry_at: row.get(11)?,
                    })
                },
            )
            .optional()
            .map_err(LocalIndexError::from)
    }

    /// Atomically acquire a job lease. Expired leases are reclaimable after restart.
    pub fn try_acquire_allotment_lease(
        &self,
        job_id: &str,
        owner_device_id: &str,
        lease_token: &str,
        now_epoch_secs: u64,
        expires_at_epoch_secs: u64,
    ) -> Result<bool, LocalIndexError> {
        let changed = self.connection.execute(
            "UPDATE allotment_jobs SET
                lease_owner_device_id=?2, lease_token=?3, lease_expires_at=?5,
                status='RUNNING', updated_at=CAST(?4 AS TEXT)
             WHERE id=?1 AND cancel_requested=0
               AND status IN ('CREATED','PREPARING_PROVIDER_SESSION','VERIFICATION_REQUIRED_REFRESH','RUNNING','PARTIALLY_COMPLETE','WAITING_FOR_PROVIDER_AVAILABILITY')
               AND (lease_token IS NULL OR lease_expires_at IS NULL OR lease_expires_at <= ?4)",
            params![
                job_id,
                owner_device_id,
                lease_token,
                now_epoch_secs as i64,
                expires_at_epoch_secs as i64
            ],
        )?;
        Ok(changed == 1)
    }

    pub fn release_allotment_lease(
        &self,
        job_id: &str,
        lease_token: &str,
    ) -> Result<bool, LocalIndexError> {
        Ok(self.connection.execute(
            "UPDATE allotment_jobs SET lease_owner_device_id=NULL, lease_token=NULL,
                    lease_expires_at=NULL
             WHERE id=?1 AND lease_token=?2",
            params![job_id, lease_token],
        )? == 1)
    }

    pub fn request_allotment_cancel(&self, job_id: &str) -> Result<bool, LocalIndexError> {
        let tx = self.connection.unchecked_transaction()?;
        let changed = tx.execute(
            "UPDATE allotment_jobs SET cancel_requested=1, status='CANCELLED', updated_at=CURRENT_TIMESTAMP
             WHERE id=?1 AND status NOT IN ('COMPLETE','CANCELLED')",
            [job_id],
        )?;
        if changed == 1 {
            tx.execute(
                "UPDATE allotment_attempts SET status='CANCELLED', next_retry_at=NULL
                 WHERE job_id=?1 AND status NOT IN ('ALLOTTED','NOT_ALLOTTED','NOT_FOUND','MANUAL_RESULT')",
                [job_id],
            )?;
        }
        tx.commit()?;
        Ok(changed == 1)
    }

    pub fn resumable_allotment_job_ids(
        &self,
        now_epoch_secs: u64,
    ) -> Result<Vec<String>, LocalIndexError> {
        let mut stmt = self.connection.prepare(
            "SELECT id FROM allotment_jobs
             WHERE cancel_requested=0
               AND status IN ('CREATED','PREPARING_PROVIDER_SESSION','VERIFICATION_REQUIRED_REFRESH','RUNNING','PARTIALLY_COMPLETE','WAITING_FOR_PROVIDER_AVAILABILITY')
               AND (lease_token IS NULL OR lease_expires_at IS NULL OR lease_expires_at <= ?1)
             ORDER BY created_at, id",
        )?;
        let rows = stmt.query_map([now_epoch_secs as i64], |row| row.get(0))?;
        rows.collect::<std::result::Result<Vec<_>, _>>()
            .map_err(LocalIndexError::from)
    }

    /// Drop stale continuation handles after process restart while preserving durable jobs.
    pub fn reconcile_ephemeral_allotment_state_after_restart(&self) -> Result<(), LocalIndexError> {
        let tx = self.connection.unchecked_transaction()?;
        tx.execute(
            "UPDATE allotment_jobs SET
                status='PREPARING_PROVIDER_SESSION',
                lease_owner_device_id=NULL, lease_token=NULL, lease_expires_at=NULL,
                updated_at=CURRENT_TIMESTAMP
             WHERE status IN ('RUNNING','PARTIALLY_COMPLETE','WAITING_FOR_PROVIDER_AVAILABILITY',
                              'PREPARING_PROVIDER_SESSION')
               AND NOT EXISTS (
                    SELECT 1 FROM provider_challenges c
                    WHERE c.job_id=allotment_jobs.id AND c.status IN ('REQUIRED','PRESENTED')
               )",
            [],
        )?;
        tx.execute(
            "UPDATE allotment_jobs SET
                status='VERIFICATION_REQUIRED_REFRESH',
                lease_owner_device_id=NULL, lease_token=NULL, lease_expires_at=NULL,
                updated_at=CURRENT_TIMESTAMP
             WHERE id IN (
                SELECT job_id FROM provider_challenges WHERE status IN ('REQUIRED','PRESENTED')
             )",
            [],
        )?;
        tx.execute(
            "UPDATE allotment_attempts SET
                status='VERIFICATION_REQUIRED_REFRESH', next_retry_at=NULL
             WHERE id IN (
                SELECT attempt_id FROM provider_challenges WHERE status IN ('REQUIRED','PRESENTED')
             ) AND status NOT IN ('ALLOTTED','NOT_ALLOTTED','NOT_FOUND','MANUAL_RESULT')",
            [],
        )?;
        tx.execute(
            "UPDATE provider_challenges SET status='EXPIRED', continuation_reference=NULL
             WHERE status IN ('REQUIRED','PRESENTED')",
            [],
        )?;
        tx.commit()?;
        Ok(())
    }

    pub fn provider_challenge(
        &self,
        challenge_id: &str,
    ) -> Result<Option<ProviderChallengeState>, LocalIndexError> {
        self.connection
            .query_row(
                "SELECT id, job_id, attempt_id, account_id, provider_id, challenge_type,
                        status, endpoint_id, continuation_reference, created_at, expires_at
                 FROM provider_challenges WHERE id=?1",
                [challenge_id],
                |row| {
                    Ok(ProviderChallengeState {
                        id: row.get(0)?,
                        job_id: row.get(1)?,
                        attempt_id: row.get(2)?,
                        account_id: row.get(3)?,
                        provider_id: row.get(4)?,
                        challenge_type: row.get(5)?,
                        status: row.get(6)?,
                        endpoint_id: row.get(7)?,
                        continuation_reference: row.get(8)?,
                        created_at: row.get(9)?,
                        expires_at: row.get(10)?,
                    })
                },
            )
            .optional()
            .map_err(LocalIndexError::from)
    }

    pub fn allotment_job_execution(
        &self,
        job_id: &str,
    ) -> Result<Option<AllotmentJobExecutionRow>, LocalIndexError> {
        self.connection
            .query_row(
                "SELECT id, application_id, session_id, ipo_name, registrar_id,
                        registrar_name, official_status_url, provider_id, status,
                        actor_member_id, cancel_requested
                 FROM allotment_jobs WHERE id=?1",
                [job_id],
                |row| {
                    Ok(AllotmentJobExecutionRow {
                        id: row.get(0)?,
                        application_id: row.get(1)?,
                        session_id: row.get(2)?,
                        ipo_name: row.get(3)?,
                        registrar_id: row.get(4)?,
                        registrar_name: row.get(5)?,
                        official_status_url: row.get(6)?,
                        provider_id: row.get(7)?,
                        status: row.get(8)?,
                        actor_member_id: row.get(9)?,
                        cancel_requested: row.get::<_, i64>(10)? != 0,
                    })
                },
            )
            .optional()
            .map_err(LocalIndexError::from)
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
