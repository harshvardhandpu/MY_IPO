//! Application service: wires the domain, vault, index, and ranking together
//! behind narrowly-scoped operations. Sensitive input (PAN/UPI) enters here
//! only through explicit command arguments and is encrypted immediately.

use std::path::PathBuf;
use std::sync::OnceLock;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use sanket_domain::{
    CoreMember, EventEnvelope, EventPayload, FriendAccount, InvestmentSession, IpoMetadataSnapshot,
    Money, NewEvent, Role,
};
use sanket_identity_security::{
    IdentityKey, InMemoryKeyProvider, OsKeyringKeyProvider, Pan, RuntimeSecurityMode,
    assert_mode_allows_provider,
};
use sanket_intelligence_vault::{InvestmentDecisionRequest, PlannedIpo};
use sanket_local_index::LocalIndex;
use sanket_member_vault::MemberVault;
use sanket_ranking::{DevRankingAlgorithm, RankingAlgorithm};

pub const DEV_KEY_ID: &str = "dev-key-1";
pub const PRODUCTION_KEY_ID: &str = "os-keyring:v1:identity-key-v1";
pub const LOOKUP_AUTHORIZATION_LIFETIME_SECS: u64 = 300;

fn allotment_rate_limiter() -> &'static sanket_allotment::ProviderRateLimiter {
    static LIMITER: OnceLock<sanket_allotment::ProviderRateLimiter> = OnceLock::new();
    LIMITER.get_or_init(|| {
        // Gate 4F condition B: distinct per-provider policies, not one global
        // Default. KfintechFixture stays zero-spacing (synthetic); live
        // providers keep their tested spacing/attempt ceilings.
        let limiter = sanket_allotment::ProviderRateLimiter::new(Default::default());
        for kind in [
            sanket_allotment::ProviderId::KfintechFixture,
            sanket_allotment::ProviderId::KfintechLive,
            sanket_allotment::ProviderId::BigshareLive,
            sanket_allotment::ProviderId::MufgIntimeLive,
        ] {
            limiter.set_policy(
                kind.as_str(),
                sanket_allotment::ProviderRatePolicy::for_provider(kind),
            );
        }
        limiter
    })
}

fn execution_provider_id(
    mode: RuntimeSecurityMode,
    provider: sanket_allotment::ProviderId,
) -> sanket_allotment::ProviderId {
    if matches!(mode, RuntimeSecurityMode::DevelopmentSynthetic)
        && provider == sanket_allotment::ProviderId::KfintechLive
    {
        sanket_allotment::ProviderId::KfintechFixture
    } else {
        provider
    }
}

fn provider_health(provider: sanket_allotment::ProviderId) -> &'static str {
    use sanket_allotment::{AllotmentProvider, ProviderHealth, ProviderId};
    let health = match provider {
        ProviderId::KfintechFixture => ProviderHealth::Available,
        ProviderId::KfintechLive => sanket_allotment::KfintechProvider::new().health(),
        ProviderId::BigshareLive => sanket_allotment::BigshareProvider::new().health(),
        ProviderId::MufgIntimeLive => sanket_allotment::MufgIntimeProvider::new().health(),
    };
    match health {
        ProviderHealth::Available => "AVAILABLE",
        ProviderHealth::Degraded => "DEGRADED",
        ProviderHealth::HumanVerificationRequired => "HUMAN_VERIFICATION_REQUIRED",
        ProviderHealth::Broken => "BROKEN",
        ProviderHealth::Unknown => "UNKNOWN",
    }
}

fn allotment_provider(
    provider: sanket_allotment::ProviderId,
) -> Box<dyn sanket_allotment::AllotmentProvider> {
    use sanket_allotment::ProviderId;
    match provider {
        ProviderId::KfintechFixture => Box::new(sanket_allotment::FixtureKfintechProvider),
        ProviderId::KfintechLive => Box::new(sanket_allotment::KfintechProvider::new()),
        ProviderId::BigshareLive => Box::new(sanket_allotment::BigshareProvider::new()),
        ProviderId::MufgIntimeLive => Box::new(sanket_allotment::MufgIntimeProvider::new()),
    }
}

fn epoch_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

