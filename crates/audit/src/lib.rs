use sanket_domain::{EventEnvelope, EventError, EventPayload, NewEvent};
use sanket_identity_security::SensitiveAccessAudit;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AuditContext {
    pub event_id: String,
    pub actor_member_id: String,
    pub device_id: String,
    pub occurred_at: String,
    pub app_version: String,
}

/// Convert a service-emitted [`SensitiveAccessAudit`] into a sealable event.
///
/// This is the wiring between purpose-scoped access and durable event
/// persistence: callers append the resulting envelope to the MemberVault.
pub fn access_audit_event(
    audit: &SensitiveAccessAudit,
    context: AuditContext,
) -> Result<EventEnvelope, EventError> {
    sensitive_identity_accessed(context, audit.account_id.clone(), audit.purpose.clone())
}

pub fn sensitive_identity_accessed(
    context: AuditContext,
    account_id: String,
    purpose: String,
) -> Result<EventEnvelope, EventError> {
    EventEnvelope::seal(NewEvent {
        aggregate_type: "AUDIT_EVENT".to_owned(),
        aggregate_id: context.event_id.clone(),
        aggregate_revision: 1,
        previous_event_hash: None,
        payload: EventPayload::SensitiveIdentityAccessed {
            account_id,
            purpose,
        },
        event_id: context.event_id,
        actor_member_id: context.actor_member_id,
        device_id: context.device_id,
        occurred_at: context.occurred_at,
        app_version: context.app_version,
    })
}
