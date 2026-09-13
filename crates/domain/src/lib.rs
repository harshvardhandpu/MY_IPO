use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

mod investment;
mod members;
mod money;

pub use investment::{
    InvestmentAllocation, InvestmentError, InvestmentSession, IpoApplication, IpoMetadataSnapshot,
    SessionStatus,
};
pub use members::{CoreMember, FriendAccount, FriendShareError, MemberStatus};
pub use money::{BasisPoints, BasisPointsError, Money};

pub const EVENT_SCHEMA_VERSION: u16 = 1;

fn default_application_source() -> String {
    "OWNER_CURRENT_ENTRY".to_owned()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Role {
    Owner,
    Admin,
    CoreMember,
}

impl Role {
    pub const fn can_manage_members(self) -> bool {
        matches!(self, Self::Owner | Self::Admin)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SyncStatus {
    Synced,
    Syncing,
    Pending,
    Offline,
    Conflict,
    AuthenticationRequired,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum EventPayload {
    DeviceRegistered {
        device_label: String,
    },
    SettingsInitialized {
        settings_version: u16,
    },
    /// Authentication verifier metadata for the first local owner account.
    /// The verifier is a salted password hash, never a plaintext password.
    OwnerBootstrapped {
        account_id: String,
        email: String,
        verifier_version: u16,
        verifier: String,
    },
    /// A one-time invite is issued. Only the digest is authoritative/persisted.
    InviteIssued {
        invite_id: String,
        account_id: String,
        email: String,
        invite_digest: String,
        expires_at: String,
    },
    /// A signup submitted a verifier for an invited account; the account remains pending.
    SignupPending {
        account_id: String,
        invite_id: String,
        email: String,
        verifier_version: u16,
        verifier: String,
    },
    /// An owner/admin approved a pending account.
    AccountApproved {
        account_id: String,
        approved_by: String,
    },
    /// An owner/admin revoked an account.
    AccountRevoked {
        account_id: String,
        revoked_by: String,
    },
    /// A session was authenticated; only a digest of the ephemeral token is persisted.
    SessionStarted {
        session_hash: String,
        account_id: String,
        expires_at: String,
    },
    /// A session was logged out; only a digest of the ephemeral token is persisted.
    SessionLoggedOut {
        session_hash: String,
        account_id: String,
    },
    SensitiveIdentityAccessed {
        account_id: String,
        purpose: String,
    },
    /// Owner granted one bounded real-investor lookup. Contains no PAN or credential.
    LookupAuthorizationGranted {
        authorization_id: String,
        application_id: String,
        provider_id: String,
        expiry_time: String,
    },
    /// The bounded lookup authorization was consumed immediately before PAN access.
    LookupAuthorizationConsumed {
        authorization_id: String,
        application_id: String,
        provider_id: String,
        #[serde(default)]
        execution_id: String,
        #[serde(default)]
        account_ids: Vec<String>,
        timestamp: String,
    },
    /// Member onboarded. Carries display name only — never PAN/UPI/email.
    MemberCreated {
        member_id: String,
        display_name: String,
        role: Role,
    },
    /// Member-wide notification: a friend account was added. Carries no PAN.
    FriendAdded {
        friend_id: String,
        owner_member_id: String,
        label: String,
        share_basis_points: i64,
    },
    /// Member-wide notification: a friend account was archived (never deleted).
    FriendArchived {
        friend_id: String,
        owner_member_id: String,
    },
    /// An investment session was opened with a positive declared capital.
    InvestmentSessionCreated {
        session_id: String,
        actor_member_id: String,
        declared_capital_paise: i64,
    },
    /// An IPO application was added to a session.
    IpoApplicationCreated {
        application_id: String,
        session_id: String,
        ipo_name: String,
        planned_amount_paise: i64,
        #[serde(default)]
        registrar_id: String,
        #[serde(default)]
        registrar_name: String,
        #[serde(default)]
        official_status_url: Option<String>,
        #[serde(default)]
        expected_allotment_date: Option<String>,
        #[serde(default = "default_application_source")]
        source: String,
        #[serde(default)]
        application_date: Option<String>,
        #[serde(default)]
        #[serde(skip_serializing_if = "Option::is_none")]
        metadata: Option<Box<IpoMetadataSnapshot>>,
    },
    /// A final allocation was added (account-scoped; no PAN).
    AllocationAdded {
        allocation_id: String,
        application_id: String,
        account_id: String,
        amount_paise: i64,
        share_basis_points: i64,
    },
    /// The session was submitted. References an optional recommendation.
    InvestmentSessionSubmitted {
        session_id: String,
        recommendation_id: Option<String>,
    },
    /// Owner voided a submitted session (correction). Session remains auditable; excluded from active totals.
    InvestmentSessionVoided {
        session_id: String,
        reason: String,
    },
    /// A recommendation was generated (algorithm-version tagged, draft only).
    InvestmentRecommendationGenerated {
        session_id: String,
        algorithm_version: String,
    },
    /// A recommendation was applied to the draft (does not auto-submit).
    InvestmentRecommendationApplied {
        session_id: String,
        recommendation_id: String,
    },
    /// Allotment check job created for one IPO/application (no PAN).
    AllotmentJobCreated {
        job_id: String,
        application_id: String,
        session_id: String,
        ipo_name: String,
        registrar_id: String,
        #[serde(default)]
        registrar_name: String,
        #[serde(default)]
        official_status_url: Option<String>,
        provider_id: String,
    },
    /// Job status advanced.
    AllotmentJobStatusChanged {
        job_id: String,
        status: String,
    },
    /// Per-account attempt result (account_id only; never PAN).
    AllotmentAttemptRecorded {
        attempt_id: String,
        job_id: String,
        account_id: String,
        status: String,
        allotted_lots: Option<u32>,
        allotted_shares: Option<u64>,
        source: String,
        provider_reference: Option<String>,
    },
    /// Durable retry/progress state for one account attempt (never PAN).
    AllotmentAttemptStateUpdated {
        attempt_id: String,
        job_id: String,
        account_id: String,
        status: String,
        attempt_count: u32,
        allotted_lots: Option<u32>,
        allotted_shares: Option<u64>,
        source: String,
        provider_reference: Option<String>,
        safe_message: Option<String>,
        last_attempt_at: String,
        next_retry_at: Option<String>,
    },
    /// A definitive allotment fact, separated from provider-attempt history.
    /// Contains provenance and account identifiers only; never PAN or secrets.
    AllotmentResolutionFactRecorded {
        fact_id: String,
        job_id: String,
        account_id: String,
        outcome: String,
        source: String,
        allotted_lots: Option<u32>,
        allotted_shares: Option<u64>,
        provider_reference: Option<String>,
        provenance: String,
        supersedes_attempt_id: Option<String>,
    },
    /// Safe durable metadata for resumable human verification. Never session secrets or PAN.
    AllotmentProviderChallengeUpdated {
        challenge_id: String,
        job_id: String,
        attempt_id: String,
        account_id: String,
        provider_id: String,
        challenge_type: String,
        status: String,
        endpoint_id: String,
        continuation_reference: Option<String>,
        created_at: String,
        expires_at: Option<String>,
    },
    /// Public registrar issue mapping discovered or revalidated (never PAN).
    AllotmentProviderDiscovered {
        application_id: String,
        registrar_id: String,
        provider_id: String,
        provider_issue_id: String,
        ipo_name: String,
        official_status_url: String,
        last_verified_at: String,
    },
    /// Explicit estimated-profit basis; this is never realized profit.
    EstimatedProfitUpdated {
        application_id: String,
        account_id: String,
        basis: String,
        reference_price_paise: Option<i64>,
        issue_price_paise: Option<i64>,
        allotted_shares: u64,
        estimated_profit_paise: Option<i64>,
        provenance: Option<String>,
        observed_at: String,
    },
}

impl EventPayload {
    fn event_type(&self) -> &'static str {
        match self {
            Self::DeviceRegistered { .. } => "DEVICE_REGISTERED",
            Self::SettingsInitialized { .. } => "SETTINGS_INITIALIZED",
            Self::OwnerBootstrapped { .. } => "OWNER_BOOTSTRAPPED",
            Self::InviteIssued { .. } => "INVITE_ISSUED",
            Self::SignupPending { .. } => "SIGNUP_PENDING",
            Self::AccountApproved { .. } => "ACCOUNT_APPROVED",
            Self::AccountRevoked { .. } => "ACCOUNT_REVOKED",
            Self::SessionStarted { .. } => "SESSION_STARTED",
            Self::SessionLoggedOut { .. } => "SESSION_LOGGED_OUT",
            Self::SensitiveIdentityAccessed { .. } => "SENSITIVE_IDENTITY_ACCESSED",
            Self::LookupAuthorizationGranted { .. } => "LOOKUP_AUTHORIZATION_GRANTED",
            Self::LookupAuthorizationConsumed { .. } => "LOOKUP_AUTHORIZATION_CONSUMED",
            Self::MemberCreated { .. } => "MEMBER_CREATED",
            Self::FriendAdded { .. } => "FRIEND_ADDED",
            Self::FriendArchived { .. } => "FRIEND_ARCHIVED",
            Self::InvestmentSessionCreated { .. } => "INVESTMENT_SESSION_CREATED",
            Self::IpoApplicationCreated { .. } => "IPO_APPLICATION_CREATED",
            Self::AllocationAdded { .. } => "ALLOCATION_ADDED",
            Self::InvestmentSessionSubmitted { .. } => "INVESTMENT_SESSION_SUBMITTED",
            Self::InvestmentSessionVoided { .. } => "INVESTMENT_SESSION_VOIDED",
            Self::InvestmentRecommendationGenerated { .. } => "INVESTMENT_RECOMMENDATION_GENERATED",
            Self::InvestmentRecommendationApplied { .. } => "INVESTMENT_RECOMMENDATION_APPLIED",
            Self::AllotmentJobCreated { .. } => "ALLOTMENT_JOB_CREATED",
            Self::AllotmentJobStatusChanged { .. } => "ALLOTMENT_JOB_STATUS_CHANGED",
            Self::AllotmentAttemptRecorded { .. } => "ALLOTMENT_ATTEMPT_RECORDED",
            Self::AllotmentAttemptStateUpdated { .. } => "ALLOTMENT_ATTEMPT_STATE_UPDATED",
            Self::AllotmentResolutionFactRecorded { .. } => "ALLOTMENT_RESOLUTION_FACT_RECORDED",
            Self::AllotmentProviderChallengeUpdated { .. } => {
                "ALLOTMENT_PROVIDER_CHALLENGE_UPDATED"
            }
            Self::AllotmentProviderDiscovered { .. } => "ALLOTMENT_PROVIDER_DISCOVERED",
            Self::EstimatedProfitUpdated { .. } => "ESTIMATED_PROFIT_UPDATED",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NewEvent {
    pub event_id: String,
    pub aggregate_type: String,
    pub aggregate_id: String,
    pub aggregate_revision: u64,
    pub actor_member_id: String,
    pub device_id: String,
    pub occurred_at: String,
    pub app_version: String,
    pub previous_event_hash: Option<String>,
    pub payload: EventPayload,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EventEnvelope {
    schema_version: u16,
    #[serde(deserialize_with = "deserialize_event_id")]
    event_id: String,
    event_type: String,
    aggregate_type: String,
    aggregate_id: String,
    aggregate_revision: u64,
    actor_member_id: String,
    device_id: String,
    occurred_at: String,
    app_version: String,
    payload: EventPayload,
    previous_event_hash: Option<String>,
    content_hash: String,
}

#[derive(Debug, Error)]
pub enum EventError {
    #[error("event field {0} cannot be empty")]
    EmptyField(&'static str),
    #[error("event id is not a path-safe filename: {0}")]
    UnsafeEventId(String),
    #[error("aggregate revision must be at least one")]
    InvalidRevision,
    #[error("event serialization failed: {0}")]
    Serialization(#[from] serde_json::Error),
}

#[derive(Serialize)]
struct HashableEvent<'a> {
    schema_version: u16,
    event_id: &'a str,
    event_type: &'a str,
    aggregate_type: &'a str,
    aggregate_id: &'a str,
    aggregate_revision: u64,
    actor_member_id: &'a str,
    device_id: &'a str,
    occurred_at: &'a str,
    app_version: &'a str,
    payload: &'a EventPayload,
    previous_event_hash: &'a Option<String>,
}

impl EventEnvelope {
    pub fn seal(mut event: NewEvent) -> Result<Self, EventError> {
        // If the caller left the event id empty, mint a stable UUIDv7 id so
        // events are unique and append-only without forcing every caller to
        // generate ids by hand.
        if event.event_id.trim().is_empty() {
            event.event_id = uuid::Uuid::now_v7().to_string();
        }
        validate_required("event_id", &event.event_id)?;
        validate_event_id(&event.event_id)?;
        validate_required("aggregate_type", &event.aggregate_type)?;
        validate_required("aggregate_id", &event.aggregate_id)?;
        validate_required("actor_member_id", &event.actor_member_id)?;
        validate_required("device_id", &event.device_id)?;
        validate_required("occurred_at", &event.occurred_at)?;
        validate_required("app_version", &event.app_version)?;
        if event.aggregate_revision == 0 {
            return Err(EventError::InvalidRevision);
        }

        let mut envelope = Self {
            schema_version: EVENT_SCHEMA_VERSION,
            event_id: event.event_id,
            event_type: event.payload.event_type().to_owned(),
            aggregate_type: event.aggregate_type,
            aggregate_id: event.aggregate_id,
            aggregate_revision: event.aggregate_revision,
            actor_member_id: event.actor_member_id,
            device_id: event.device_id,
            occurred_at: event.occurred_at,
            app_version: event.app_version,
            payload: event.payload,
            previous_event_hash: event.previous_event_hash,
            content_hash: String::new(),
        };
        envelope.content_hash = envelope.calculate_hash()?;
        Ok(envelope)
    }

    pub fn event_type(&self) -> &str {
        &self.event_type
    }

    pub fn event_id(&self) -> &str {
        &self.event_id
    }

    pub fn content_hash(&self) -> &str {
        &self.content_hash
    }

    /// The event payload, for projection/replay.
    pub fn payload(&self) -> &EventPayload {
        &self.payload
    }

    pub fn actor_member_id(&self) -> &str {
        &self.actor_member_id
    }

    pub fn device_id(&self) -> &str {
        &self.device_id
    }

    pub fn occurred_at(&self) -> &str {
        &self.occurred_at
    }

    pub fn verify_integrity(&self) -> Result<bool, EventError> {
        Ok(self.content_hash == self.calculate_hash()?)
    }

    fn calculate_hash(&self) -> Result<String, EventError> {
        let hashable = HashableEvent {
            schema_version: self.schema_version,
            event_id: &self.event_id,
            event_type: &self.event_type,
            aggregate_type: &self.aggregate_type,
            aggregate_id: &self.aggregate_id,
            aggregate_revision: self.aggregate_revision,
            actor_member_id: &self.actor_member_id,
            device_id: &self.device_id,
            occurred_at: &self.occurred_at,
            app_version: &self.app_version,
            payload: &self.payload,
            previous_event_hash: &self.previous_event_hash,
        };
        let bytes = serde_json::to_vec(&hashable)?;
        Ok(format!("sha256:{:x}", Sha256::digest(bytes)))
    }
}

fn deserialize_event_id<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let event_id = String::deserialize(deserializer)?;
    validate_event_id(&event_id).map_err(serde::de::Error::custom)?;
    Ok(event_id)
}

fn validate_event_id(event_id: &str) -> Result<(), EventError> {
    if event_id.is_empty()
        || !event_id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
    {
        return Err(EventError::UnsafeEventId(event_id.to_owned()));
    }
    Ok(())
}

fn validate_required(field: &'static str, value: &str) -> Result<(), EventError> {
    if value.trim().is_empty() {
        return Err(EventError::EmptyField(field));
    }
    Ok(())
}
