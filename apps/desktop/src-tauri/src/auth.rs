use std::collections::{BTreeMap, HashMap};
use std::fmt;
use std::sync::{Mutex, OnceLock};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;
use uuid::Uuid;
use zeroize::Zeroize;

use sanket_domain::{EventEnvelope, EventPayload, NewEvent, Role};
use sanket_identity_security::{
    PASSWORD_VERIFIER_VERSION, Password, PasswordVerification, PasswordVerifier, hash_password,
    verify_password,
};
use sanket_member_vault::MemberVault;

pub const MAX_FAILED_ATTEMPT_BACKOFF_SECS: u64 = 30;
const SESSION_LIFETIME_SECS: u64 = 3_600;

#[derive(Debug, Error)]
pub enum AuthError {
    #[error("authentication vault error: {0}")]
    Vault(#[from] sanket_member_vault::MemberVaultError),
    #[error("authentication event error: {0}")]
    Event(#[from] sanket_domain::EventError),
    #[error("password credential operation failed")]
    Credential(#[from] sanket_identity_security::PasswordCredentialError),
    #[error("authentication operation rejected")]
    Rejected,
    #[error("authentication operation is invalid: {0}")]
    Invalid(&'static str),
}

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OwnerBootstrapRequest {
    pub account_id: String,
    pub email: String,
    pub password: String,
}

impl fmt::Debug for OwnerBootstrapRequest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("OwnerBootstrapRequest")
            .field("account_id", &self.account_id)
            .field("email", &self.email)
            .field("password", &"[REDACTED]")
            .finish()
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InviteIssueRequest {
    pub actor_account_id: String,
    pub email: String,
    pub expires_at: u64,
}

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SignupRequest {
    pub invite_id: String,
    pub invite_secret: String,
    pub account_id: String,
    pub password: String,
}

impl fmt::Debug for SignupRequest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SignupRequest")
            .field("invite_id", &self.invite_id)
            .field("invite_secret", &"[REDACTED]")
            .field("account_id", &self.account_id)
            .field("password", &"[REDACTED]")
            .finish()
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountDecisionRequest {
    pub actor_account_id: String,
    pub account_id: String,
}

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoginRequest {
    pub login: String,
    pub password: String,
}

impl fmt::Debug for LoginRequest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("LoginRequest")
            .field("login", &self.login)
            .field("password", &"[REDACTED]")
            .finish()
    }
}

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LogoutRequest {
    pub session_token: String,
}

impl fmt::Debug for LogoutRequest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("LogoutRequest")
            .field("session_token", &"[REDACTED]")
            .finish()
    }
}

#[derive(Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IssuedInvite {
    pub invite_id: String,
    pub account_id: String,
    pub invite_secret: String,
    pub invite_digest: String,
    pub expires_at: u64,
}

impl fmt::Debug for IssuedInvite {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("IssuedInvite")
            .field("invite_id", &self.invite_id)
            .field("account_id", &self.account_id)
            .field("invite_secret", &"[REDACTED]")
            .field("invite_digest", &self.invite_digest)
            .field("expires_at", &self.expires_at)
            .finish()
    }
}

#[derive(Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LoginResponse {
    pub accepted: bool,
    pub account_id: Option<String>,
    pub role: Option<Role>,
    #[serde(skip_serializing)]
    pub session_token: Option<String>,
    pub retry_after_secs: u64,
}

impl fmt::Debug for LoginResponse {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("LoginResponse")
            .field("accepted", &self.accepted)
            .field("account_id", &self.account_id)
            .field("role", &self.role)
            .field("session_token", &"[REDACTED]")
            .field("retry_after_secs", &self.retry_after_secs)
            .finish()
    }
}

