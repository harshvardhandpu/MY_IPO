//! Application service: wires the domain, vault, index, and ranking together
//! behind narrowly-scoped operations. Sensitive input (PAN/UPI) enters here
//! only through explicit command arguments and is encrypted immediately.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use sanket_domain::{
    CoreMember, EventEnvelope, EventPayload, FriendAccount, InvestmentSession, Money, NewEvent,
    Role,
};
use sanket_identity_security::{IdentityKey, Pan};
use sanket_intelligence_vault::{InvestmentDecisionRequest, PlannedIpo};
use sanket_local_index::LocalIndex;
use sanket_member_vault::MemberVault;
use sanket_ranking::{DevRankingAlgorithm, RankingAlgorithm};

pub const DEV_KEY_ID: &str = "dev-key-1";

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
    #[error("{0}")]
    Invalid(String),
}

pub type Result<T> = std::result::Result<T, ServiceError>;

/// Shared application state: vault, projection index, and the dev key provider.
pub struct Application {
    device_id: String,
    vault: MemberVault,
    index: LocalIndex,
}

/// A stable per-device development key derived from the device id (NOT a
/// hardcoded literal key, and not for production use).
fn dev_key(device_id: &str) -> IdentityKey {
    let digest = Sha256::digest(device_id.as_bytes());
    let mut bytes = [0u8; 32];
    bytes.copy_from_slice(&digest);
    IdentityKey::from_bytes(&bytes)
}

impl Application {
    pub fn new(device_id: String, vault_root: PathBuf, index_path: PathBuf) -> Result<Self> {
        let vault = MemberVault::open(vault_root)?;
        let index = LocalIndex::open(&index_path)?;
        Ok(Self {
            device_id,
            vault,
            index,
        })
    }

    fn cipher(&self) -> sanket_identity_security::IdentityCipher {
        sanket_identity_security::IdentityCipher::new(dev_key(&self.device_id))
    }

    fn now() -> String {
        // RFC3339 via the same std-only approach as identity-security.
        let secs = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        format!("{:?}", secs)
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
        let envelope = self.cipher().encrypt(
            identity_payload_to_json(&identity_payload).as_bytes(),
            DEV_KEY_ID,
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
        let envelope = self.cipher().encrypt(
            identity_payload_to_json(&identity_payload).as_bytes(),
            DEV_KEY_ID,
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
        let total_invested: i64 = sessions.iter().map(|(_, _, paise, _)| *paise).sum();
        let member_count = self.index.list_members()?.len() as u32;
        let friend_count = self.index.list_active_friends()?.len() as u32;
        let submitted = sessions
            .iter()
            .filter(|(_, _, _, s)| s == "SUBMITTED")
            .count();
        Ok(Dashboard {
            total_planned_paise: total_invested,
            submitted_session_count: submitted as u32,
            member_count,
            friend_count,
            profit_paise: 0, // profit remains unavailable until real allotment records
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