#[derive(Debug, thiserror::Error)]
pub enum ServiceError {
    #[error("vault error: {0}")]
    Vault(#[from] sanket_member_vault::MemberVaultError),
    #[error("index error: {0}")]
    Index(#[from] sanket_local_index::LocalIndexError),
    #[error("domain error: {0}")]
    Domain(#[from] sanket_domain::InvestmentError),
    #[error("event error: {0}")]
    Event(#[from] sanket_domain::EventError),
    #[error("cipher error: {0}")]
    Cipher(#[from] sanket_identity_security::CipherError),
    #[error("PAN error: {0}")]
    Pan(#[from] sanket_identity_security::PanError),
    #[error("identity secret error: {0}")]
    Secret(#[from] sanket_identity_security::IdentitySecretError),
    #[error("ranking error: {0}")]
    Ranking(#[from] sanket_ranking::RecommendationError),
    #[error("key provider: {0}")]
    KeyProvider(String),
    #[error("{0}")]
    Invalid(String),
}

pub type Result<T> = std::result::Result<T, ServiceError>;

#[derive(Clone, Debug, PartialEq, Eq)]
struct LookupAuthorizationRecord {
    authorization_id: String,
    application_id: String,
    provider_id: String,
    expiry_time: String,
    expiry_epoch: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct LookupAuthorizationState {
    status: &'static str,
    record: Option<LookupAuthorizationRecord>,
}

/// Shared application state: vault, projection index, and the dev key provider.
pub struct Application {
    device_id: String,
    vault: MemberVault,
    index: LocalIndex,
    security_mode: RuntimeSecurityMode,
}

/// A stable per-device development key derived from the device id (NOT a
/// hardcoded literal key, and not for production use).
fn dev_key(device_id: &str) -> IdentityKey {
    let digest = Sha256::digest(device_id.as_bytes());
    let mut bytes = [0u8; 32];
    bytes.copy_from_slice(&digest);
    IdentityKey::from_bytes(&bytes)
}

fn resolve_security_mode() -> RuntimeSecurityMode {
    std::env::var("SANKET_SECURITY_MODE")
        .map(|s| RuntimeSecurityMode::parse(&s))
        .unwrap_or(RuntimeSecurityMode::DevelopmentSynthetic)
}

/// Reject a format-valid PAN before free-form text can be persisted into an
/// event, SQLite projection, provider reference, or log-adjacent error path.
fn reject_embedded_pan(field: &str, value: &str) -> Result<()> {
    let normalized = value.to_ascii_uppercase();
    for candidate in normalized.as_bytes().windows(10) {
        if let Ok(candidate) = std::str::from_utf8(candidate) {
            if Pan::parse(candidate).is_ok() {
                return Err(ServiceError::Invalid(format!(
                    "{field} must not contain a PAN"
                )));
            }
        }
    }
    Ok(())
}

fn looks_like_upi(value: &str) -> bool {
    let Some((local, handle)) = value.split_once('@') else {
        return false;
    };
    !local.is_empty()
        && !handle.is_empty()
        && local
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"._-".contains(&byte))
        && handle
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"._-".contains(&byte))
}

fn validate_metadata_text(field: &str, value: &str) -> Result<()> {
    if value.len() > 512 || value.chars().any(char::is_control) {
        return Err(ServiceError::Invalid(format!(
            "{field} contains unsafe text"
        )));
    }
    reject_embedded_pan(field, value)?;
    let normalized = value.to_ascii_lowercase();
    if looks_like_upi(value)
        || normalized.contains("bearer ")
        || normalized.contains("api_key")
        || normalized.contains("api-key")
        || normalized.contains("password")
        || normalized.contains("secret")
        || normalized.contains("token=")
        || normalized.contains("token:")
    {
        return Err(ServiceError::Invalid(format!(
            "{field} contains sensitive data"
        )));
    }
    Ok(())
}

fn validate_metadata_snapshot(metadata: &IpoMetadataSnapshot) -> Result<()> {
    for (field, value) in [
        ("metadata source", metadata.metadata_source.as_str()),
        ("source IPO id", metadata.source_ipo_id.as_str()),
        ("source status", metadata.source_status.as_str()),
        ("source name", metadata.source_name.as_str()),
        ("source symbol", metadata.source_symbol.as_str()),
        ("fetched at", metadata.fetched_at.as_str()),
        ("price basis", metadata.price_basis.as_str()),
    ] {
        validate_metadata_text(field, value)?;
    }
    for (field, value) in [
        ("source ISIN", metadata.source_isin.as_deref()),
        ("revalidated at", metadata.revalidated_at.as_deref()),
        ("bidding start date", metadata.bidding_start_date.as_deref()),
        ("bidding end date", metadata.bidding_end_date.as_deref()),
        ("allotment date", metadata.allotment_date.as_deref()),
        ("listing date", metadata.listing_date.as_deref()),
        ("registrar name", metadata.registrar_name.as_deref()),
        (
            "registrar short name",
            metadata.registrar_short_name.as_deref(),
        ),
        ("registrar website", metadata.registrar_website.as_deref()),
    ] {
        if let Some(value) = value {
            validate_metadata_text(field, value)?;
        }
    }
    Ok(())
}

fn is_valid_iso_date(value: &str) -> bool {
    let mut parts = value.split('-');
    let (Some(year), Some(month), Some(day), None) =
        (parts.next(), parts.next(), parts.next(), parts.next())
    else {
        return false;
    };
    if year.len() != 4 || month.len() != 2 || day.len() != 2 {
        return false;
    }
    let (Ok(year), Ok(month), Ok(day)) = (
        year.parse::<u32>(),
        month.parse::<u32>(),
        day.parse::<u32>(),
    ) else {
        return false;
    };
    let leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    let max_day = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if leap => 29,
        2 => 28,
        _ => return false,
    };
    (1..=max_day).contains(&day)
}

impl Application {
    pub fn new(device_id: String, vault_root: PathBuf, index_path: PathBuf) -> Result<Self> {
        Self::with_mode(device_id, vault_root, index_path, resolve_security_mode())
    }

    pub fn with_mode(
        device_id: String,
        vault_root: PathBuf,
        index_path: PathBuf,
        security_mode: RuntimeSecurityMode,
    ) -> Result<Self> {
        let vault = MemberVault::open(vault_root)?;
        let index = LocalIndex::open(&index_path)?;
        Ok(Self {
            device_id,
            vault,
            index,
            security_mode,
        })
    }

    pub fn security_mode(&self) -> RuntimeSecurityMode {
        self.security_mode
    }

    fn identity_key(&self) -> Result<IdentityKey> {
        match self.security_mode {
            RuntimeSecurityMode::DevelopmentSynthetic => Ok(dev_key(&self.device_id)),
            RuntimeSecurityMode::ProductionSecure => {
                let os = OsKeyringKeyProvider::new();
                assert_mode_allows_provider(self.security_mode, &os)
                    .map_err(|e| ServiceError::KeyProvider(e.to_string()))?;
                os.ensure_key(PRODUCTION_KEY_ID)
                    .map_err(|e| ServiceError::KeyProvider(e.to_string()))
            }
        }
    }

    fn cipher(&self) -> Result<sanket_identity_security::IdentityCipher> {
        Ok(sanket_identity_security::IdentityCipher::new(
            self.identity_key()?,
        ))
    }

    fn key_provider_for_sensitive(&self) -> Result<Box<dyn sanket_identity_security::KeyProvider>> {
        match self.security_mode {
            RuntimeSecurityMode::DevelopmentSynthetic => Ok(Box::new(InMemoryKeyProvider::new(
                self.active_key_id(),
                dev_key(&self.device_id),
            ))),
            RuntimeSecurityMode::ProductionSecure => {
                let os = OsKeyringKeyProvider::new();
                assert_mode_allows_provider(self.security_mode, &os)
                    .map_err(|e| ServiceError::KeyProvider(e.to_string()))?;
                let _ = os
                    .ensure_key(PRODUCTION_KEY_ID)
                    .map_err(|e| ServiceError::KeyProvider(e.to_string()))?;
                Ok(Box::new(os))
            }
        }
    }

    fn active_key_id(&self) -> &'static str {
        match self.security_mode {
            RuntimeSecurityMode::DevelopmentSynthetic => DEV_KEY_ID,
            RuntimeSecurityMode::ProductionSecure => PRODUCTION_KEY_ID,
        }
    }

    fn validate_allotment_provider(&self, provider_id: &str) -> Result<()> {
        match (self.security_mode, provider_id) {
            (RuntimeSecurityMode::DevelopmentSynthetic, "kfintech-fixture") => Ok(()),
            // Live adapters are enqueueable in dev to reach their typed
            // operational/human-verification states; the unattended run path
            // still fails closed before any PAN access or network lookup.
            (RuntimeSecurityMode::DevelopmentSynthetic, "kfintech-live")
            | (RuntimeSecurityMode::DevelopmentSynthetic, "bigshare-live")
            | (RuntimeSecurityMode::DevelopmentSynthetic, "mufg-intime-live") => Ok(()),
            (RuntimeSecurityMode::ProductionSecure, "kfintech-live")
            | (RuntimeSecurityMode::ProductionSecure, "bigshare-live")
            | (RuntimeSecurityMode::ProductionSecure, "mufg-intime-live") => {
                self.identity_key().map(|_| ())
            }
            (RuntimeSecurityMode::ProductionSecure, "kfintech-fixture") => {
                Err(ServiceError::Invalid(
                    "fixture provider is restricted to DEVELOPMENT_SYNTHETIC".into(),
                ))
            }
            _ => Err(ServiceError::Invalid(
                "unsupported allotment provider".into(),
            )),
        }
    }

    fn now() -> String {
        // RFC3339 via the same std-only approach as identity-security.
        let secs = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        format!("{secs}")
    }

    // --- onboarding ---

    /// Onboard a core member. PAN and UPI are validated and encrypted here;
    /// only the masked PAN and persisted ids are returned to the UI.
    pub fn onboard_member(&self, req: OnboardMemberRequest) -> Result<OnboardMemberResponse> {
        // email/broker are captured for a future profile extension; not yet
        // persisted, and never enter any event or AI payload.
        let _ = (&req.email, &req.broker);
        let pan = Pan::parse(&req.pan)?;
        let masked_pan = pan.mask();
        let member_id = req.member_id.clone();
        let sensitive_record_id = format!("rec-{member_id}");

        // Encrypt the full identity payload (PAN + UPI).
        let identity_payload = sanket_identity_security::IdentitySecret {
            pan: pan.clone(),
            upi_id: Some(req.upi_id.clone()),
        };
        let envelope = self.cipher()?.encrypt(
            identity_payload_to_json(&identity_payload).as_bytes(),
            self.active_key_id(),
        )?;
        self.vault.store_member_identity(&member_id, &envelope)?;

        // Build the profile (masked PAN only) and persist.
        let role = role_from_str(&req.role);
        let mut member = CoreMember::onboard(
            member_id.clone(),
            req.display_name.clone(),
            role,
            masked_pan.clone(),
            sensitive_record_id,
        );
        if let Some(account_label) = &req.primary_account_label {
            member.designate_primary_account(account_label);
        }
        self.vault.store_member_profile(&member)?;

        // Project into SQLite.
        self.index.upsert_member(
            &member_id,
            &req.display_name,
            &req.role,
            masked_pan.as_str(),
        )?;

        // Emit the member-created event.
        let event = EventEnvelope::seal(NewEvent {
            event_id: String::new(),
            aggregate_type: "member".to_owned(),
            aggregate_id: member_id.clone(),
            aggregate_revision: 1,
            actor_member_id: member_id.clone(),
            device_id: self.device_id.clone(),
            occurred_at: Self::now(),
            app_version: env!("CARGO_PKG_VERSION").to_owned(),
            previous_event_hash: None,
            payload: EventPayload::MemberCreated {
                member_id: member_id.clone(),
                display_name: req.display_name.clone(),
                role,
            },
        })?;
        self.vault.append_event(&event)?;
        self.index.apply_event(&event)?;

        Ok(OnboardMemberResponse {
            member_id,
            masked_pan: masked_pan.to_string(),
        })
    }

    /// Add a friend account with an optional profit-share.
    pub fn add_friend(&self, req: AddFriendRequest) -> Result<AddFriendResponse> {
        // broker is captured for a future profile extension; not yet persisted.
        let _ = &req.broker;
        let pan = Pan::parse(&req.pan)?;
        let masked_pan = pan.mask();
        let friend_id = req.friend_id.clone();
        let sensitive_record_id = format!("rec-{friend_id}");
        let share_bp = if req.share_eligible {
            req.share_basis_points.unwrap_or(1_000)
        } else {
            0
        };
        let share = sanket_domain::BasisPoints::try_new(share_bp)
            .map_err(|e| ServiceError::Invalid(e.to_string()))?;

        let identity_payload = sanket_identity_security::IdentitySecret {
            pan: pan.clone(),
            upi_id: Some(req.upi_id.clone()),
        };
        let envelope = self.cipher()?.encrypt(
            identity_payload_to_json(&identity_payload).as_bytes(),
            self.active_key_id(),
        )?;
        self.vault.store_friend_identity(&friend_id, &envelope)?;

        let friend = FriendAccount::create(
            friend_id.clone(),
            req.owner_member_id.clone(),
            req.name.clone(),
            masked_pan.clone(),
            sensitive_record_id,
        );
        let mut friend = friend;
        if req.share_eligible {
            friend.set_share_basis_points(share).ok();
        }
        self.vault.store_friend_profile(&friend)?;

        self.index.upsert_friend(
            &friend_id,
            &req.owner_member_id,
            &req.name,
            masked_pan.as_str(),
            share.value(),
        )?;

        let event = EventEnvelope::seal(NewEvent {
            event_id: String::new(),
            aggregate_type: "friend_account".to_owned(),
            aggregate_id: friend_id.clone(),
            aggregate_revision: 1,
            actor_member_id: req.owner_member_id.clone(),
            device_id: self.device_id.clone(),
            occurred_at: Self::now(),
            app_version: env!("CARGO_PKG_VERSION").to_owned(),
            previous_event_hash: None,
            payload: EventPayload::FriendAdded {
                friend_id: friend_id.clone(),
                owner_member_id: req.owner_member_id.clone(),
                label: req.name.clone(),
                share_basis_points: share.value(),
            },
        })?;
        self.vault.append_event(&event)?;
        self.index.apply_event(&event)?;

        Ok(AddFriendResponse {
            friend_id,
            masked_pan: masked_pan.to_string(),
        })
    }

    /// Archive a friend (never deletes; preserves identity/investment links).
    pub fn archive_friend(&self, friend_id: &str, owner_member_id: &str) -> Result<()> {
        self.index.archive_friend(friend_id)?;
        let event = EventEnvelope::seal(NewEvent {
            event_id: String::new(),
            aggregate_type: "friend_account".to_owned(),
            aggregate_id: friend_id.to_owned(),
            aggregate_revision: 1,
            actor_member_id: owner_member_id.to_owned(),
            device_id: self.device_id.clone(),
            occurred_at: Self::now(),
            app_version: env!("CARGO_PKG_VERSION").to_owned(),
            previous_event_hash: None,
            payload: EventPayload::FriendArchived {
                friend_id: friend_id.to_owned(),
                owner_member_id: owner_member_id.to_owned(),
            },
        })?;
        self.vault.append_event(&event)?;
        Ok(self.index.apply_event(&event)?)
    }

    // --- recommendations ---

    /// Run CHECK: build the sanitized decision request and run the dev algorithm.
    pub fn check(&self, req: CheckRequest) -> Result<CheckResponse> {
        let ipos: Vec<String> = req.ipos.iter().map(|i| i.name.clone()).collect();
        let account_count = req.account_ids.len() as u32;

        // Build the sanitized request and fail closed if it carries any secret.
        let planned: Vec<PlannedIpo> = req
            .ipos
            .iter()
            .map(|i| PlannedIpo {
                typed_name: i.name.clone(),
                planned_amount_per_account_paise: i.amount_paise,
            })
            .collect();
        let decision = InvestmentDecisionRequest::with_account_count(
            req.session_id.clone(),
            req.declared_capital_paise,
            planned,
            "dev-ranking-v001",
            account_count,
        );
        decision
            .assert_safe()
            .map_err(|e| ServiceError::Invalid(e.to_string()))?;

        let algorithm = DevRankingAlgorithm::new();
        let recommendation = algorithm.evaluate(ipos, account_count)?;

        Ok(CheckResponse {
            session_id: req.session_id,
            algorithm_version: algorithm.version().to_owned(),
            label: recommendation.label().to_owned(),
            explanation: algorithm.explain(&recommendation),
            ipos: recommendation.ipos().map(ranked_to_response).collect(),
        })
    }

    /// SUBMIT: persist the human-selected session, applications, and allocations.
    pub fn submit(&self, req: SubmitRequest) -> Result<SubmitResponse> {
        if req.declared_capital_paise <= 0 {
            return Err(ServiceError::Invalid(
                "declared capital must be positive".to_owned(),
            ));
        }
        if req.ipos.is_empty() {
            return Err(ServiceError::Invalid(
                "at least one IPO is required".to_owned(),
            ));
        }

        // Session (open then submitted).
        let session_id = req.session_id.clone();
        let mut session = InvestmentSession::open(
            &session_id,
            &req.actor_member_id,
            Money::from_paise(req.declared_capital_paise),
        )?;
        if let Some(rec_id) = &req.recommendation_id {
            session.attach_recommendation(rec_id);
        }

        let mut events = Vec::new();
        // 1. session created
        events.push(EventEnvelope::seal(NewEvent {
            event_id: String::new(),
            aggregate_type: "session".to_owned(),
            aggregate_id: session_id.clone(),
            aggregate_revision: 1,
            actor_member_id: req.actor_member_id.clone(),
            device_id: self.device_id.clone(),
            occurred_at: Self::now(),
            app_version: env!("CARGO_PKG_VERSION").to_owned(),
            previous_event_hash: None,
            payload: EventPayload::InvestmentSessionCreated {
                session_id: session_id.clone(),
                actor_member_id: req.actor_member_id.clone(),
                declared_capital_paise: req.declared_capital_paise,
            },
        })?);

        // 2. applications + allocations
        for (idx, ipo) in req.ipos.iter().enumerate() {
            let registrar =
                sanket_allotment::ProviderRegistry::resolve_registrar(&ipo.registrar_id)
                    .ok_or_else(|| ServiceError::Invalid("unsupported registrar".into()))?;
            if let Some(date) = &ipo.expected_allotment_date {
                reject_embedded_pan("expected allotment date", date)?;
            }
            if let Some(metadata) = &ipo.metadata_snapshot {
                validate_metadata_snapshot(metadata)?;
                let account_count = i64::try_from(ipo.account_ids.len())
                    .map_err(|_| ServiceError::Invalid("too many accounts".to_owned()))?;
                let total_capital = ipo
                    .amount_paise
                    .checked_mul(account_count)
                    .ok_or_else(|| ServiceError::Invalid("amount is too large".to_owned()))?;
                if metadata.metadata_source != "UPSTOX_IPO_API"
                    || metadata.source_status != "OPEN"
                    || metadata.source_ipo_id.is_empty()
                    || metadata.amount_per_account_paise != ipo.amount_paise
                    || metadata.total_capital_paise != total_capital
                {
                    return Err(ServiceError::Invalid(
                        "official IPO metadata calculation mismatch".to_owned(),
                    ));
                }
            }
            let app_id = format!("{session_id}-app-{idx}");
            events.push(EventEnvelope::seal(NewEvent {
                event_id: String::new(),
                aggregate_type: "application".to_owned(),
                aggregate_id: app_id.clone(),
                aggregate_revision: 1,
                actor_member_id: req.actor_member_id.clone(),
                device_id: self.device_id.clone(),
                occurred_at: Self::now(),
                app_version: env!("CARGO_PKG_VERSION").to_owned(),
                previous_event_hash: None,
                payload: EventPayload::IpoApplicationCreated {
                    application_id: app_id.clone(),
                    session_id: session_id.clone(),
                    ipo_name: ipo.name.clone(),
                    planned_amount_paise: ipo.amount_paise,
                    registrar_id: registrar.registrar_id.into(),
                    registrar_name: registrar.registrar_name.into(),
                    official_status_url: Some(registrar.official_status_url.into()),
                    expected_allotment_date: ipo.expected_allotment_date.clone(),
                    source: if ipo.metadata_snapshot.is_some() {
                        "UPSTOX_IPO_API".into()
                    } else {
                        "OWNER_CURRENT_ENTRY".into()
                    },
                    application_date: None,
                    metadata: ipo.metadata_snapshot.clone().map(Box::new),
                },
            })?);

            for (alloc_idx, account_id) in ipo.account_ids.iter().enumerate() {
                let alloc_id = format!("{app_id}-alloc-{alloc_idx}");
                events.push(EventEnvelope::seal(NewEvent {
                    event_id: String::new(),
                    aggregate_type: "allocation".to_owned(),
                    aggregate_id: alloc_id.clone(),
                    aggregate_revision: 1,
                    actor_member_id: req.actor_member_id.clone(),
                    device_id: self.device_id.clone(),
                    occurred_at: Self::now(),
                    app_version: env!("CARGO_PKG_VERSION").to_owned(),
                    previous_event_hash: None,
                    payload: EventPayload::AllocationAdded {
                        allocation_id: alloc_id,
                        application_id: app_id.clone(),
                        account_id: account_id.clone(),
                        amount_paise: ipo.amount_paise,
                        share_basis_points: 1_000,
                    },
                })?);
            }
        }

        // 3. session submitted
        events.push(EventEnvelope::seal(NewEvent {
            event_id: String::new(),
            aggregate_type: "session".to_owned(),
            aggregate_id: session_id.clone(),
            aggregate_revision: 2,
            actor_member_id: req.actor_member_id.clone(),
            device_id: self.device_id.clone(),
            occurred_at: Self::now(),
            app_version: env!("CARGO_PKG_VERSION").to_owned(),
            previous_event_hash: None,
            payload: EventPayload::InvestmentSessionSubmitted {
                session_id: session_id.clone(),
                recommendation_id: req.recommendation_id.clone(),
            },
        })?);

        // Persist + project.
        for event in &events {
            self.vault.append_event(event)?;
            self.index.apply_event(event)?;
        }

        let allocation_count = req.ipos.iter().map(|i| i.account_ids.len()).sum::<usize>();
        Ok(SubmitResponse {
            session_id,
            allocation_count: allocation_count as u32,
        })
    }

    pub fn record_historical_application(
        &self,
        req: HistoricalApplicationRequest,
    ) -> Result<HistoricalApplicationResponse> {
        if !req.owner_affirmed {
            return Err(ServiceError::Invalid(
                "owner affirmation is required".to_owned(),
            ));
        }
        if req.actor_member_id != req.account_id {
            return Err(ServiceError::Invalid(
                "historical applications must use the affirming owner account".to_owned(),
            ));
        }
        let owner_exists = self
            .index
            .list_members()?
            .into_iter()
            .any(|(id, _, role, _)| id == req.account_id && role == "OWNER");
        if !owner_exists {
            return Err(ServiceError::Invalid(
                "owner primary account was not found".to_owned(),
            ));
        }
        if req.amount_paise <= 0 {
            return Err(ServiceError::Invalid(
                "historical application amount must be positive".to_owned(),
            ));
        }
        let ipo_name = req.ipo_name.trim();
        if ipo_name.is_empty() {
            return Err(ServiceError::Invalid("IPO name is required".to_owned()));
        }
        reject_embedded_pan("IPO name", ipo_name)?;
        reject_embedded_pan("provider issue id", &req.provider_issue_id)?;
        if req.provider_issue_id.is_empty()
            || req.provider_issue_id.len() > 32
            || !req
                .provider_issue_id
                .bytes()
                .all(|byte| byte.is_ascii_digit())
        {
            return Err(ServiceError::Invalid(
                "provider issue id must contain 1 to 32 digits".to_owned(),
            ));
        }
        if let Some(application_date) = req.application_date.as_deref() {
            reject_embedded_pan("application date", application_date)?;
            if !is_valid_iso_date(application_date) {
                return Err(ServiceError::Invalid(
                    "application date must be a real YYYY-MM-DD date".to_owned(),
                ));
            }
        }
        let registrar = sanket_allotment::ProviderRegistry::resolve_registrar(&req.registrar_id)
            .ok_or_else(|| ServiceError::Invalid("unsupported registrar".into()))?;
        let provider_id = execution_provider_id(self.security_mode, registrar.provider_id)
            .as_str()
            .to_owned();
        if self.index.historical_application_exists(
            &req.actor_member_id,
            ipo_name,
            &provider_id,
            &req.provider_issue_id,
        )? {
            return Err(ServiceError::Invalid(
                "this historical application already exists".to_owned(),
            ));
        }
        let session_id = format!("history-{}", uuid::Uuid::now_v7());
        let application_id = format!("{session_id}-app-0");
        let allocation_id = format!("{application_id}-alloc-0");
        let occurred_at = Self::now();

        let mut session = InvestmentSession::open(
            &session_id,
            &req.actor_member_id,
            Money::from_paise(req.amount_paise),
        )?;
        session.mark_submitted();
        let events = [
            EventEnvelope::seal(NewEvent {
                event_id: String::new(),
                aggregate_type: "session".into(),
                aggregate_id: session_id.clone(),
                aggregate_revision: 1,
                actor_member_id: req.actor_member_id.clone(),
                device_id: self.device_id.clone(),
                occurred_at: occurred_at.clone(),
                app_version: env!("CARGO_PKG_VERSION").into(),
                previous_event_hash: None,
                payload: EventPayload::InvestmentSessionCreated {
                    session_id: session_id.clone(),
                    actor_member_id: req.actor_member_id.clone(),
                    declared_capital_paise: req.amount_paise,
                },
            })?,
            EventEnvelope::seal(NewEvent {
                event_id: String::new(),
                aggregate_type: "application".into(),
                aggregate_id: application_id.clone(),
                aggregate_revision: 1,
                actor_member_id: req.actor_member_id.clone(),
                device_id: self.device_id.clone(),
                occurred_at: occurred_at.clone(),
                app_version: env!("CARGO_PKG_VERSION").into(),
                previous_event_hash: None,
                payload: EventPayload::IpoApplicationCreated {
                    application_id: application_id.clone(),
                    session_id: session_id.clone(),
                    ipo_name: ipo_name.to_owned(),
                    planned_amount_paise: req.amount_paise,
                    registrar_id: registrar.registrar_id.into(),
                    registrar_name: registrar.registrar_name.into(),
                    official_status_url: Some(registrar.official_status_url.into()),
                    expected_allotment_date: None,
                    source: "OWNER_HISTORICAL_ENTRY".into(),
                    application_date: req.application_date,
                    metadata: None,
                },
            })?,
            EventEnvelope::seal(NewEvent {
                event_id: String::new(),
                aggregate_type: "allocation".into(),
                aggregate_id: allocation_id.clone(),
                aggregate_revision: 1,
                actor_member_id: req.actor_member_id.clone(),
                device_id: self.device_id.clone(),
                occurred_at: occurred_at.clone(),
                app_version: env!("CARGO_PKG_VERSION").into(),
                previous_event_hash: None,
                payload: EventPayload::AllocationAdded {
                    allocation_id: allocation_id.clone(),
                    application_id: application_id.clone(),
                    account_id: req.account_id,
                    amount_paise: req.amount_paise,
                    share_basis_points: 1_000,
                },
            })?,
            EventEnvelope::seal(NewEvent {
                event_id: String::new(),
                aggregate_type: "application".into(),
                aggregate_id: application_id.clone(),
                aggregate_revision: 2,
                actor_member_id: req.actor_member_id.clone(),
                device_id: self.device_id.clone(),
                occurred_at: occurred_at.clone(),
                app_version: env!("CARGO_PKG_VERSION").into(),
                previous_event_hash: None,
                payload: EventPayload::AllotmentProviderDiscovered {
                    application_id: application_id.clone(),
                    registrar_id: registrar.registrar_id.into(),
                    provider_id: provider_id.clone(),
                    provider_issue_id: req.provider_issue_id.clone(),
                    ipo_name: ipo_name.to_owned(),
                    official_status_url: registrar.official_status_url.into(),
                    last_verified_at: occurred_at.clone(),
                },
            })?,
            EventEnvelope::seal(NewEvent {
                event_id: String::new(),
                aggregate_type: "session".into(),
                aggregate_id: session_id.clone(),
                aggregate_revision: 2,
                actor_member_id: req.actor_member_id,
                device_id: self.device_id.clone(),
                occurred_at,
                app_version: env!("CARGO_PKG_VERSION").into(),
                previous_event_hash: None,
                payload: EventPayload::InvestmentSessionSubmitted {
                    session_id: session_id.clone(),
                    recommendation_id: None,
                },
            })?,
        ];
        for event in &events {
            self.vault.append_event(event)?;
            self.index.apply_event(event)?;
        }

        Ok(HistoricalApplicationResponse {
            session_id,
            application_id,
            allocation_id,
            provider_id,
            provider_issue_id: req.provider_issue_id,
            source: "OWNER_HISTORICAL_ENTRY".into(),
        })
    }

    // --- reads ---

    pub fn list_members(&self) -> Result<Vec<MemberRow>> {
        Ok(self
            .index
            .list_members()?
            .into_iter()
            .map(|(id, name, role, masked)| MemberRow {
                id,
                name,
                role,
                masked_pan: masked,
            })
            .collect())
    }

    pub fn list_friends(&self) -> Result<Vec<FriendRow>> {
        Ok(self
            .index
            .list_active_friends()?
            .into_iter()
            .map(|(id, owner, label, masked, share)| FriendRow {
                id,
                owner_member_id: owner,
                label,
                masked_pan: masked,
                share_basis_points: share,
            })
            .collect())
    }

    pub fn dashboard(&self) -> Result<Dashboard> {
        let sessions = self.index.list_sessions()?;
        // Only submitted sessions contribute to active invested totals. VOIDED stays auditable elsewhere.
        let active: Vec<_> = sessions
            .iter()
            .filter(|(_, _, _, status)| status.as_str() == "SUBMITTED")
            .collect();
        let total_invested: i64 = active.iter().map(|(_, _, paise, _)| *paise).sum();
        let member_count = self.index.list_members()?.len() as u32;
        let friend_count = self.index.list_active_friends()?.len() as u32;
        let submitted = active.len();
        Ok(Dashboard {
            total_planned_paise: total_invested,
            submitted_session_count: submitted as u32,
            member_count,
            friend_count,
            profit_paise: 0, // profit remains unavailable until real allotment records
        })
    }

    /// Owner correction: void a submitted investment session so it no longer affects active totals.
    /// Append-only; original events remain. Does not access PAN or contact registrars.
    pub fn void_submitted_session(&self, req: VoidSessionRequest) -> Result<VoidSessionResponse> {
        if !req.owner_affirmed {
            return Err(ServiceError::Invalid(
                "owner affirmation is required to void a session".to_owned(),
            ));
        }
        let reason = req.reason.trim();
        if reason.is_empty() || reason.len() > 200 {
            return Err(ServiceError::Invalid(
                "void reason must be 1 to 200 characters".to_owned(),
            ));
        }
        reject_embedded_pan("void reason", reason)?;

        let sessions = self.index.list_sessions()?;
        let Some((session_id, actor_member_id, declared_capital_paise, status)) = sessions
            .into_iter()
            .find(|(id, _, _, _)| id == &req.session_id)
        else {
            return Err(ServiceError::Invalid("session not found".to_owned()));
        };
        if status != "SUBMITTED" {
            return Err(ServiceError::Invalid(
                "only a submitted session can be voided".to_owned(),
            ));
        }
        if actor_member_id != req.actor_member_id {
            return Err(ServiceError::Invalid(
                "only the session actor may void this session".to_owned(),
            ));
        }
        let owner_ok = self
            .index
            .list_members()?
            .into_iter()
            .any(|(id, _, role, _)| id == req.actor_member_id && role == "OWNER");
        if !owner_ok {
            return Err(ServiceError::Invalid(
                "only the owner may void a submitted session".to_owned(),
            ));
        }

        // Domain transition guard (Submitted → Voided).
        let mut session = InvestmentSession::open(
            &session_id,
            &actor_member_id,
            Money::from_paise(declared_capital_paise.max(1)),
        )?;
        session.mark_submitted();
        session.mark_voided()?;

        let event = EventEnvelope::seal(NewEvent {
            event_id: String::new(),
            aggregate_type: "session".into(),
            aggregate_id: session_id.clone(),
            aggregate_revision: epoch_secs().max(3),
            actor_member_id: req.actor_member_id.clone(),
            device_id: self.device_id.clone(),
            occurred_at: Self::now(),
            app_version: env!("CARGO_PKG_VERSION").into(),
            previous_event_hash: None,
            payload: EventPayload::InvestmentSessionVoided {
                session_id: session_id.clone(),
                reason: reason.to_owned(),
            },
        })?;
        self.append_and_project(&event)?;

        // Best-effort: stop non-terminal allotment jobs for this session (no provider calls).
        for (job_id, _, job_session_id, _, _, job_status) in self.index.list_allotment_jobs()? {
            if job_session_id == session_id
                && !matches!(job_status.as_str(), "COMPLETE" | "CANCELLED")
            {
                let _ = self.index.request_allotment_cancel(&job_id);
            }
        }

        Ok(VoidSessionResponse {
            session_id,
            status: "VOIDED".into(),
            reason: reason.to_owned(),
        })
    }

    // --- allotment ---

    fn append_and_project(&self, event: &EventEnvelope) -> Result<()> {
        self.vault.append_event(event)?;
        self.index.apply_event(event)?;
        Ok(())
    }

    /// Persist a job before any provider work starts. The background worker may
    /// safely reconcile this after process restart.
    pub fn enqueue_allotment_check(
        &self,
        req: StartAllotmentRequest,
    ) -> Result<AllotmentJobReport> {
        let account_ids = self
            .index
            .list_account_ids_for_application(&req.application_id)?;
        if account_ids.is_empty() {
            return Err(ServiceError::Invalid(
                "no allocations found for application".into(),
            ));
        }
        if let Some((job_id, ..)) = self.index.list_allotment_jobs()?.into_iter().rev().find(
            |(_, application_id, _, _, _, status)| {
                application_id == &req.application_id
                    && !matches!(status.as_str(), "COMPLETE" | "CANCELLED")
            },
        ) {
            return self.get_allotment_report(&job_id);
        }
        let job_id = format!("job-{}", uuid::Uuid::now_v7());
        let registrar_input = req
            .registrar_id
            .as_deref()
            .ok_or_else(|| ServiceError::Invalid("registrar is required".into()))?;
        let registrar = sanket_allotment::ProviderRegistry::resolve_registrar(registrar_input)
            .ok_or_else(|| ServiceError::Invalid("unsupported registrar".into()))?;
        let provider = execution_provider_id(self.security_mode, registrar.provider_id);
        let registrar_id = registrar.registrar_id.to_owned();
        let registrar_name = registrar.registrar_name.to_owned();
        let provider_id = provider.as_str().to_owned();
        let official_status_url = Some(registrar.official_status_url.to_owned());
        self.validate_allotment_provider(&provider_id)?;
        if matches!(self.security_mode, RuntimeSecurityMode::ProductionSecure) {
            self.require_lookup_permit(&req.application_id, &provider_id)?;
        }
        reject_embedded_pan("IPO name", &req.ipo_name)?;
        reject_embedded_pan("registrar id", &registrar_id)?;
        reject_embedded_pan("registrar name", &registrar_name)?;
        reject_embedded_pan("provider id", &provider_id)?;
        if let Some(url) = &official_status_url {
            reject_embedded_pan("official status URL", url)?;
        }
        let event = EventEnvelope::seal(NewEvent {
            event_id: String::new(),
            aggregate_type: "allotment_job".into(),
            aggregate_id: job_id.clone(),
            aggregate_revision: 1,
            actor_member_id: req.actor_member_id.clone(),
            device_id: self.device_id.clone(),
            occurred_at: Self::now(),
            app_version: env!("CARGO_PKG_VERSION").into(),
            previous_event_hash: None,
            payload: EventPayload::AllotmentJobCreated {
                job_id: job_id.clone(),
                application_id: req.application_id.clone(),
                session_id: req.session_id,
                ipo_name: req.ipo_name.clone(),
                registrar_id: registrar_id.clone(),
                registrar_name: registrar_name.clone(),
                official_status_url: official_status_url.clone(),
                provider_id: provider_id.clone(),
            },
        })?;
        self.append_and_project(&event)?;
        for (index, account_id) in account_ids.iter().enumerate() {
            let attempt_id = format!("att-{}", uuid::Uuid::now_v7());
            let pending = EventEnvelope::seal(NewEvent {
                event_id: String::new(),
                aggregate_type: "allotment_attempt".into(),
                aggregate_id: attempt_id.clone(),
                aggregate_revision: index as u64 + 1,
                actor_member_id: req.actor_member_id.clone(),
                device_id: self.device_id.clone(),
                occurred_at: Self::now(),
                app_version: env!("CARGO_PKG_VERSION").into(),
                previous_event_hash: None,
                payload: EventPayload::AllotmentAttemptStateUpdated {
                    attempt_id,
                    job_id: job_id.clone(),
                    account_id: account_id.clone(),
                    status: "PENDING".into(),
                    attempt_count: 0,
                    allotted_lots: None,
                    allotted_shares: None,
                    source: "AUTOMATED_PROVIDER".into(),
                    provider_reference: None,
                    safe_message: None,
                    last_attempt_at: String::new(),
                    next_retry_at: None,
                },
            })?;
            self.append_and_project(&pending)?;
        }
        self.get_allotment_report(&job_id)
    }

    /// Compatibility helper for tests/direct callers: enqueue then execute once.
    pub fn start_allotment_check(&self, req: StartAllotmentRequest) -> Result<AllotmentJobReport> {
        let queued = self.enqueue_allotment_check(req)?;
        let _ = self.run_allotment_job_once(&queued.job_id)?;
        self.get_allotment_report(&queued.job_id)
    }

    pub fn run_allotment_job_once(&self, job_id: &str) -> Result<bool> {
        let job = self
            .index
            .allotment_job_execution(job_id)?
            .ok_or_else(|| ServiceError::Invalid("allotment job not found".into()))?;
        let token = uuid::Uuid::now_v7().to_string();
        let now = epoch_secs();
        if !self.index.try_acquire_allotment_lease(
            job_id,
            &self.device_id,
            &token,
            now,
            now.saturating_add(120),
        )? {
            return Ok(false);
        }
        let req = StartAllotmentRequest {
            application_id: job.application_id,
            session_id: job.session_id,
            ipo_name: job.ipo_name,
            actor_member_id: job.actor_member_id,
            registrar_id: Some(job.registrar_id),
        };
        let result = self.execute_allotment_check(req, job_id.to_owned());
        let _ = self.index.release_allotment_lease(job_id, &token);
        result.map(|_| true)
    }

    pub fn resumable_allotment_job_ids(&self) -> Result<Vec<String>> {
        Ok(self.index.resumable_allotment_job_ids(epoch_secs())?)
    }

    pub fn reconcile_provider_runtime_after_restart(&self) -> Result<()> {
        Ok(self
            .index
            .reconcile_ephemeral_allotment_state_after_restart()?)
    }

    pub fn cancel_allotment_job(&self, job_id: &str) -> Result<bool> {
        Ok(self.index.request_allotment_cancel(job_id)?)
    }

    pub fn list_allotment_candidates(&self) -> Result<Vec<AllotmentCandidateRow>> {
        let jobs = self.index.list_allotment_jobs()?;
        let mut candidates = Vec::new();
        for application in self.index.list_submitted_applications()? {
            let descriptor =
                sanket_allotment::ProviderRegistry::resolve_registrar(&application.registrar_id);
            let (provider_id, provider_name, provider_health) = descriptor
                .map(|provider| {
                    let provider_id =
                        execution_provider_id(self.security_mode, provider.provider_id);
                    (
                        provider_id.as_str().to_owned(),
                        provider.registrar_name.to_owned(),
                        provider_health(provider_id).to_string(),
                    )
                })
                .unwrap_or_else(|| {
                    (
                        "unsupported".into(),
                        "Unsupported registrar".into(),
                        "UNKNOWN".into(),
                    )
                });
            let latest_job = jobs
                .iter()
                .rev()
                .find(|(_, application_id, ..)| application_id == &application.application_id);
            let (pending_count, final_count, last_checked, overall_job_state) =
                if let Some((job_id, ..)) = latest_job {
                    let report = self.get_allotment_report(job_id)?;
                    let final_count = report
                        .accounts
                        .iter()
                        .filter(|row| {
                            matches!(
                                row.status.as_str(),
                                "ALLOTTED"
                                    | "NOT_ALLOTTED"
                                    | "NOT_FOUND"
                                    | "MANUAL_RESULT"
                                    | "CANCELLED"
                            )
                        })
                        .count() as u32;
                    let last_checked = report
                        .accounts
                        .iter()
                        .filter_map(|row| row.checked_at.clone())
                        .max();
                    (
                        report.accounts.len() as u32 - final_count,
                        final_count,
                        last_checked,
                        report.status,
                    )
                } else {
                    (application.account_count, 0, None, "READY_TO_CHECK".into())
                };
            candidates.push(AllotmentCandidateRow {
                application_id: application.application_id,
                session_id: application.session_id,
                ipo_name: application.ipo_name,
                planned_amount_paise: application.planned_amount_paise,
                account_count: application.account_count,
                registrar_id: application.registrar_id,
                registrar_name: application.registrar_name,
                provider_id,
                provider_name,
                official_status_url: application.official_status_url,
                provider_health,
                expected_allotment_date: application.expected_allotment_date,
                pending_count,
                final_count,
                last_checked,
                overall_job_state,
            });
        }
        Ok(candidates)
    }

    fn allotment_report_row(
        &self,
        job_id: &str,
        application_id: &str,
        registrar_id: &str,
        provider_id: &str,
        account_id: &str,
    ) -> Result<AllotmentReportRow> {
        let attempt = self
            .index
            .allotment_attempt(job_id, account_id)?
            .ok_or_else(|| ServiceError::Invalid("allotment attempt not found".into()))?;
        let (display_name, account_kind) = self.index.display_label_for_account(account_id)?;
        let masked_pan = self
            .index
            .masked_pan_for_account(account_id)?
            .unwrap_or_else(|| "[MASKED]".into());
        let application_amount_paise = self
            .index
            .allocation_amount_for_account(application_id, account_id)?
            .unwrap_or(0);
        let (profit_basis, estimated_profit_paise, profit_provenance) = self
            .index
            .estimated_profit_for_account(application_id, account_id)?
            .unwrap_or_else(|| ("UNAVAILABLE".into(), None, None));
        let provenance = if attempt.source == "MANUAL" {
            "OWNER_REPORTED_MANUAL"
        } else if matches!(
            attempt.status.as_str(),
            "ALLOTTED" | "NOT_ALLOTTED" | "NOT_FOUND"
        ) {
            "CONFIRMED_PROVIDER_RESPONSE"
        } else {
            "PROVIDER_OPERATIONAL_STATE"
        };
        let human_verification_state = match attempt.status.as_str() {
            "NEEDS_HUMAN_VERIFICATION" => Some("REQUIRED".into()),
            "VERIFICATION_REQUIRED_REFRESH" => Some("REFRESH_REQUIRED".into()),
            _ => None,
        };
        Ok(AllotmentReportRow {
            attempt_id: attempt.id,
            account_id: account_id.to_owned(),
            display_name,
            account_kind,
            masked_pan,
            status: attempt.status,
            allotted_lots: attempt.allotted_lots.map(|value| value as u32),
            allotted_shares: attempt.allotted_shares.map(|value| value as u64),
            provider_id: provider_id.to_owned(),
            registrar_id: registrar_id.to_owned(),
            source: attempt.source,
            provenance: provenance.into(),
            application_amount_paise,
            checked_at: attempt.last_attempt_at.filter(|value| !value.is_empty()),
            safe_provider_reference: attempt.provider_reference,
            safe_message: attempt.safe_message,
            next_retry_at: attempt.next_retry_at,
            human_verification_state,
            estimated_profit_paise,
            profit_basis,
            profit_provenance,
        })
    }

    fn execute_allotment_check(
        &self,
        req: StartAllotmentRequest,
        job_id: String,
    ) -> Result<AllotmentJobReport> {
        let accounts = self
            .index
            .list_account_ids_for_application(&req.application_id)?;
        if accounts.is_empty() {
            return Err(ServiceError::Invalid(
                "no allocations found for application".into(),
            ));
        }

        let job = self
            .index
            .allotment_job_execution(&job_id)?
            .ok_or_else(|| ServiceError::Invalid("allotment job not found".into()))?;
        let registrar_id = job.registrar_id.clone();
        let registrar_name = job.registrar_name.clone();
        let provider_id = job.provider_id.clone();
        // Gate 4F condition C: the registrar URL must come from the persisted
        // registry-resolved job. A missing URL fails closed — never defaulted
        // to another registrar's portal.
        let official_url = job
            .official_status_url
            .clone()
            .filter(|u| !u.is_empty())
            .ok_or_else(|| {
                ServiceError::Invalid(format!(
                    "allotment job {job_id} has no official status URL for registrar {registrar_id}"
                ))
            })?;

        let _job = sanket_allotment::AllotmentCheckJob::create(
            job_id.clone(),
            req.application_id.clone(),
            req.session_id.clone(),
            req.ipo_name.clone(),
            registrar_id.clone(),
            registrar_name.clone(),
            provider_id.clone(),
        )
        .map_err(|e| ServiceError::Invalid(e.to_string()))?;

        let running_event = EventEnvelope::seal(NewEvent {
            event_id: String::new(),
            aggregate_type: "allotment_job".into(),
            aggregate_id: job_id.clone(),
            aggregate_revision: 2,
            actor_member_id: req.actor_member_id.clone(),
            device_id: self.device_id.clone(),
            occurred_at: Self::now(),
            app_version: env!("CARGO_PKG_VERSION").into(),
            previous_event_hash: None,
            payload: EventPayload::AllotmentJobStatusChanged {
                job_id: job_id.clone(),
                status: "RUNNING".into(),
            },
        })?;
        self.append_and_project(&running_event)?;

        let provider_kind = sanket_allotment::ProviderRegistry::resolve(&provider_id)
            .ok_or_else(|| ServiceError::Invalid("unsupported allotment provider".into()))?;
        let provider = allotment_provider(provider_kind);
        let key_provider = self.key_provider_for_sensitive()?;
        let mut sensitive =
            sanket_identity_security::SensitiveIdentityService::new_boxed(key_provider);
        let mut report_rows = Vec::new();
        let mut rev = 3u64;
        let policy = sanket_allotment::ProviderRatePolicy::for_provider(provider_kind);
        let mut cancelled = false;
        let mut lookup_permit = None;
        let mut lookup_authorization_consumed = false;

        for account_id in &accounts {
            if self
                .index
                .allotment_job_execution(&job_id)?
                .is_some_and(|row| row.cancel_requested)
            {
                cancelled = true;
                break;
            }
            let previous = self.index.allotment_attempt(&job_id, account_id)?;
            let now = epoch_secs();
            let retry_due = previous
                .as_ref()
                .and_then(|a| a.next_retry_at.as_deref())
                .and_then(|s| s.parse::<u64>().ok())
                .is_none_or(|at| at <= now);
            let paused = previous.as_ref().is_some_and(|a| {
                matches!(
                    a.status.as_str(),
                    "ALLOTTED"
                        | "NOT_ALLOTTED"
                        | "NOT_FOUND"
                        | "MANUAL_RESULT"
                        | "NEEDS_HUMAN_VERIFICATION"
                        | "UNKNOWN"
                ) || a.attempt_count >= policy.max_attempts
                    || !retry_due
            });
            if paused {
                report_rows.push(self.allotment_report_row(
                    &job_id,
                    &req.application_id,
                    &registrar_id,
                    &provider_id,
                    account_id,
                )?);
                continue;
            }
            let attempt_id = previous
                .as_ref()
                .map(|a| a.id.clone())
                .unwrap_or_else(|| format!("att-{}", uuid::Uuid::now_v7()));
            let attempt_count = previous
                .as_ref()
                .map(|a| a.attempt_count.saturating_add(1))
                .unwrap_or(1);
            allotment_rate_limiter().wait_turn(&provider_id);
            let masked = self
                .index
                .masked_pan_for_account(account_id)?
                .unwrap_or_else(|| "[MASKED]".into());
            let ctx = sanket_allotment::AllotmentLookupContext {
                job_id: job_id.clone(),
                attempt_id: attempt_id.clone(),
                account_id: account_id.clone(),
                issue: sanket_allotment::RegistrarIssue {
                    registrar_id: registrar_id.clone(),
                    registrar_name: registrar_name.clone(),
                    official_status_url: Some(official_url.clone()),
                    issue_code: None,
                    ipo_name: req.ipo_name.clone(),
                },
            };
            let source = if provider_kind == sanket_allotment::ProviderId::KfintechFixture {
                "FIXTURE"
            } else {
                "AUTOMATED_PROVIDER"
            };
            let prepare_denied = if provider_kind != sanket_allotment::ProviderId::KfintechFixture
                && lookup_permit.is_none()
            {
                match self.require_lookup_permit(&req.application_id, &provider_id) {
                    Ok(permit) => {
                        lookup_permit = Some(permit);
                        false
                    }
                    Err(_error)
                        if self.security_mode == RuntimeSecurityMode::DevelopmentSynthetic =>
                    {
                        true
                    }
                    Err(error) => return Err(error),
                }
            } else {
                false
            };
            let prepared = if prepare_denied {
                let error = if provider_kind == sanket_allotment::ProviderId::BigshareLive {
                    sanket_allotment::ProviderError::NeedsHuman(
                        "bigshare portal requires official human verification".into(),
                    )
                } else {
                    sanket_allotment::ProviderError::Unavailable(
                        "real investor lookup authorization is required".into(),
                    )
                };
                Err(error)
            } else {
                provider.prepare_lookup(&ctx)
            };
            let (status, lots, shares, pref, source) = match prepared {
                Ok(Some(result)) => (
                    result.status().as_str().to_owned(),
                    result.allotted_lots(),
                    result.allotted_shares(),
                    result.provider_reference().map(str::to_owned),
                    source.to_owned(),
                ),
                Err(error) => (
                    error.to_status().as_str().to_owned(),
                    None,
                    None,
                    None,
                    source.to_owned(),
                ),
                Ok(None) => {
                    let permit = if provider_kind == sanket_allotment::ProviderId::KfintechFixture {
                        None
                    } else if let Some(permit) = lookup_permit.clone() {
                        Some(permit)
                    } else {
                        match self.require_lookup_permit(&req.application_id, &provider_id) {
                            Ok(permit) => Some(permit),
                            Err(_error)
                                if self.security_mode
                                    == RuntimeSecurityMode::DevelopmentSynthetic =>
                            {
                                None
                            }
                            Err(error) => return Err(error),
                        }
                    };
                    if provider_kind != sanket_allotment::ProviderId::KfintechFixture
                        && permit.is_none()
                    {
                        (
                            "PROVIDER_UNAVAILABLE".into(),
                            None,
                            None,
                            None,
                            source.to_owned(),
                        )
                    } else {
                        if provider_kind != sanket_allotment::ProviderId::KfintechFixture {
                            self.validate_final_lookup_permit(
                                permit.as_ref(),
                                &req.application_id,
                                &provider_id,
                            )?;
                        }
                        if let Some(permit) = permit.as_ref() {
                            if !lookup_authorization_consumed {
                                self.consume_lookup_authorization(permit)?;
                                lookup_authorization_consumed = true;
                                lookup_permit = Some(permit.clone());
                            }
                        }
                        let envelope = self
                            .vault
                            .load_member_identity(account_id)
                            .or_else(|_| self.vault.load_friend_identity(account_id))?;
                        let masked_pan =
                            sanket_identity_security::MaskedPan::from_display(masked.clone())
                                .unwrap_or_else(|_| {
                                    sanket_identity_security::MaskedPan::from_parts("AAAAA", "A")
                                });
                        let record = sanket_identity_security::SensitiveIdentityRecord::from_stored(
                            account_id.clone(),
                            masked_pan,
                            envelope,
                        );
                        let result = sensitive
                            .with_pan(
                                &record,
                                sanket_identity_security::SensitivePurpose::AllotmentCheck,
                                &req.actor_member_id,
                                |pan_str| {
                                    let pan = Pan::parse(pan_str).map_err(|e| e.to_string())?;
                                    let provider_result = match permit.as_ref() {
                                        Some(permit) => provider.check_allotment_with_permit(
                                            &req.application_id,
                                            &ctx,
                                            &pan,
                                            permit,
                                        ),
                                        None => provider.check_allotment(&ctx, &pan),
                                    };
                                    Ok(match provider_result {
                                        Ok(result) => (
                                            result.status().as_str().to_owned(),
                                            result.allotted_lots(),
                                            result.allotted_shares(),
                                            result.provider_reference().map(str::to_owned),
                                            source.to_owned(),
                                        ),
                                        Err(error) => (
                                            error.to_status().as_str().to_owned(),
                                            None,
                                            None,
                                            None,
                                            source.to_owned(),
                                        ),
                                    })
                                },
                            )
                            .map_err(|error| ServiceError::Invalid(error.to_string()))?
                            .map_err(ServiceError::Invalid)?;
                        if let Some(audit) = sensitive.take_last_audit() {
                            let audit_event = EventEnvelope::seal(NewEvent {
                                event_id: String::new(),
                                aggregate_type: "sensitive_identity".into(),
                                aggregate_id: account_id.clone(),
                                aggregate_revision: attempt_count as u64,
                                actor_member_id: req.actor_member_id.clone(),
                                device_id: self.device_id.clone(),
                                occurred_at: audit.occurred_at,
                                app_version: env!("CARGO_PKG_VERSION").into(),
                                previous_event_hash: None,
                                payload: EventPayload::SensitiveIdentityAccessed {
                                    account_id: audit.account_id,
                                    purpose: audit.purpose,
                                },
                            })?;
                            self.append_and_project(&audit_event)?;
                        }
                        result
                    }
                }
            };

            let retryable = matches!(
                status.as_str(),
                "RATE_LIMITED" | "PROVIDER_UNAVAILABLE" | "RETRYABLE_ERROR" | "PENDING"
            );
            let next_retry_at = if retryable && attempt_count < policy.max_attempts {
                Some(
                    epoch_secs()
                        .saturating_add(policy.next_backoff_ms(attempt_count) / 1_000)
                        .to_string(),
                )
            } else {
                None
            };
            let safe_message = match status.as_str() {
                "RATE_LIMITED" => Some("Provider rate limit; retry scheduled".into()),
                "PROVIDER_UNAVAILABLE" => Some("Provider unavailable; retry scheduled".into()),
                "RETRYABLE_ERROR" => Some("Temporary provider error; retry scheduled".into()),
                "NEEDS_HUMAN_VERIFICATION" => {
                    Some("Official registrar verification is required".into())
                }
                "UNKNOWN" => Some("Provider response could not be confirmed".into()),
                _ => None,
            };
            let attempt_event = EventEnvelope::seal(NewEvent {
                event_id: String::new(),
                aggregate_type: "allotment_attempt".into(),
                aggregate_id: attempt_id.clone(),
                aggregate_revision: attempt_count as u64,
                actor_member_id: req.actor_member_id.clone(),
                device_id: self.device_id.clone(),
                occurred_at: Self::now(),
                app_version: env!("CARGO_PKG_VERSION").into(),
                previous_event_hash: None,
                payload: EventPayload::AllotmentAttemptStateUpdated {
                    attempt_id: attempt_id.clone(),
                    job_id: job_id.clone(),
                    account_id: account_id.clone(),
                    status: status.clone(),
                    attempt_count,
                    allotted_lots: lots,
                    allotted_shares: shares,
                    source: source.clone(),
                    provider_reference: pref.clone(),
                    safe_message,
                    last_attempt_at: epoch_secs().to_string(),
                    next_retry_at,
                },
            })?;
            self.append_and_project(&attempt_event)?;
            rev += 1;
            eprintln!(
                "allotment provider={} job_id={} account_id={} attempt={} state={}",
                provider_id, job_id, account_id, attempt_count, status
            );

            report_rows.push(self.allotment_report_row(
                &job_id,
                &req.application_id,
                &registrar_id,
                &provider_id,
                account_id,
            )?);
        }

        let mut has_retryable = false;
        for account_id in &accounts {
            if let Some(a) = self.index.allotment_attempt(&job_id, account_id)? {
                if matches!(
                    a.status.as_str(),
                    "RATE_LIMITED" | "PROVIDER_UNAVAILABLE" | "RETRYABLE_ERROR" | "PENDING"
                ) && a.attempt_count < policy.max_attempts
                {
                    has_retryable = true;
                }
            }
        }
        let final_count = report_rows
            .iter()
            .filter(|row| {
                matches!(
                    row.status.as_str(),
                    "ALLOTTED" | "NOT_ALLOTTED" | "NOT_FOUND" | "MANUAL_RESULT" | "CANCELLED"
                )
            })
            .count() as u32;
        let pending_count = report_rows.len() as u32 - final_count;
        let final_status = if cancelled {
            "CANCELLED"
        } else if final_count > 0 && pending_count > 0 {
            "PARTIALLY_COMPLETE"
        } else if report_rows
            .iter()
            .any(|r| r.status == "NEEDS_HUMAN_VERIFICATION")
        {
            "NEEDS_HUMAN_VERIFICATION"
        } else if has_retryable || report_rows.len() < accounts.len() {
            "PARTIALLY_COMPLETE"
        } else if report_rows.iter().all(|r| {
            matches!(
                r.status.as_str(),
                "ALLOTTED" | "NOT_ALLOTTED" | "NOT_FOUND" | "MANUAL_RESULT"
            )
        }) {
            "COMPLETE"
        } else {
            "COMPLETE_WITH_UNCONFIRMED"
        };
        let final_event = EventEnvelope::seal(NewEvent {
            event_id: String::new(),
            aggregate_type: "allotment_job".into(),
            aggregate_id: job_id.clone(),
            aggregate_revision: rev,
            actor_member_id: req.actor_member_id.clone(),
            device_id: self.device_id.clone(),
            occurred_at: Self::now(),
            app_version: env!("CARGO_PKG_VERSION").into(),
            previous_event_hash: None,
            payload: EventPayload::AllotmentJobStatusChanged {
                job_id: job_id.clone(),
                status: final_status.into(),
            },
        })?;
        self.append_and_project(&final_event)?;

        let checked_at = report_rows
            .iter()
            .filter_map(|row| row.checked_at.clone())
            .max();
        let _ = job; // domain object validated construction
        Ok(AllotmentJobReport {
            job_id,
            application_id: req.application_id,
            ipo_name: req.ipo_name,
            registrar_id,
            registrar_name,
            provider_id,
            status: final_status.into(),
            official_status_url: Some(official_url),
            checked_at,
            final_count,
            pending_count,
            accounts: report_rows,
        })
    }

    pub fn get_allotment_report(&self, job_id: &str) -> Result<AllotmentJobReport> {
        let job = self
            .index
            .allotment_job_execution(job_id)?
            .ok_or_else(|| ServiceError::Invalid("allotment job not found".into()))?;
        let attempts = self.index.list_allotment_attempts(&job.id)?;
        let mut accounts = Vec::new();
        for (_, account_id, ..) in attempts {
            accounts.push(self.allotment_report_row(
                &job.id,
                &job.application_id,
                &job.registrar_id,
                &job.provider_id,
                &account_id,
            )?);
        }
        let checked_at = accounts
            .iter()
            .filter_map(|row| row.checked_at.clone())
            .max();
        let final_count = accounts
            .iter()
            .filter(|row| {
                matches!(
                    row.status.as_str(),
                    "ALLOTTED" | "NOT_ALLOTTED" | "NOT_FOUND" | "MANUAL_RESULT" | "CANCELLED"
                )
            })
            .count() as u32;
        let pending_count = accounts.len() as u32 - final_count;
        Ok(AllotmentJobReport {
            job_id: job.id,
            application_id: job.application_id,
            ipo_name: job.ipo_name,
            registrar_id: job.registrar_id,
            registrar_name: job.registrar_name,
            provider_id: job.provider_id,
            status: job.status,
            official_status_url: job.official_status_url,
            checked_at,
            final_count,
            pending_count,
            accounts,
        })
    }

    pub fn record_manual_allotment_result(
        &self,
        req: ManualAllotmentRequest,
    ) -> Result<AllotmentReportRow> {
        let job = self
            .index
            .allotment_job_execution(&req.job_id)?
            .ok_or_else(|| ServiceError::Invalid("allotment job not found".into()))?;
        if job.actor_member_id != req.actor_member_id {
            return Err(ServiceError::Invalid(
                "manual result actor does not own the allotment job".into(),
            ));
        }
        let account_ids = self
            .index
            .list_account_ids_for_application(&job.application_id)?;
        if !account_ids
            .iter()
            .any(|account_id| account_id == &req.account_id)
        {
            return Err(ServiceError::Invalid(
                "manual result account is not part of the allotment job".into(),
            ));
        }
        if let Some(note) = &req.note {
            reject_embedded_pan("manual result note", note)?;
        }
        let previous = self.index.allotment_attempt(&req.job_id, &req.account_id)?;
        let attempt_id = previous
            .as_ref()
            .map(|attempt| attempt.id.clone())
            .unwrap_or_else(|| format!("att-{}", uuid::Uuid::now_v7()));
        let attempt_count = previous
            .as_ref()
            .map(|attempt| attempt.attempt_count.saturating_add(1))
            .unwrap_or(1);
        let reported_outcome = if req.explicit_not_allotted {
            "NOT_ALLOTTED"
        } else if req.allotted_shares.unwrap_or(0) > 0 || req.allotted_lots.unwrap_or(0) > 0 {
            "ALLOTTED"
        } else {
            "UNKNOWN"
        };
        let event = EventEnvelope::seal(NewEvent {
            event_id: String::new(),
            aggregate_type: "allotment_attempt".into(),
            aggregate_id: attempt_id.clone(),
            aggregate_revision: 1,
            actor_member_id: req.actor_member_id.clone(),
            device_id: self.device_id.clone(),
            occurred_at: Self::now(),
            app_version: env!("CARGO_PKG_VERSION").into(),
            previous_event_hash: None,
            payload: EventPayload::AllotmentAttemptStateUpdated {
                attempt_id: attempt_id.clone(),
                job_id: req.job_id.clone(),
                account_id: req.account_id.clone(),
                status: "MANUAL_RESULT".into(),
                attempt_count,
                allotted_lots: req.allotted_lots,
                allotted_shares: req.allotted_shares,
                source: "MANUAL".into(),
                provider_reference: req.note.clone(),
                safe_message: Some(format!(
                    "Manually recorded by an authorized local actor: {reported_outcome}"
                )),
                last_attempt_at: epoch_secs().to_string(),
                next_retry_at: None,
            },
        })?;
        self.append_and_project(&event)?;
        let complete = account_ids.iter().all(|account_id| {
            self.index
                .allotment_attempt(&req.job_id, account_id)
                .ok()
                .flatten()
                .is_some_and(|attempt| {
                    matches!(
                        attempt.status.as_str(),
                        "ALLOTTED" | "NOT_ALLOTTED" | "NOT_FOUND" | "MANUAL_RESULT"
                    )
                })
        });
        let status_event = EventEnvelope::seal(NewEvent {
            event_id: String::new(),
            aggregate_type: "allotment_job".into(),
            aggregate_id: req.job_id.clone(),
            aggregate_revision: attempt_count as u64 + 2,
            actor_member_id: req.actor_member_id.clone(),
            device_id: self.device_id.clone(),
            occurred_at: Self::now(),
            app_version: env!("CARGO_PKG_VERSION").into(),
            previous_event_hash: None,
            payload: EventPayload::AllotmentJobStatusChanged {
                job_id: req.job_id.clone(),
                status: if complete {
                    "COMPLETE".into()
                } else {
                    "COMPLETE_WITH_UNCONFIRMED".into()
                },
            },
        })?;
        self.append_and_project(&status_event)?;
        self.allotment_report_row(
            &req.job_id,
            &job.application_id,
            &job.registrar_id,
            &job.provider_id,
            &req.account_id,
        )
    }

    pub fn estimate_profit(&self, req: EstimateProfitRequest) -> Result<EstimatedProfitDto> {
        reject_embedded_pan("profit source", &req.source)?;
        reject_embedded_pan("profit as-of value", &req.as_of)?;
        if let Some(note) = &req.note {
            reject_embedded_pan("profit note", note)?;
        }
        if !self
            .index
            .list_account_ids_for_application(&req.application_id)?
            .iter()
            .any(|account_id| account_id == &req.account_id)
        {
            return Err(ServiceError::Invalid(
                "estimated-profit account is not part of the application".into(),
            ));
        }
        let basis = match req.basis.to_ascii_uppercase().as_str() {
            "ACTUAL_LISTING_PRICE" => sanket_allotment::ProfitPriceBasis::ActualListingPrice,
            "CURRENT_MARKET_PRICE" => sanket_allotment::ProfitPriceBasis::CurrentMarketPrice,
            "OWNER_EXPECTED_PRICE" => sanket_allotment::ProfitPriceBasis::OwnerExpectedPrice,
            "PUBLIC_ESTIMATE" => sanket_allotment::ProfitPriceBasis::PublicEstimate,
            _ => sanket_allotment::ProfitPriceBasis::Unavailable,
        };
        if !matches!(
            basis,
            sanket_allotment::ProfitPriceBasis::Unavailable
                | sanket_allotment::ProfitPriceBasis::OwnerExpectedPrice
        ) && (req.source.trim().is_empty() || req.as_of.trim().is_empty())
        {
            return Err(ServiceError::Invalid(
                "external price bases require source and as-of provenance".into(),
            ));
        }
        let est = if matches!(basis, sanket_allotment::ProfitPriceBasis::Unavailable)
            || req.reference_price_paise.is_none()
            || req.issue_price_paise.is_none()
        {
            sanket_allotment::EstimatedProfit::unavailable(req.allotted_shares)
        } else {
            sanket_allotment::EstimatedProfit::compute(
                basis,
                req.allotted_shares,
                Money::from_paise(req.reference_price_paise.unwrap_or(0)),
                Money::from_paise(req.issue_price_paise.unwrap_or(0)),
                req.note.clone(),
            )
        };
        let dto = EstimatedProfitDto {
            basis: est.basis.as_str().into(),
            reference_price_paise: est.reference_price_paise,
            issue_price_paise: est.issue_price_paise,
            allotted_shares: est.allotted_shares,
            estimated_profit_paise: est.estimated_profit_paise,
            note: est.note.clone(),
            is_realized: false,
        };
        let event = EventEnvelope::seal(NewEvent {
            event_id: String::new(),
            aggregate_type: "estimated_profit".into(),
            aggregate_id: format!("{}:{}", req.application_id, req.account_id),
            aggregate_revision: 1,
            actor_member_id: req.actor_member_id,
            device_id: self.device_id.clone(),
            occurred_at: Self::now(),
            app_version: env!("CARGO_PKG_VERSION").into(),
            previous_event_hash: None,
            payload: EventPayload::EstimatedProfitUpdated {
                application_id: req.application_id,
                account_id: req.account_id,
                basis: dto.basis.clone(),
                reference_price_paise: dto.reference_price_paise,
                issue_price_paise: dto.issue_price_paise,
                allotted_shares: dto.allotted_shares,
                estimated_profit_paise: dto.estimated_profit_paise,
                provenance: if req.source.trim().is_empty() {
                    None
                } else {
                    Some(req.source)
                },
                observed_at: req.as_of,
            },
        })?;
        self.append_and_project(&event)?;
        Ok(dto)
    }

    fn application_provider_id(&self, application_id: &str) -> Result<String> {
        let application = self
            .index
            .list_submitted_applications()?
            .into_iter()
            .find(|application| application.application_id == application_id)
            .ok_or_else(|| ServiceError::Invalid("submitted application not found".into()))?;
        let registrar =
            sanket_allotment::ProviderRegistry::resolve_registrar(&application.registrar_id)
                .ok_or_else(|| ServiceError::Invalid("unsupported registrar".into()))?;
        Ok(
            execution_provider_id(self.security_mode, registrar.provider_id)
                .as_str()
                .to_owned(),
        )
    }

    fn lookup_authorization_state(
        &self,
        application_id: &str,
        provider_id: &str,
    ) -> Result<LookupAuthorizationState> {
        let events = self.vault.list_events()?;
        let owner_ids: Vec<&str> = events
            .iter()
            .filter_map(|event| match event.payload() {
                EventPayload::MemberCreated {
                    member_id, role, ..
                } if *role == Role::Owner => Some(member_id.as_str()),
                _ => None,
            })
            .collect();
        let mut grants = Vec::new();
        let mut consumed = Vec::new();
        for event in &events {
            match event.payload() {
                EventPayload::LookupAuthorizationGranted {
                    authorization_id,
                    application_id: event_application_id,
                    provider_id: event_provider_id,
                    expiry_time,
                } if event_application_id == application_id
                    && event_provider_id == provider_id
                    && owner_ids.contains(&event.actor_member_id()) =>
                {
                    grants.push(LookupAuthorizationRecord {
                        authorization_id: authorization_id.clone(),
                        application_id: event_application_id.clone(),
                        provider_id: event_provider_id.clone(),
                        expiry_time: expiry_time.clone(),
                        expiry_epoch: expiry_time.parse().unwrap_or(0),
                    });
                }
                EventPayload::LookupAuthorizationConsumed {
                    authorization_id,
                    application_id: event_application_id,
                    provider_id: event_provider_id,
                    ..
                } if event_application_id == application_id && event_provider_id == provider_id => {
                    consumed.push(authorization_id.as_str());
                }
                _ => {}
            }
        }
        let record = grants
            .into_iter()
            .rev()
            .find(|grant| !consumed.contains(&grant.authorization_id.as_str()));
        let Some(record) = record else {
            return Ok(LookupAuthorizationState {
                status: "NOT_GRANTED",
                record: None,
            });
        };
        let status = if record.expiry_epoch > epoch_secs() {
            "ACTIVE"
        } else {
            "EXPIRED"
        };
        Ok(LookupAuthorizationState {
            status,
            record: Some(record),
        })
    }

    fn require_lookup_permit(
        &self,
        application_id: &str,
        provider_id: &str,
    ) -> Result<sanket_allotment::RealInvestorLookupPermit> {
        if !matches!(self.security_mode, RuntimeSecurityMode::ProductionSecure) {
            return Err(ServiceError::Invalid(
                "real investor lookup requires PRODUCTION_SECURE".into(),
            ));
        }
        let security = self.security_status()?;
        if security.key_provider != "os-keyring" || !security.real_pan_allowed {
            return Err(ServiceError::Invalid(
                "real investor lookup requires an active OS keyring".into(),
            ));
        }
        if provider_id.ends_with("-fixture") {
            return Err(ServiceError::Invalid(
                "real investor lookup cannot use a fixture provider".into(),
            ));
        }
        let state = self.lookup_authorization_state(application_id, provider_id)?;
        let record = state.record.ok_or_else(|| {
            ServiceError::Invalid("real investor lookup authorization is not granted".into())
        })?;
        if state.status != "ACTIVE" {
            return Err(ServiceError::Invalid(
                "real investor lookup authorization is expired".into(),
            ));
        }
        sanket_allotment::RealInvestorLookupPermit::new(
            record.authorization_id,
            record.application_id,
            record.provider_id,
        )
        .map_err(|error| ServiceError::Invalid(error.to_string()))
    }

    fn validate_final_lookup_permit(
        &self,
        permit: Option<&sanket_allotment::RealInvestorLookupPermit>,
        application_id: &str,
        provider_id: &str,
    ) -> Result<()> {
        if !matches!(self.security_mode, RuntimeSecurityMode::ProductionSecure) {
            return Err(ServiceError::Invalid(
                "real investor lookup requires PRODUCTION_SECURE".into(),
            ));
        }
        let security = self.security_status()?;
        if security.key_provider != "os-keyring" || !security.real_pan_allowed {
            return Err(ServiceError::Invalid(
                "real investor lookup requires an active OS keyring".into(),
            ));
        }
        let permit = permit.ok_or_else(|| {
            ServiceError::Invalid("real investor lookup authorization is not granted".into())
        })?;
        if !permit.matches(application_id, provider_id) {
            return Err(ServiceError::Invalid(
                "real investor lookup permit scope mismatch".into(),
            ));
        }
        Ok(())
    }

    fn consume_lookup_authorization(
        &self,
        permit: &sanket_allotment::RealInvestorLookupPermit,
    ) -> Result<()> {
        let state =
            self.lookup_authorization_state(permit.application_id(), permit.provider_id())?;
        let record = state.record.ok_or_else(|| {
            ServiceError::Invalid("real investor lookup authorization is not granted".into())
        })?;
        if state.status != "ACTIVE" || record.authorization_id != permit.authorization_id() {
            return Err(ServiceError::Invalid(
                "real investor lookup authorization is no longer active".into(),
            ));
        }
        let event = EventEnvelope::seal(NewEvent {
            event_id: String::new(),
            aggregate_type: "lookup_authorization".into(),
            aggregate_id: permit.authorization_id().into(),
            aggregate_revision: 2,
            actor_member_id: "SYSTEM".into(),
            device_id: self.device_id.clone(),
            occurred_at: Self::now(),
            app_version: env!("CARGO_PKG_VERSION").into(),
            previous_event_hash: None,
            payload: EventPayload::LookupAuthorizationConsumed {
                authorization_id: permit.authorization_id().into(),
                application_id: permit.application_id().into(),
                provider_id: permit.provider_id().into(),
                timestamp: Self::now(),
            },
        })?;
        self.append_and_project(&event)
    }

    pub fn get_lookup_authorization_status(
        &self,
        application_id: &str,
    ) -> Result<LookupAuthorizationStatusDto> {
        let provider_id = self.application_provider_id(application_id)?;
        let state = self.lookup_authorization_state(application_id, &provider_id)?;
        Ok(LookupAuthorizationStatusDto {
            provider_id,
            status: state.status.into(),
            authorization_id: state
                .record
                .as_ref()
                .map(|record| record.authorization_id.clone()),
            expiry_time: state.record.map(|record| record.expiry_time),
        })
    }

    pub fn authorize_real_investor_lookup(
        &self,
        req: LookupAuthorizationRequest,
    ) -> Result<LookupAuthorizationStatusDto> {
        if !req.owner_affirmed {
            return Err(ServiceError::Invalid(
                "explicit owner confirmation is required".into(),
            ));
        }
        if !matches!(self.security_mode, RuntimeSecurityMode::ProductionSecure) {
            return Err(ServiceError::Invalid(
                "real investor lookup requires PRODUCTION_SECURE".into(),
            ));
        }
        let security = self.security_status()?;
        if security.key_provider != "os-keyring" || !security.real_pan_allowed {
            return Err(ServiceError::Invalid(
                "real investor lookup requires an active OS keyring".into(),
            ));
        }
        let events = self.vault.list_events()?;
        let owner_ok = events.iter().any(|event| {
            event.actor_member_id() == req.actor_member_id.as_str()
                && matches!(
                    event.payload(),
                    EventPayload::MemberCreated {
                        member_id,
                        role: Role::Owner,
                        ..
                    } if member_id == &req.actor_member_id
                )
        });
        if !owner_ok {
            return Err(ServiceError::Invalid(
                "only an owner may authorize a real investor lookup".into(),
            ));
        }
        let provider_id = self.application_provider_id(&req.application_id)?;
        self.validate_allotment_provider(&provider_id)?;
        let state = self.lookup_authorization_state(&req.application_id, &provider_id)?;
        if state.status == "ACTIVE" {
            return Err(ServiceError::Invalid(
                "a lookup authorization is already active".into(),
            ));
        }
        let authorization_id = format!("lookup-auth-{}", uuid::Uuid::now_v7());
        let expiry_time = epoch_secs()
            .saturating_add(LOOKUP_AUTHORIZATION_LIFETIME_SECS)
            .to_string();
        let event = EventEnvelope::seal(NewEvent {
            event_id: String::new(),
            aggregate_type: "lookup_authorization".into(),
            aggregate_id: authorization_id.clone(),
            aggregate_revision: 1,
            actor_member_id: req.actor_member_id,
            device_id: self.device_id.clone(),
            occurred_at: Self::now(),
            app_version: env!("CARGO_PKG_VERSION").into(),
            previous_event_hash: None,
            payload: EventPayload::LookupAuthorizationGranted {
                authorization_id: authorization_id.clone(),
                application_id: req.application_id.clone(),
                provider_id: provider_id.clone(),
                expiry_time: expiry_time.clone(),
            },
        })?;
        self.append_and_project(&event)?;
        Ok(LookupAuthorizationStatusDto {
            provider_id,
            status: "ACTIVE".into(),
            authorization_id: Some(authorization_id),
            expiry_time: Some(expiry_time),
        })
    }

    pub fn security_status(&self) -> Result<SecurityStatusDto> {
        let keyring_ready = match self.security_mode {
            RuntimeSecurityMode::DevelopmentSynthetic => false,
            RuntimeSecurityMode::ProductionSecure => self.identity_key().is_ok(),
        };
        Ok(SecurityStatusDto {
            mode: self.security_mode.as_str().into(),
            key_provider: match self.security_mode {
                RuntimeSecurityMode::DevelopmentSynthetic => "in-memory-dev".into(),
                RuntimeSecurityMode::ProductionSecure => "os-keyring".into(),
            },
            real_pan_allowed: keyring_ready,
            os_keyring_release_blocker: !keyring_ready,
            blocker: if keyring_ready {
                None
            } else if matches!(
                self.security_mode,
                RuntimeSecurityMode::DevelopmentSynthetic
            ) {
                Some("Development synthetic mode: real PAN is disabled".into())
            } else {
                Some("OS keyring unavailable, locked, or corrupted".into())
            },
        })
    }
}

fn identity_payload_to_json(identity_payload: &sanket_identity_security::IdentitySecret) -> String {
    serde_json::json!({
        "pan": identity_payload.pan.as_normalized(),
        "upi_id": identity_payload.upi_id,
    })
    .to_string()
}

fn role_from_str(s: &str) -> Role {
    match s.to_uppercase().as_str() {
        "CORE_MEMBER" => Role::CoreMember,
        _ => Role::Owner,
    }
}

fn ranked_to_response(r: &sanket_ranking::RankedIpo) -> RankedIpoResponse {
    RankedIpoResponse {
        typed_name: r.typed_name().to_owned(),
        ranking: r.ranking(),
        score: r.score(),
        recommended_account_count: r.recommended_account_count(),
        recommended_allocation_ratio_bp: r.recommended_allocation_ratio_bp(),
        skip: r.skip(),
        reason: r.reason().to_owned(),
        missing_public_data: r.missing_public_data().to_vec(),
    }
}

// --- request/response DTOs (serde) ---

#[derive(Debug, Deserialize)]
pub struct OnboardMemberRequest {
    pub member_id: String,
    pub display_name: String,
    pub email: String,
    pub role: String,
    pub primary_account_label: Option<String>,
    pub broker: Option<String>,
    pub upi_id: String,
    pub pan: String,
    pub consented: bool,
}

#[derive(Debug, Serialize)]
pub struct OnboardMemberResponse {
    pub member_id: String,
    pub masked_pan: String,
}

#[derive(Debug, Deserialize)]
pub struct AddFriendRequest {
    pub friend_id: String,
    pub owner_member_id: String,
    pub name: String,
    pub upi_id: String,
    pub pan: String,
    pub broker: Option<String>,
    pub share_eligible: bool,
    pub share_basis_points: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct AddFriendResponse {
    pub friend_id: String,
    pub masked_pan: String,
}

#[derive(Debug, Deserialize)]
pub struct CheckIpoInput {
    pub name: String,
    pub amount_paise: i64,
}

#[derive(Debug, Deserialize)]
pub struct CheckRequest {
    pub session_id: String,
    pub declared_capital_paise: i64,
    pub account_ids: Vec<String>,
    pub ipos: Vec<CheckIpoInput>,
}

#[derive(Debug, Serialize)]
pub struct RankedIpoResponse {
    pub typed_name: String,
    pub ranking: u32,
    pub score: i64,
    pub recommended_account_count: u32,
    pub recommended_allocation_ratio_bp: i64,
    pub skip: bool,
    pub reason: String,
    pub missing_public_data: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct CheckResponse {
    pub session_id: String,
    pub algorithm_version: String,
    pub label: String,
    pub explanation: String,
    pub ipos: Vec<RankedIpoResponse>,
}

#[derive(Debug, Deserialize)]
pub struct SubmitIpoInput {
    pub name: String,
    pub amount_paise: i64,
    pub account_ids: Vec<String>,
    pub registrar_id: String,
    pub expected_allotment_date: Option<String>,
    #[serde(default)]
    pub metadata_snapshot: Option<IpoMetadataSnapshot>,
    #[serde(default)]
    pub confirm_metadata_changes: bool,
}

#[derive(Debug, Deserialize)]
pub struct SubmitRequest {
    pub session_id: String,
    pub actor_member_id: String,
    pub declared_capital_paise: i64,
    pub recommendation_id: Option<String>,
    pub ipos: Vec<SubmitIpoInput>,
}

#[derive(Debug, Serialize)]
pub struct SubmitResponse {
    pub session_id: String,
    pub allocation_count: u32,
}

#[derive(Debug, Deserialize)]
pub struct HistoricalApplicationRequest {
    pub actor_member_id: String,
    pub account_id: String,
    pub ipo_name: String,
    pub amount_paise: i64,
    pub application_date: Option<String>,
    pub registrar_id: String,
    pub provider_issue_id: String,
    pub owner_affirmed: bool,
}

#[derive(Debug, Serialize)]
pub struct HistoricalApplicationResponse {
    pub session_id: String,
    pub application_id: String,
    pub allocation_id: String,
    pub provider_id: String,
    pub provider_issue_id: String,
    pub source: String,
}

#[derive(Debug, Deserialize)]
pub struct VoidSessionRequest {
    pub session_id: String,
    pub actor_member_id: String,
    pub reason: String,
    pub owner_affirmed: bool,
}

#[derive(Debug, Serialize)]
pub struct VoidSessionResponse {
    pub session_id: String,
    pub status: String,
    pub reason: String,
}

#[derive(Debug, Serialize)]
pub struct MemberRow {
    pub id: String,
    pub name: String,
    pub role: String,
    pub masked_pan: String,
}

#[derive(Debug, Serialize)]
pub struct FriendRow {
    pub id: String,
    pub owner_member_id: String,
    pub label: String,
    pub masked_pan: String,
    pub share_basis_points: i64,
}

#[derive(Debug, Serialize)]
pub struct Dashboard {
    pub total_planned_paise: i64,
    pub submitted_session_count: u32,
    pub member_count: u32,
    pub friend_count: u32,
    pub profit_paise: i64,
}

#[derive(Debug, Serialize)]
pub struct AllotmentCandidateRow {
    pub application_id: String,
    pub session_id: String,
    pub ipo_name: String,
    pub planned_amount_paise: i64,
    pub account_count: u32,
    pub registrar_id: String,
    pub registrar_name: String,
    pub provider_id: String,
    pub provider_name: String,
    pub official_status_url: Option<String>,
    pub provider_health: String,
    pub expected_allotment_date: Option<String>,
    pub pending_count: u32,
    pub final_count: u32,
    pub last_checked: Option<String>,
    pub overall_job_state: String,
}

#[derive(Debug, Deserialize)]
pub struct StartAllotmentRequest {
    pub application_id: String,
    pub session_id: String,
    pub ipo_name: String,
    pub actor_member_id: String,
    pub registrar_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct LookupAuthorizationRequest {
    pub application_id: String,
    pub actor_member_id: String,
    pub owner_affirmed: bool,
}

#[derive(Debug, Serialize)]
pub struct AllotmentReportRow {
    pub attempt_id: String,
    pub account_id: String,
    pub display_name: String,
    pub account_kind: String,
    pub masked_pan: String,
    pub status: String,
    pub allotted_lots: Option<u32>,
    pub allotted_shares: Option<u64>,
    pub provider_id: String,
    pub registrar_id: String,
    pub source: String,
    pub provenance: String,
    pub application_amount_paise: i64,
    pub checked_at: Option<String>,
    pub safe_provider_reference: Option<String>,
    pub safe_message: Option<String>,
    pub next_retry_at: Option<String>,
    pub human_verification_state: Option<String>,
    pub estimated_profit_paise: Option<i64>,
    pub profit_basis: String,
    pub profit_provenance: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct AllotmentJobReport {
    pub job_id: String,
    pub application_id: String,
    pub ipo_name: String,
    pub registrar_id: String,
    pub registrar_name: String,
    pub provider_id: String,
    pub status: String,
    pub official_status_url: Option<String>,
    pub checked_at: Option<String>,
    pub final_count: u32,
    pub pending_count: u32,
    pub accounts: Vec<AllotmentReportRow>,
}

#[derive(Debug, Deserialize)]
pub struct ManualAllotmentRequest {
    pub job_id: String,
    pub account_id: String,
    pub actor_member_id: String,
    pub allotted_lots: Option<u32>,
    pub allotted_shares: Option<u64>,
    pub explicit_not_allotted: bool,
    pub note: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct EstimateProfitRequest {
    pub application_id: String,
    pub account_id: String,
    pub actor_member_id: String,
    pub basis: String,
    pub allotted_shares: u64,
    pub reference_price_paise: Option<i64>,
    pub issue_price_paise: Option<i64>,
    pub source: String,
    pub as_of: String,
    pub note: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct EstimatedProfitDto {
    pub basis: String,
    pub reference_price_paise: Option<i64>,
    pub issue_price_paise: Option<i64>,
    pub allotted_shares: u64,
    pub estimated_profit_paise: Option<i64>,
    pub note: Option<String>,
    /// Always false for estimated path — realized profit is separate.
    pub is_realized: bool,
}

#[derive(Debug, Serialize)]
pub struct SecurityStatusDto {
    pub mode: String,
    pub key_provider: String,
    pub real_pan_allowed: bool,
    pub os_keyring_release_blocker: bool,
    pub blocker: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct LookupAuthorizationStatusDto {
    pub provider_id: String,
    pub status: String,
    pub authorization_id: Option<String>,
    pub expiry_time: Option<String>,
}

#[cfg(test)]
mod lookup_authorization_tests {
    use super::*;

    fn test_app(mode: RuntimeSecurityMode) -> (Application, PathBuf) {
        let root =
            std::env::temp_dir().join(format!("sanket-lookup-auth-{}", uuid::Uuid::now_v7()));
        std::fs::create_dir_all(&root).expect("test root");
        let app = Application::with_mode(
            "test-device".into(),
            root.join("vault"),
            root.join("index.sqlite"),
            mode,
        )
        .expect("application");
        (app, root)
    }

    fn event(id: &str, actor: &str, payload: EventPayload) -> EventEnvelope {
        EventEnvelope::seal(NewEvent {
            event_id: id.into(),
            aggregate_type: "lookup_authorization".into(),
            aggregate_id: "lookup-auth-1".into(),
            aggregate_revision: 1,
            actor_member_id: actor.into(),
            device_id: "test-device".into(),
            occurred_at: "1788210900".into(),
            app_version: "0.1.0".into(),
            previous_event_hash: None,
            payload,
        })
        .expect("event")
    }

    #[test]
    fn development_mode_denies_lookup_authorization() {
        let (app, root) = test_app(RuntimeSecurityMode::DevelopmentSynthetic);
        let error = app
            .authorize_real_investor_lookup(LookupAuthorizationRequest {
                application_id: "application-1".into(),
                actor_member_id: "owner-1".into(),
                owner_affirmed: true,
            })
            .expect_err("development mode must deny");
        assert!(error.to_string().contains("PRODUCTION_SECURE"));
        drop(app);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn production_without_keyring_denies_lookup_authorization() {
        let (app, root) = test_app(RuntimeSecurityMode::ProductionSecure);
        let security = app.security_status().expect("security status");
        assert!(!security.real_pan_allowed);
        let error = app
            .authorize_real_investor_lookup(LookupAuthorizationRequest {
                application_id: "application-1".into(),
                actor_member_id: "owner-1".into(),
                owner_affirmed: true,
            })
            .expect_err("missing keyring must deny");
        assert!(matches!(error, ServiceError::Invalid(_)));
        drop(app);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn production_lookup_requires_owner_confirmation() {
        let (app, root) = test_app(RuntimeSecurityMode::ProductionSecure);
        let error = app
            .authorize_real_investor_lookup(LookupAuthorizationRequest {
                application_id: "application-1".into(),
                actor_member_id: "owner-1".into(),
                owner_affirmed: false,
            })
            .expect_err("missing owner confirmation must deny");
        assert!(error.to_string().contains("confirmation"));
        drop(app);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn expired_lookup_authorization_is_denied_by_event_replay() {
        let (app, root) = test_app(RuntimeSecurityMode::DevelopmentSynthetic);
        app.vault
            .append_event(&event(
                "owner-1",
                "owner-1",
                EventPayload::MemberCreated {
                    member_id: "owner-1".into(),
                    display_name: "Owner".into(),
                    role: Role::Owner,
                },
            ))
            .expect("owner");
        app.vault
            .append_event(&event(
                "grant-1",
                "owner-1",
                EventPayload::LookupAuthorizationGranted {
                    authorization_id: "lookup-auth-1".into(),
                    application_id: "application-1".into(),
                    provider_id: "mufg-intime-live".into(),
                    expiry_time: "0".into(),
                },
            ))
            .expect("grant");
        let state = app
            .lookup_authorization_state("application-1", "mufg-intime-live")
            .expect("state");
        assert_eq!(state.status, "EXPIRED");
        assert!(
            app.require_lookup_permit("application-1", "mufg-intime-live")
                .is_err()
        );
        drop(app);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn valid_owner_authorization_yields_scoped_runtime_permit() {
        let (app, root) = test_app(RuntimeSecurityMode::DevelopmentSynthetic);
        app.vault
            .append_event(&event(
                "owner-1",
                "owner-1",
                EventPayload::MemberCreated {
                    member_id: "owner-1".into(),
                    display_name: "Owner".into(),
                    role: Role::Owner,
                },
            ))
            .expect("owner");
        app.vault
            .append_event(&event(
                "grant-1",
                "owner-1",
                EventPayload::LookupAuthorizationGranted {
                    authorization_id: "lookup-auth-1".into(),
                    application_id: "application-1".into(),
                    provider_id: "mufg-intime-live".into(),
                    expiry_time: (epoch_secs() + LOOKUP_AUTHORIZATION_LIFETIME_SECS).to_string(),
                },
            ))
            .expect("grant");
        let state = app
            .lookup_authorization_state("application-1", "mufg-intime-live")
            .expect("state");
        assert_eq!(state.status, "ACTIVE");
        let permit = sanket_allotment::RealInvestorLookupPermit::new(
            "lookup-auth-1",
            "application-1",
            "mufg-intime-live",
        )
        .expect("permit");
        assert!(permit.matches("application-1", "mufg-intime-live"));
        assert!(!permit.matches("other-application", "mufg-intime-live"));
        app.consume_lookup_authorization(&permit)
            .expect("consume authorization");
        let consumed = app
            .lookup_authorization_state("application-1", "mufg-intime-live")
            .expect("consumed state");
        assert_eq!(consumed.status, "NOT_GRANTED");
        drop(app);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn final_permit_check_precedes_pan_access() {
        let source = include_str!("service.rs");
        let permit_check = source
            .find("self.validate_final_lookup_permit(")
            .expect("final permit check");
        let pan_access = source.find(".with_pan(").expect("PAN boundary");
        assert!(permit_check < pan_access);
    }
}