impl LoginResponse {
    /// Deliberately omits account/session data so rejected logins have one public outcome.
    pub fn public_outcome(&self) -> bool {
        self.accepted
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AccountStatus {
    Pending,
    Active,
    Revoked,
}

#[derive(Clone, Debug)]
struct AccountState {
    email: String,
    verifier: String,
    verifier_version: u16,
    role: Role,
    status: AccountStatus,
}

#[derive(Clone, Debug)]
struct InviteState {
    account_id: String,
    email: String,
    digest: String,
    expires_at: u64,
    used: bool,
}

#[derive(Clone, Debug)]
struct ReplayedSession {
    account_id: String,
    expires_at: u64,
}

/// Reconstructed authentication state. The vault event log is the only source of truth.
#[derive(Clone, Debug, Default)]
pub struct AuthState {
    accounts: BTreeMap<String, AccountState>,
    invites: BTreeMap<String, InviteState>,
    sessions: BTreeMap<String, ReplayedSession>,
}

impl AuthState {
    pub fn owner_count(&self) -> usize {
        self.accounts
            .values()
            .filter(|account| account.role == Role::Owner)
            .count()
    }

    pub fn account_status(&self, account_id: &str) -> Option<AccountStatus> {
        self.accounts.get(account_id).map(|account| account.status)
    }

    pub fn account_role(&self, account_id: &str) -> Option<Role> {
        self.accounts.get(account_id).map(|account| account.role)
    }

    pub fn active_session_count(&self) -> usize {
        self.sessions.len()
    }

    pub fn session_is_valid(&self, session_hash: &str, now: u64) -> bool {
        self.sessions.get(session_hash).is_some_and(|session| {
            now < session.expires_at
                && self
                    .accounts
                    .get(&session.account_id)
                    .is_some_and(|account| account.status == AccountStatus::Active)
        })
    }

    fn apply(&mut self, event: &EventEnvelope) {
        match event.payload() {
            EventPayload::OwnerBootstrapped {
                account_id,
                email,
                verifier_version,
                verifier,
            } => {
                self.accounts.insert(
                    account_id.clone(),
                    AccountState {
                        email: email.clone(),
                        verifier: verifier.clone(),
                        verifier_version: *verifier_version,
                        role: Role::Owner,
                        status: AccountStatus::Active,
                    },
                );
            }
            EventPayload::InviteIssued {
                invite_id,
                account_id,
                email,
                invite_digest,
                expires_at,
            } => {
                self.invites.insert(
                    invite_id.clone(),
                    InviteState {
                        account_id: account_id.clone(),
                        email: email.clone(),
                        digest: invite_digest.clone(),
                        expires_at: expires_at.parse().unwrap_or(0),
                        used: false,
                    },
                );
            }
            EventPayload::SignupPending {
                account_id,
                invite_id,
                email,
                verifier_version,
                verifier,
            } => {
                if let Some(invite) = self.invites.get_mut(invite_id) {
                    invite.used = true;
                }
                self.accounts.insert(
                    account_id.clone(),
                    AccountState {
                        email: email.clone(),
                        verifier: verifier.clone(),
                        verifier_version: *verifier_version,
                        role: Role::CoreMember,
                        status: AccountStatus::Pending,
                    },
                );
            }
            EventPayload::AccountApproved { account_id, .. } => {
                if let Some(account) = self.accounts.get_mut(account_id) {
                    if account.status == AccountStatus::Pending {
                        account.status = AccountStatus::Active;
                    }
                }
            }
            EventPayload::AccountRevoked { account_id, .. } => {
                if let Some(account) = self.accounts.get_mut(account_id) {
                    account.status = AccountStatus::Revoked;
                }
            }
            EventPayload::SessionStarted {
                session_hash,
                account_id,
                expires_at,
            } => {
                self.sessions.insert(
                    session_hash.clone(),
                    ReplayedSession {
                        account_id: account_id.clone(),
                        expires_at: expires_at.parse().unwrap_or(0),
                    },
                );
            }
            EventPayload::SessionLoggedOut { session_hash, .. } => {
                self.sessions.remove(session_hash);
            }
            _ => {}
        }
    }
}

#[derive(Clone, Debug)]
struct LiveSession {
    account_id: String,
    role: Role,
    expires_at: u64,
}

#[derive(Default)]
struct RuntimeState {
    sessions: HashMap<String, LiveSession>,
    failures: HashMap<String, FailedAttempt>,
}

#[derive(Clone, Copy, Debug)]
struct FailedAttempt {
    count: u32,
    next_allowed_at: u64,
}

/// Native auth service. Events are persisted to MemberVault; sessions and
/// failed-attempt state intentionally remain in memory only.
pub struct AuthService {
    vault: MemberVault,
    device_id: String,
    mutation_lock: Mutex<()>,
    runtime: Mutex<RuntimeState>,
}

impl AuthService {
    pub fn new(vault: MemberVault, device_id: impl Into<String>) -> Self {
        Self {
            vault,
            device_id: device_id.into(),
            mutation_lock: Mutex::new(()),
            runtime: Mutex::new(RuntimeState::default()),
        }
    }

    pub fn vault(&self) -> &MemberVault {
        &self.vault
    }

    pub fn reconstruct_state(&self) -> Result<AuthState, AuthError> {
        let mut events = self.vault.list_events()?;
        events.sort_by(|left, right| {
            left.occurred_at()
                .cmp(right.occurred_at())
                .then_with(|| left.event_id().cmp(right.event_id()))
        });
        let mut state = AuthState::default();
        for event in &events {
            state.apply(event);
        }
        Ok(state)
    }

    pub fn bootstrap_owner_at(
        &self,
        request: OwnerBootstrapRequest,
        now: u64,
    ) -> Result<(), AuthError> {
        let _guard = self.mutation_lock.lock().map_err(|_| AuthError::Rejected)?;
        if !self.vault.is_uninitialized()? {
            return Err(AuthError::Rejected);
        }
        if request.account_id.trim().is_empty() || request.email.trim().is_empty() {
            return Err(AuthError::Invalid("owner identity is required"));
        }
        if request.password.is_empty() {
            return Err(AuthError::Invalid("owner password is required"));
        }
        let verifier = hash_password(&owned_password(request.password))?;
        let account_id = request.account_id;
        self.append_event(
            account_id.clone(),
            account_id.clone(),
            now,
            EventPayload::OwnerBootstrapped {
                account_id,
                email: request.email,
                verifier_version: PASSWORD_VERIFIER_VERSION,
                verifier: verifier.as_str().to_owned(),
            },
        )
    }

    pub fn issue_invite_at(
        &self,
        request: InviteIssueRequest,
        now: u64,
    ) -> Result<IssuedInvite, AuthError> {
        let _guard = self.mutation_lock.lock().map_err(|_| AuthError::Rejected)?;
        let state = self.reconstruct_state()?;
        let Some(actor) = state.accounts.get(&request.actor_account_id) else {
            return Err(AuthError::Rejected);
        };
        if actor.status != AccountStatus::Active || !actor.role.can_manage_members() {
            return Err(AuthError::Rejected);
        }
        if request.email.trim().is_empty() || request.expires_at <= now {
            return Err(AuthError::Invalid("invite identity or expiry is invalid"));
        }
        let invite_id = format!("invite-{}", Uuid::now_v7());
        let account_id = format!("account-{}", Uuid::now_v7());
        let invite_secret = format!("{}{}", Uuid::now_v7().simple(), Uuid::now_v7().simple());
        let invite_digest = digest(&invite_secret);
        self.append_event(
            invite_id.clone(),
            request.actor_account_id,
            now,
            EventPayload::InviteIssued {
                invite_id: invite_id.clone(),
                account_id: account_id.clone(),
                email: request.email,
                invite_digest: invite_digest.clone(),
                expires_at: request.expires_at.to_string(),
            },
        )?;
        Ok(IssuedInvite {
            invite_id,
            account_id,
            invite_secret,
            invite_digest,
            expires_at: request.expires_at,
        })
    }

    pub fn complete_signup_at(&self, request: SignupRequest, now: u64) -> Result<(), AuthError> {
        let _guard = self.mutation_lock.lock().map_err(|_| AuthError::Rejected)?;
        let state = self.reconstruct_state()?;
        let Some(invite) = state.invites.get(&request.invite_id) else {
            return Err(AuthError::Rejected);
        };
        if invite.used
            || now >= invite.expires_at
            || digest(&request.invite_secret) != invite.digest
        {
            return Err(AuthError::Rejected);
        }
        if request.account_id.trim().is_empty() {
            return Err(AuthError::Invalid("account identity is required"));
        }
        if request.password.is_empty() || invite.account_id != request.account_id {
            return Err(AuthError::Rejected);
        }
        let verifier = hash_password(&owned_password(request.password))?;
        let account_id = request.account_id;
        self.append_event(
            account_id.clone(),
            account_id.clone(),
            now,
            EventPayload::SignupPending {
                account_id,
                invite_id: request.invite_id,
                email: invite.email.clone(),
                verifier_version: PASSWORD_VERIFIER_VERSION,
                verifier: verifier.as_str().to_owned(),
            },
        )
    }

    pub fn approve_account_at(
        &self,
        request: AccountDecisionRequest,
        now: u64,
    ) -> Result<(), AuthError> {
        self.account_decision(request, now, true)
    }

    pub fn revoke_account_at(
        &self,
        request: AccountDecisionRequest,
        now: u64,
    ) -> Result<(), AuthError> {
        self.account_decision(request, now, false)
    }

    fn account_decision(
        &self,
        request: AccountDecisionRequest,
        now: u64,
        approve: bool,
    ) -> Result<(), AuthError> {
        let _guard = self.mutation_lock.lock().map_err(|_| AuthError::Rejected)?;
        let state = self.reconstruct_state()?;
        let actor_id = request.actor_account_id.clone();
        let target_id = request.account_id.clone();
        let Some(actor) = state.accounts.get(&actor_id) else {
            return Err(AuthError::Rejected);
        };
        let Some(account) = state.accounts.get(&target_id) else {
            return Err(AuthError::Rejected);
        };
        if actor.status != AccountStatus::Active
            || !actor.role.can_manage_members()
            || (approve && account.status != AccountStatus::Pending)
        {
            return Err(AuthError::Rejected);
        }
        self.append_event(
            target_id.clone(),
            actor_id.clone(),
            now,
            if approve {
                EventPayload::AccountApproved {
                    account_id: target_id,
                    approved_by: actor_id,
                }
            } else {
                EventPayload::AccountRevoked {
                    account_id: target_id,
                    revoked_by: actor_id,
                }
            },
        )
    }

    pub fn login_at(&self, request: LoginRequest, now: u64) -> LoginResponse {
        let key = request.login.trim().to_ascii_lowercase();
        let mut runtime = self.runtime.lock().expect("auth runtime lock poisoned");
        if let Some(failure) = runtime.failures.get(&key) {
            if now < failure.next_allowed_at {
                return LoginResponse {
                    accepted: false,
                    account_id: None,
                    role: None,
                    session_token: None,
                    retry_after_secs: failure.next_allowed_at - now,
                };
            }
        }
        let state = self.reconstruct_state().unwrap_or_default();
        let account = state.accounts.iter().find(|(account_id, account)| {
            account_id.eq_ignore_ascii_case(&request.login)
                || account.email.eq_ignore_ascii_case(request.login.trim())
        });
        let password = owned_password(request.password);
        let verified = account
            .map(|(_, account)| verify_password(&password, &account.verifier))
            .unwrap_or_else(|| verify_password(&password, dummy_verifier().as_str()));
        let Some((account_id, account)) = account else {
            return rejected_login(&mut runtime, key, now);
        };
        let account_id = account_id.clone();
        let role = account.role;
        if verified != PasswordVerification::Verified
            || account.verifier_version != PASSWORD_VERIFIER_VERSION
            || account.status != AccountStatus::Active
        {
            return rejected_login(&mut runtime, key, now);
        }
        runtime.failures.remove(&key);
        let session_token = format!("{}{}", Uuid::now_v7().simple(), Uuid::now_v7().simple());
        let expires_at = now.saturating_add(SESSION_LIFETIME_SECS);
        runtime.sessions.insert(
            session_token.clone(),
            LiveSession {
                account_id: account_id.clone(),
                role,
                expires_at,
            },
        );
        drop(runtime);
        if self
            .append_event(
                session_hash(&session_token),
                account_id.clone(),
                now,
                EventPayload::SessionStarted {
                    session_hash: session_hash(&session_token),
                    account_id: account_id.clone(),
                    expires_at: expires_at.to_string(),
                },
            )
            .is_err()
        {
            let mut runtime = self.runtime.lock().expect("auth runtime lock poisoned");
            runtime.sessions.remove(&session_token);
            return LoginResponse {
                accepted: false,
                account_id: None,
                role: None,
                session_token: None,
                retry_after_secs: 0,
            };
        }
        LoginResponse {
            accepted: true,
            account_id: Some(account_id),
            role: Some(role),
            session_token: Some(session_token),
            retry_after_secs: 0,
        }
    }

    pub fn validate_session(&self, token: &str, now: u64) -> Option<(String, Role)> {
        let mut runtime = self.runtime.lock().ok()?;
        let session = runtime.sessions.get(token)?.clone();
        if now >= session.expires_at {
            runtime.sessions.remove(token);
            return None;
        }
        let state = self.reconstruct_state().ok()?;
        if state.account_status(&session.account_id) != Some(AccountStatus::Active) {
            runtime.sessions.remove(token);
            return None;
        }
        Some((session.account_id, session.role))
    }

    pub fn logout_at(&self, token: &str, now: u64) -> Result<(), AuthError> {
        let session = {
            let mut runtime = self.runtime.lock().map_err(|_| AuthError::Rejected)?;
            runtime.sessions.remove(token)
        };
        let Some(session) = session else {
            return Err(AuthError::Rejected);
        };
        self.append_event(
            session_hash(token),
            session.account_id.clone(),
            now,
            EventPayload::SessionLoggedOut {
                session_hash: session_hash(token),
                account_id: session.account_id,
            },
        )
    }

    fn append_event(
        &self,
        aggregate_id: String,
        actor_account_id: String,
        now: u64,
        payload: EventPayload,
    ) -> Result<(), AuthError> {
        let event = EventEnvelope::seal(NewEvent {
            event_id: String::new(),
            aggregate_type: "auth".to_owned(),
            aggregate_id,
            aggregate_revision: 1,
            actor_member_id: actor_account_id,
            device_id: self.device_id.clone(),
            occurred_at: now.to_string(),
            app_version: env!("CARGO_PKG_VERSION").to_owned(),
            previous_event_hash: None,
            payload,
        })?;
        self.vault.append_event(&event)?;
        Ok(())
    }
}

fn owned_password(mut value: String) -> Password {
    let password = Password::new(&value);
    value.zeroize();
    password
}

fn rejected_login(runtime: &mut RuntimeState, key: String, now: u64) -> LoginResponse {
    let failure = runtime.failures.entry(key).or_insert(FailedAttempt {
        count: 0,
        next_allowed_at: now,
    });
    failure.count = failure.count.saturating_add(1);
    let delay = 1u64
        .checked_shl(failure.count.saturating_sub(1).min(6))
        .unwrap_or(MAX_FAILED_ATTEMPT_BACKOFF_SECS)
        .min(MAX_FAILED_ATTEMPT_BACKOFF_SECS);
    failure.next_allowed_at = now.saturating_add(delay);
    LoginResponse {
        accepted: false,
        account_id: None,
        role: None,
        session_token: None,
        retry_after_secs: delay,
    }
}

fn digest(value: &str) -> String {
    format!("sha256:{:x}", Sha256::digest(value.as_bytes()))
}

fn session_hash(value: &str) -> String {
    digest(value)
}

fn dummy_verifier() -> &'static PasswordVerifier {
    static DUMMY: OnceLock<PasswordVerifier> = OnceLock::new();
    DUMMY.get_or_init(|| hash_password(&Password::new("dummy-password")).expect("argon2 policy"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use sanket_domain::{EventPayload, Role};
    use sanket_member_vault::MemberVault;
    use serde_json::to_string;

    fn vault(name: &str) -> MemberVault {
        let root = std::env::temp_dir().join(format!(
            "sanket-native-auth-{name}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let _ = std::fs::remove_dir_all(&root);
        MemberVault::open(root).unwrap()
    }

    fn owner_request(password: &str) -> OwnerBootstrapRequest {
        OwnerBootstrapRequest {
            account_id: "owner-1".into(),
            email: "owner@example.invalid".into(),
            password: password.into(),
        }
    }

    #[test]
    fn owner_bootstrap_reconstructs_from_vault_and_never_uses_projection_counts() {
        let vault = vault("owner");
        let auth = AuthService::new(vault, "device-1");
        auth.bootstrap_owner_at(owner_request("owner-password"), 100)
            .unwrap();

        let state = auth.reconstruct_state().unwrap();
        assert_eq!(state.owner_count(), 1);
        assert_eq!(state.account_status("owner-1"), Some(AccountStatus::Active));
        assert_eq!(state.account_role("owner-1"), Some(Role::Owner));
        assert!(
            auth.bootstrap_owner_at(owner_request("second-password"), 101)
                .is_err()
        );
    }

    #[test]
    fn invite_is_private_one_time_expiring_and_persists_only_a_digest() {
        let vault = vault("invite");
        let auth = AuthService::new(vault, "device-1");
        auth.bootstrap_owner_at(owner_request("owner-password"), 100)
            .unwrap();
        let issued = auth
            .issue_invite_at(
                InviteIssueRequest {
                    actor_account_id: "owner-1".into(),
                    email: "member@example.invalid".into(),
                    expires_at: 200,
                },
                100,
            )
            .unwrap();

        let event_json = to_string(&auth.vault().list_events().unwrap()).unwrap();
        assert!(event_json.contains(&issued.invite_digest));
        assert!(!event_json.contains(&issued.invite_secret));

        auth.complete_signup_at(
            SignupRequest {
                invite_id: issued.invite_id.clone(),
                invite_secret: issued.invite_secret.clone(),
                account_id: issued.account_id.clone(),
                password: "member-password".into(),
            },
            150,
        )
        .unwrap();
        assert_eq!(
            auth.reconstruct_state()
                .unwrap()
                .account_status(&issued.account_id),
            Some(AccountStatus::Pending)
        );
        assert!(
            auth.complete_signup_at(
                SignupRequest {
                    invite_id: issued.invite_id,
                    invite_secret: issued.invite_secret,
                    account_id: "member-2".into(),
                    password: "member-password".into(),
                },
                151,
            )
            .is_err()
        );
    }

    #[test]
    fn expired_invite_is_rejected_without_persisting_the_secret() {
        let vault = vault("expired-invite");
        let auth = AuthService::new(vault, "device-1");
        auth.bootstrap_owner_at(owner_request("owner-password"), 100)
            .unwrap();
        let issued = auth
            .issue_invite_at(
                InviteIssueRequest {
                    actor_account_id: "owner-1".into(),
                    email: "member@example.invalid".into(),
                    expires_at: 101,
                },
                100,
            )
            .unwrap();
        assert!(
            auth.complete_signup_at(
                SignupRequest {
                    invite_id: issued.invite_id,
                    invite_secret: issued.invite_secret,
                    account_id: issued.account_id,
                    password: "member-password".into(),
                },
                102,
            )
            .is_err()
        );
    }

    #[test]
    fn approval_and_revocation_reconstruct_account_state_from_events() {
        let vault = vault("approval");
        let auth = AuthService::new(vault, "device-1");
        auth.bootstrap_owner_at(owner_request("owner-password"), 100)
            .unwrap();
        let issued = auth
            .issue_invite_at(
                InviteIssueRequest {
                    actor_account_id: "owner-1".into(),
                    email: "member@example.invalid".into(),
                    expires_at: 300,
                },
                100,
            )
            .unwrap();
        let member_account_id = issued.account_id.clone();
        auth.complete_signup_at(
            SignupRequest {
                invite_id: issued.invite_id,
                invite_secret: issued.invite_secret,
                account_id: member_account_id.clone(),
                password: "member-password".into(),
            },
            150,
        )
        .unwrap();
        auth.approve_account_at(
            AccountDecisionRequest {
                actor_account_id: "owner-1".into(),
                account_id: member_account_id.clone(),
            },
            160,
        )
        .unwrap();
        assert_eq!(
            auth.reconstruct_state()
                .unwrap()
                .account_status(&member_account_id),
            Some(AccountStatus::Active)
        );
        auth.revoke_account_at(
            AccountDecisionRequest {
                actor_account_id: "owner-1".into(),
                account_id: member_account_id.clone(),
            },
            170,
        )
        .unwrap();
        assert_eq!(
            auth.reconstruct_state()
                .unwrap()
                .account_status(&member_account_id),
            Some(AccountStatus::Revoked)
        );
    }

    #[test]
    fn login_is_generic_for_unknown_wrong_pending_and_revoked_accounts() {
        let vault = vault("login-generic");
        let auth = AuthService::new(vault, "device-1");
        auth.bootstrap_owner_at(owner_request("owner-password"), 100)
            .unwrap();
        let wrong = auth.login_at(
            LoginRequest {
                login: "owner@example.invalid".into(),
                password: "wrong-password".into(),
            },
            200,
        );
        let unknown = auth.login_at(
            LoginRequest {
                login: "unknown@example.invalid".into(),
                password: "wrong-password".into(),
            },
            200,
        );
        assert_eq!(wrong.public_outcome(), unknown.public_outcome());
        assert!(!wrong.accepted);
        assert!(wrong.session_token.is_none());
    }

    #[test]
    fn failed_login_backoff_is_bounded_and_session_validation_is_server_side() {
        let vault = vault("session");
        let auth = AuthService::new(vault, "device-1");
        auth.bootstrap_owner_at(owner_request("owner-password"), 100)
            .unwrap();
        let first_failure = auth.login_at(
            LoginRequest {
                login: "owner@example.invalid".into(),
                password: "wrong-password".into(),
            },
            200,
        );
        assert!(first_failure.retry_after_secs > 0);
        assert!(first_failure.retry_after_secs <= MAX_FAILED_ATTEMPT_BACKOFF_SECS);

        let accepted = auth.login_at(
            LoginRequest {
                login: "owner@example.invalid".into(),
                password: "owner-password".into(),
            },
            201,
        );
        assert!(accepted.accepted);
        let session_value = accepted.session_token.clone().unwrap();
        assert!(auth.validate_session(&session_value, 201).is_some());
        let replayed = auth.reconstruct_state().unwrap();
        assert!(replayed.session_is_valid(&session_hash(&session_value), 201));
        assert!(!replayed.session_is_valid(&session_hash(&session_value), 4_000));
        auth.logout_at(&session_value, 202).unwrap();
        assert!(auth.validate_session(&session_value, 203).is_none());

        let session_event_json = to_string(&auth.vault().list_events().unwrap()).unwrap();
        assert!(!session_event_json.contains(&session_value));
        assert!(!session_event_json.contains("owner-password"));
    }

    #[test]
    fn auth_dtos_do_not_debug_or_serialize_transient_secrets() {
        let owner = owner_request("owner-password");
        let signup = SignupRequest {
            invite_id: "invite-1".into(),
            invite_secret: "invite-secret".into(),
            account_id: "member-1".into(),
            password: "member-password".into(),
        };
        let login = LoginRequest {
            login: "owner@example.invalid".into(),
            password: "owner-password".into(),
        };
        let invite = IssuedInvite {
            invite_id: "invite-1".into(),
            account_id: "member-1".into(),
            invite_secret: "invite-secret".into(),
            invite_digest: "sha256:digest".into(),
            expires_at: 200,
        };
        for debug in [
            format!("{owner:?}"),
            format!("{signup:?}"),
            format!("{login:?}"),
            format!("{invite:?}"),
        ] {
            assert!(!debug.contains("owner-password"));
            assert!(!debug.contains("member-password"));
            assert!(!debug.contains("invite-secret"));
        }

        let response = LoginResponse {
            accepted: true,
            account_id: Some("owner-1".into()),
            role: Some(Role::Owner),
            session_token: Some("session-token".into()),
            retry_after_secs: 0,
        };
        let wire = to_string(&response).unwrap();
        assert!(!wire.contains("session-token"));
        assert!(!wire.contains("sessionToken"));
    }

    #[test]
    fn auth_events_are_metadata_only() {
        let payloads = [
            EventPayload::SignupPending {
                account_id: "member-1".into(),
                invite_id: "invite-1".into(),
                email: "member@example.invalid".into(),
                verifier_version: 1,
                verifier: "argon2id-verifier".into(),
            },
            EventPayload::AccountApproved {
                account_id: "member-1".into(),
                approved_by: "owner-1".into(),
            },
            EventPayload::AccountRevoked {
                account_id: "member-1".into(),
                revoked_by: "owner-1".into(),
            },
            EventPayload::SessionStarted {
                session_hash: "sha256:session".into(),
                account_id: "owner-1".into(),
                expires_at: "300".into(),
            },
            EventPayload::SessionLoggedOut {
                session_hash: "sha256:session".into(),
                account_id: "owner-1".into(),
            },
        ];
        let json = to_string(&payloads).unwrap();
        assert!(!json.contains("plain-password"));
        assert!(!json.contains("invite-secret"));
        assert!(!json.contains("refresh-secret"));
        assert!(!json.contains("4111111111111111"));
        assert!(!json.contains("session-token"));
    }
}
