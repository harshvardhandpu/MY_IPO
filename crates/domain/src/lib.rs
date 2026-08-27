use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

mod members;
mod money;

pub use members::{CoreMember, FriendAccount, FriendShareError, MemberStatus};
pub use money::{BasisPoints, BasisPointsError, Money};

pub const EVENT_SCHEMA_VERSION: u16 = 1;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Role {
    Owner,
    CoreMember,
}

impl Role {
    pub const fn can_manage_members(self) -> bool {
        matches!(self, Self::Owner)
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
    SensitiveIdentityAccessed {
        account_id: String,
        purpose: String,
    },
    /// Member-wide notification: a friend account was added. Carries no PAN.
    FriendAccountAdded {
        friend_id: String,
        owner_member_id: String,
        label: String,
        share_basis_points: i64,
    },
    /// Member-wide notification: a friend account was archived (never deleted).
    FriendAccountArchived {
        friend_id: String,
        owner_member_id: String,
    },
}

impl EventPayload {
    fn event_type(&self) -> &'static str {
        match self {
            Self::DeviceRegistered { .. } => "DEVICE_REGISTERED",
            Self::SettingsInitialized { .. } => "SETTINGS_INITIALIZED",
            Self::SensitiveIdentityAccessed { .. } => "SENSITIVE_IDENTITY_ACCESSED",
            Self::FriendAccountAdded { .. } => "FRIEND_ACCOUNT_ADDED",
            Self::FriendAccountArchived { .. } => "FRIEND_ACCOUNT_ARCHIVED",
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
    pub fn seal(event: NewEvent) -> Result<Self, EventError> {
        validate_required("event_id", &event.event_id)?;
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

fn validate_required(field: &'static str, value: &str) -> Result<(), EventError> {
    if value.trim().is_empty() {
        return Err(EventError::EmptyField(field));
    }
    Ok(())
}
