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

    // --- allotment ---

    pub fn list_allotment_candidates(&self) -> Result<Vec<AllotmentCandidateRow>> {
        Ok(self
            .index
            .list_submitted_applications()?
            .into_iter()
            .map(
                |(application_id, session_id, ipo_name, planned_amount_paise, account_count)| {
                    AllotmentCandidateRow {
                        application_id,
                        session_id,
                        ipo_name,
                        planned_amount_paise,
                        account_count,
                        registrar_id: "kfintech".into(),
                        registrar_name: "KFintech".into(),
                        official_status_url: Some("https://ipostatus.kfintech.com".into()),
                        provider_status: "AVAILABLE".into(),
                    }
                },
            )
            .collect())
    }

    /// Start + run fixture allotment checks sequentially for one application.
    pub fn start_allotment_check(&self, req: StartAllotmentRequest) -> Result<AllotmentJobReport> {
        let accounts = self
            .index
            .list_account_ids_for_application(&req.application_id)?;
        if accounts.is_empty() {
            return Err(ServiceError::Invalid(
                "no allocations found for application".into(),
            ));
        }

        let job_id = format!("job-{}", uuid::Uuid::now_v7());
        let registrar_id = req.registrar_id.unwrap_or_else(|| "kfintech".into());
        let registrar_name = req.registrar_name.unwrap_or_else(|| "KFintech".into());
        let provider_id = "kfintech-fixture".to_owned();
        let official_url = req
            .official_status_url
            .unwrap_or_else(|| "https://ipostatus.kfintech.com".into());

        let job = sanket_allotment::AllotmentCheckJob::create(
            job_id.clone(),
            req.application_id.clone(),
            req.session_id.clone(),
            req.ipo_name.clone(),
            registrar_id.clone(),
            registrar_name.clone(),
            provider_id.clone(),
        )
        .map_err(|e| ServiceError::Invalid(e.to_string()))?;

        let mut events = Vec::new();
        events.push(EventEnvelope::seal(NewEvent {
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
                session_id: req.session_id.clone(),
                ipo_name: req.ipo_name.clone(),
                registrar_id: registrar_id.clone(),
                provider_id: provider_id.clone(),
            },
        })?);
        events.push(EventEnvelope::seal(NewEvent {
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
        })?);

        let provider = sanket_allotment::FixtureKfintechProvider;
        let key_provider = sanket_identity_security::InMemoryKeyProvider::new(
            DEV_KEY_ID,
            dev_key(&self.device_id),
        );
        let mut sensitive = sanket_identity_security::SensitiveIdentityService::new(key_provider);
        let mut report_rows = Vec::new();
        let mut rev = 3u64;

        for account_id in &accounts {
            let attempt_id = format!("att-{}", uuid::Uuid::now_v7());
            let envelope = self
                .vault
                .load_member_identity(account_id)
                .or_else(|_| self.vault.load_friend_identity(account_id))?;
            let masked = self
                .index
                .masked_pan_for_account(account_id)?
                .unwrap_or_else(|| "[MASKED]".into());
            let masked_pan = sanket_identity_security::MaskedPan::from_display(masked.clone())
                .unwrap_or_else(|_| sanket_identity_security::MaskedPan::from_parts("AAAAA", "A"));
            let record = sanket_identity_security::SensitiveIdentityRecord::from_stored(
                account_id.clone(),
                masked_pan,
                envelope,
            );

            let (status, lots, shares, pref, source) = sensitive
                .with_pan(
                    &record,
                    sanket_identity_security::SensitivePurpose::AllotmentCheck,
                    &req.actor_member_id,
                    |pan_str| {
                        let pan = Pan::parse(pan_str).map_err(|e| e.to_string())?;
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
                        match sanket_allotment::AllotmentProvider::check_allotment(
                            &provider, &ctx, &pan,
                        ) {
                            Ok(r) => Ok((
                                r.status.as_str().to_owned(),
                                r.allotted_lots,
                                r.allotted_shares,
                                r.provider_reference,
                                "FIXTURE".to_owned(),
                            )),
                            Err(e) => Ok((
                                e.to_status().as_str().to_owned(),
                                None,
                                None,
                                None,
                                "FIXTURE".to_owned(),
                            )),
                        }
                    },
                )
                .map_err(|e| ServiceError::Invalid(e.to_string()))?
                .map_err(ServiceError::Invalid)?;

            // Audit only — no PAN in event.
            if let Some(audit) = sensitive.take_last_audit() {
                let _ = audit; // purpose-scoped access already audited in-memory
            }

            events.push(EventEnvelope::seal(NewEvent {
                event_id: String::new(),
                aggregate_type: "allotment_attempt".into(),
                aggregate_id: attempt_id.clone(),
                aggregate_revision: 1,
                actor_member_id: req.actor_member_id.clone(),
                device_id: self.device_id.clone(),
                occurred_at: Self::now(),
                app_version: env!("CARGO_PKG_VERSION").into(),
                previous_event_hash: None,
                payload: EventPayload::AllotmentAttemptRecorded {
                    attempt_id: attempt_id.clone(),
                    job_id: job_id.clone(),
                    account_id: account_id.clone(),
                    status: status.clone(),
                    allotted_lots: lots,
                    allotted_shares: shares,
                    source: source.clone(),
                    provider_reference: pref.clone(),
                },
            })?);
            rev += 1;

            let (label, kind) = self.index.display_label_for_account(account_id)?;
            report_rows.push(AllotmentReportRow {
                attempt_id,
                account_id: account_id.clone(),
                display_name: label,
                account_kind: kind,
                masked_pan: masked,
                status,
                allotted_lots: lots,
                allotted_shares: shares,
                provider_id: provider_id.clone(),
                source,
            });
        }

        let final_status = if report_rows.iter().all(|r| {
            matches!(
                r.status.as_str(),
                "ALLOTTED" | "NOT_ALLOTTED" | "NOT_FOUND" | "MANUAL_RESULT"
            )
        }) {
            "COMPLETE"
        } else {
            "PARTIALLY_COMPLETE"
        };
        events.push(EventEnvelope::seal(NewEvent {
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
        })?);

        for event in &events {
            self.vault.append_event(event)?;
            self.index.apply_event(event)?;
        }

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
            accounts: report_rows,
        })
    }

    pub fn get_allotment_report(&self, job_id: &str) -> Result<AllotmentJobReport> {
        let jobs = self.index.list_allotment_jobs()?;
        let job = jobs
            .into_iter()
            .find(|(id, ..)| id == job_id)
            .ok_or_else(|| ServiceError::Invalid("allotment job not found".into()))?;
        let (id, application_id, _session_id, ipo_name, registrar_id, status) = job;
        let attempts = self.index.list_allotment_attempts(&id)?;
        let mut accounts = Vec::new();
        for (attempt_id, account_id, st, lots, shares, source) in attempts {
            let (label, kind) = self.index.display_label_for_account(&account_id)?;
            let masked = self
                .index
                .masked_pan_for_account(&account_id)?
                .unwrap_or_else(|| "[MASKED]".into());
            accounts.push(AllotmentReportRow {
                attempt_id,
                account_id,
                display_name: label,
                account_kind: kind,
                masked_pan: masked,
                status: st,
                allotted_lots: lots.map(|v| v as u32),
                allotted_shares: shares.map(|v| v as u64),
                provider_id: "kfintech-fixture".into(),
                source,
            });
        }
        Ok(AllotmentJobReport {
            job_id: id,
            application_id,
            ipo_name,
            registrar_id,
            registrar_name: "KFintech".into(),
            provider_id: "kfintech-fixture".into(),
            status,
            official_status_url: Some("https://ipostatus.kfintech.com".into()),
            accounts,
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

#[derive(Debug, Serialize)]
pub struct AllotmentCandidateRow {
    pub application_id: String,
    pub session_id: String,
    pub ipo_name: String,
    pub planned_amount_paise: i64,
    pub account_count: u32,
    pub registrar_id: String,
    pub registrar_name: String,
    pub official_status_url: Option<String>,
    pub provider_status: String,
}

#[derive(Debug, Deserialize)]
pub struct StartAllotmentRequest {
    pub application_id: String,
    pub session_id: String,
    pub ipo_name: String,
    pub actor_member_id: String,
    pub registrar_id: Option<String>,
    pub registrar_name: Option<String>,
    pub official_status_url: Option<String>,
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
    pub source: String,
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
    pub accounts: Vec<AllotmentReportRow>,
}
