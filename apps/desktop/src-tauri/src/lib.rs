pub mod auth;
pub mod provider_credentials;
pub mod service;
pub mod upstox;
pub mod worker;

use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::Serialize;
use tauri::Manager;
use zeroize::{Zeroize, Zeroizing};

use auth::{
    AccountDecisionRequest, AuthService, InviteIssueRequest, IssuedInvite, LoginRequest,
    LoginResponse, OwnerBootstrapRequest, SignupRequest,
};
use provider_credentials::{
    ConnectUpstoxAnalyticsTokenRequest, OsProviderCredentialStore, ProviderConnectionStatusDto,
    SecretValue, disconnect, status, store_token,
};
use service::{
    AddFriendRequest, AddFriendResponse, AllotmentCandidateRow, AllotmentJobReport, Application,
    CheckRequest, CheckResponse, Dashboard, EstimateProfitRequest, EstimatedProfitDto, FriendRow,
    HistoricalApplicationRequest, HistoricalApplicationResponse, LookupAuthorizationRequest,
    LookupAuthorizationStatusDto, ManualAllotmentRequest, MemberRow, OnboardMemberRequest,
    OnboardMemberResponse, SecurityStatusDto, StartAllotmentRequest, SubmitRequest, SubmitResponse,
    VoidSessionRequest, VoidSessionResponse,
};
use upstox::{IpoCatalogItemDto, IpoCatalogueDto, IpoListQuery};

const AUTHENTICATION_ERROR: &str = "authentication failed";
const OWNER_ALREADY_EXISTS_ERROR: &str = "Owner account already exists. Sign in instead.";

fn security_status_label(mode: sanket_identity_security::RuntimeSecurityMode) -> &'static str {
    match mode {
        sanket_identity_security::RuntimeSecurityMode::ProductionSecure => {
            "OS KEYRING (PRODUCTION_SECURE)"
        }
        sanket_identity_security::RuntimeSecurityMode::DevelopmentSynthetic => {
            "DEV SYNTHETIC / IN-MEMORY — OS KEYRING REQUIRED BEFORE REAL PAN"
        }
    }
}

#[derive(Clone)]
pub struct AppState {
    app_version: &'static str,
    device_id: String,
    projection_schema_version: u32,
    vault_root: PathBuf,
    index_path: PathBuf,
    worker: Option<worker::AllotmentWorkerHandle>,
    provider_credentials: OsProviderCredentialStore,
    pub(crate) upstox: upstox::UpstoxService,
    auth_service: Option<Arc<AuthService>>,
    auth_initialization_error: Option<String>,
    active_session_token: Arc<Mutex<Option<String>>>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct AppStatus {
    pub app_version: &'static str,
    pub device_id: String,
    pub projection_schema_version: u32,
    pub sync_status: sanket_domain::SyncStatus,
    pub os_keyring_status: &'static str,
}

impl AppState {
    /// Compatibility constructor for status-only tests and callers.
    pub fn new(device_id: String, projection_schema_version: u32) -> Self {
        Self::build(
            device_id,
            projection_schema_version,
            PathBuf::new(),
            PathBuf::new(),
        )
    }

    pub fn build(
        device_id: String,
        projection_schema_version: u32,
        vault_root: PathBuf,
        index_path: PathBuf,
    ) -> Self {
        let (auth_service, auth_initialization_error) = if vault_root.as_os_str().is_empty() {
            (None, None)
        } else {
            match service::open_verified_member_vault(
                &device_id,
                vault_root.clone(),
                &index_path,
                service::resolve_security_mode(),
            ) {
                Ok(vault) => (Some(Arc::new(AuthService::new(vault, device_id.clone()))), None),
                Err(error) => (None, Some(format!("authentication unavailable: {error}"))),
            }
        };
        Self {
            app_version: env!("CARGO_PKG_VERSION"),
            device_id,
            projection_schema_version,
            vault_root,
            index_path,
            worker: None,
            provider_credentials: OsProviderCredentialStore,
            upstox: upstox::UpstoxService::new(PathBuf::new()),
            auth_service,
            auth_initialization_error,
            active_session_token: Arc::new(Mutex::new(None)),
        }
    }

    pub fn with_worker(mut self, worker: worker::AllotmentWorkerHandle) -> Self {
        self.worker = Some(worker);
        self
    }

    pub fn with_upstox(mut self, upstox: upstox::UpstoxService) -> Self {
        self.upstox = upstox;
        self
    }

    fn application(&self) -> std::result::Result<Application, String> {
        Application::new(
            self.device_id.clone(),
            self.vault_root.clone(),
            self.index_path.clone(),
        )
        .map_err(|e| e.to_string())
    }

    pub fn status(&self) -> AppStatus {
        AppStatus {
            app_version: self.app_version,
            device_id: self.device_id.clone(),
            projection_schema_version: self.projection_schema_version,
            sync_status: sanket_domain::SyncStatus::Pending,
            os_keyring_status: security_status_label(service::resolve_security_mode()),
        }
    }

    fn auth_service(&self) -> Result<&Arc<AuthService>, String> {
        if let Some(error) = self.auth_initialization_error.as_deref() {
            return Err(error.to_owned());
        }
        self.auth_service
            .as_ref()
            .ok_or_else(|| AUTHENTICATION_ERROR.into())
    }

    fn set_active_session(&self, token: Option<String>) -> Result<(), String> {
        let mut active = self
            .active_session_token
            .lock()
            .map_err(|_| AUTHENTICATION_ERROR.to_owned())?;
        if let Some(previous) = active.as_mut() {
            previous.zeroize();
        }
        *active = token;
        Ok(())
    }

    fn active_session(&self) -> Result<Option<String>, String> {
        self.active_session_token
            .lock()
            .map(|token| token.clone())
            .map_err(|_| AUTHENTICATION_ERROR.to_owned())
    }

    fn authenticated_actor(&self) -> Result<(String, sanket_domain::Role), String> {
        let auth = self.auth_service()?;
        let mut token = self
            .active_session()?
            .ok_or_else(|| String::from(AUTHENTICATION_ERROR))?;
        let result = auth.validate_session(&token, current_time_secs());
        token.zeroize();
        match result {
            Some(identity) => Ok(identity),
            None => {
                let _ = self.set_active_session(None);
                Err(AUTHENTICATION_ERROR.into())
            }
        }
    }

    fn manager_actor(&self) -> Result<(String, sanket_domain::Role), String> {
        let actor = self.authenticated_actor()?;
        if actor.1.can_manage_members() {
            Ok(actor)
        } else {
            Err(AUTHENTICATION_ERROR.into())
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthStatus {
    pub ready: bool,
    pub authenticated: bool,
    pub account_id: Option<String>,
    pub role: Option<sanket_domain::Role>,
    /// Stable, non-secret reason when startup could not establish the
    /// authenticated vault boundary. This is distinct from an ordinary signed
    /// out state, which is ready and has no initialization error.
    pub initialization_error: Option<&'static str>,
}

impl AuthStatus {
    fn unavailable(initialization_error: Option<&'static str>) -> Self {
        Self {
            ready: false,
            authenticated: false,
            account_id: None,
            role: None,
            initialization_error,
        }
    }

    fn ready_unauthenticated() -> Self {
        Self {
            ready: true,
            authenticated: false,
            account_id: None,
            role: None,
            initialization_error: None,
        }
    }

    fn authenticated(account_id: String, role: sanket_domain::Role) -> Self {
        Self {
            ready: true,
            authenticated: true,
            account_id: Some(account_id),
            role: Some(role),
            initialization_error: None,
        }
    }
}

fn current_time_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}

fn login_with_state(state: &AppState, request: LoginRequest) -> Result<LoginResponse, String> {
    let auth = state.auth_service()?;
    let response = auth.login_at(request, current_time_secs());
    if response.accepted {
        let Some(token) = response.session_token.clone() else {
            return Err(AUTHENTICATION_ERROR.into());
        };
        state.set_active_session(Some(token))?;
    }
    Ok(response)
}

fn auth_status_for_state(state: &AppState) -> AuthStatus {
    let Some(auth) = state.auth_service.as_ref() else {
        return AuthStatus::unavailable(
            state
                .auth_initialization_error
                .as_ref()
                .map(|_| "AUTH_STARTUP_INTEGRITY_BLOCKED"),
        );
    };
    let Ok(token) = state.active_session() else {
        return AuthStatus::unavailable(Some("AUTH_SESSION_STATE_UNAVAILABLE"));
    };
    let Some(mut token) = token else {
        return AuthStatus::ready_unauthenticated();
    };
    let result = auth.validate_session(&token, current_time_secs());
    token.zeroize();
    match result {
        Some((account_id, role)) => AuthStatus::authenticated(account_id, role),
        None => {
            let _ = state.set_active_session(None);
            AuthStatus::ready_unauthenticated()
        }
    }
}

fn map_auth_error<T>(result: Result<T, auth::AuthError>) -> Result<T, String> {
    result.map_err(|error| match error {
        auth::AuthError::OwnerAlreadyExists => OWNER_ALREADY_EXISTS_ERROR.to_owned(),
        _ => AUTHENTICATION_ERROR.to_owned(),
    })
}

#[tauri::command]
fn bootstrap_owner(
    state: tauri::State<'_, AppState>,
    request: OwnerBootstrapRequest,
) -> Result<LoginResponse, String> {
    let auth = state.auth_service()?;
    let mut password = Zeroizing::new(request.password.clone());
    let login = request.email.clone();
    map_auth_error(auth.bootstrap_owner_at(request, current_time_secs()))?;
    let response = auth.login_at(
        LoginRequest {
            login,
            password: std::mem::take(&mut *password),
        },
        current_time_secs(),
    );
    if !response.accepted {
        return Err(AUTHENTICATION_ERROR.into());
    }
    let Some(token) = response.session_token.clone() else {
        return Err(AUTHENTICATION_ERROR.into());
    };
    state.set_active_session(Some(token))?;
    Ok(response)
}

#[tauri::command]
fn login(
    state: tauri::State<'_, AppState>,
    request: LoginRequest,
) -> Result<LoginResponse, String> {
    login_with_state(&state, request)
}

#[tauri::command]
fn logout(state: tauri::State<'_, AppState>) -> Result<(), String> {
    let mut active = state
        .active_session()?
        .ok_or_else(|| AUTHENTICATION_ERROR.to_owned())?;
    let result = state
        .auth_service()?
        .logout_at(&active, current_time_secs());
    active.zeroize();
    state.set_active_session(None)?;
    map_auth_error(result)
}

#[tauri::command]
fn issue_invite(
    state: tauri::State<'_, AppState>,
    mut request: InviteIssueRequest,
) -> Result<IssuedInvite, String> {
    let (actor_account_id, _) = state.manager_actor()?;
    request.actor_account_id = actor_account_id;
    map_auth_error(
        state
            .auth_service()?
            .issue_invite_at(request, current_time_secs()),
    )
}

#[tauri::command]
fn complete_signup(
    state: tauri::State<'_, AppState>,
    request: SignupRequest,
) -> Result<(), String> {
    map_auth_error(
        state
            .auth_service()?
            .complete_signup_at(request, current_time_secs()),
    )
}

#[tauri::command]
fn approve_account(
    state: tauri::State<'_, AppState>,
    mut request: AccountDecisionRequest,
) -> Result<(), String> {
    let (actor_account_id, _) = state.manager_actor()?;
    request.actor_account_id = actor_account_id;
    map_auth_error(
        state
            .auth_service()?
            .approve_account_at(request, current_time_secs()),
    )
}

#[tauri::command]
fn revoke_account(
    state: tauri::State<'_, AppState>,
    mut request: AccountDecisionRequest,
) -> Result<(), String> {
    let (actor_account_id, _) = state.manager_actor()?;
    request.actor_account_id = actor_account_id;
    map_auth_error(
        state
            .auth_service()?
            .revoke_account_at(request, current_time_secs()),
    )
}

#[tauri::command]
fn get_auth_status(state: tauri::State<'_, AppState>) -> AuthStatus {
    auth_status_for_state(&state)
}

#[tauri::command]
fn get_app_status(state: tauri::State<'_, AppState>) -> AppStatus {
    state.status()
}

fn bind_onboarding_actor(
    mut request: OnboardMemberRequest,
    actor_account_id: String,
) -> OnboardMemberRequest {
    request.member_id = actor_account_id;
    request
}

#[tauri::command]
fn onboard_member(
    state: tauri::State<'_, AppState>,
    request: OnboardMemberRequest,
) -> Result<OnboardMemberResponse, String> {
    let (actor_account_id, _) = state.manager_actor()?;
    let request = bind_onboarding_actor(request, actor_account_id);
    if !request.consented {
        return Err("consent acknowledgement is required".to_owned());
    }
    state
        .application()?
        .onboard_member(request)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn add_friend(
    state: tauri::State<'_, AppState>,
    mut request: AddFriendRequest,
) -> Result<AddFriendResponse, String> {
    let (actor_account_id, _) = state.manager_actor()?;
    if request.owner_member_id != actor_account_id {
        return Err(AUTHENTICATION_ERROR.into());
    }
    request.owner_member_id = actor_account_id;
    state
        .application()?
        .add_friend(request)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn archive_friend(
    state: tauri::State<'_, AppState>,
    friend_id: String,
    owner_member_id: String,
) -> Result<(), String> {
    let (actor_account_id, _) = state.manager_actor()?;
    if owner_member_id != actor_account_id {
        return Err(AUTHENTICATION_ERROR.into());
    }
    state
        .application()?
        .archive_friend(&friend_id, &owner_member_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn list_members(state: tauri::State<'_, AppState>) -> Result<Vec<MemberRow>, String> {
    state.authenticated_actor()?;
    state
        .application()?
        .list_members()
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn list_friends(state: tauri::State<'_, AppState>) -> Result<Vec<FriendRow>, String> {
    state.authenticated_actor()?;
    state
        .application()?
        .list_friends()
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn check_recommendation(
    state: tauri::State<'_, AppState>,
    request: CheckRequest,
) -> Result<CheckResponse, String> {
    state.authenticated_actor()?;
    state
        .application()?
        .check(request)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn submit_investment(
    state: tauri::State<'_, AppState>,
    mut request: SubmitRequest,
) -> Result<SubmitResponse, String> {
    let (actor_member_id, _) = state.authenticated_actor()?;
    request.actor_member_id = actor_member_id;
    let request = state
        .upstox
        .prepare_submission(request)
        .map_err(|e| e.to_string())?;
    state
        .application()?
        .submit(request)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn record_historical_application(
    state: tauri::State<'_, AppState>,
    mut request: HistoricalApplicationRequest,
) -> Result<HistoricalApplicationResponse, String> {
    let (actor_member_id, _) = state.authenticated_actor()?;
    request.actor_member_id = actor_member_id;
    state
        .application()?
        .record_historical_application(request)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn void_submitted_session(
    state: tauri::State<'_, AppState>,
    mut request: VoidSessionRequest,
) -> Result<VoidSessionResponse, String> {
    let (actor_member_id, _) = state.authenticated_actor()?;
    request.actor_member_id = actor_member_id;
    state
        .application()?
        .void_submitted_session(request)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn get_dashboard(state: tauri::State<'_, AppState>) -> Result<Dashboard, String> {
    state.authenticated_actor()?;
    state.application()?.dashboard().map_err(|e| e.to_string())
}

#[tauri::command]
fn list_allotment_candidates(
    state: tauri::State<'_, AppState>,
) -> Result<Vec<AllotmentCandidateRow>, String> {
    state.authenticated_actor()?;
    state
        .application()?
        .list_allotment_candidates()
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn start_allotment_check(
    state: tauri::State<'_, AppState>,
    mut request: StartAllotmentRequest,
) -> Result<AllotmentJobReport, String> {
    let (actor_member_id, _) = state.authenticated_actor()?;
    request.actor_member_id = actor_member_id;
    let report = state
        .application()?
        .enqueue_allotment_check(request)
        .map_err(|e| e.to_string())?;
    if let Some(worker) = &state.worker {
        worker.notify();
    }
    Ok(report)
}

#[tauri::command]
fn cancel_allotment_job(state: tauri::State<'_, AppState>, job_id: String) -> Result<bool, String> {
    state.authenticated_actor()?;
    state
        .application()?
        .cancel_allotment_job(&job_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn get_allotment_report(
    state: tauri::State<'_, AppState>,
    job_id: String,
) -> Result<AllotmentJobReport, String> {
    state.authenticated_actor()?;
    state
        .application()?
        .get_allotment_report(&job_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn record_manual_allotment(
    state: tauri::State<'_, AppState>,
    mut request: ManualAllotmentRequest,
) -> Result<service::AllotmentReportRow, String> {
    let (actor_member_id, _) = state.authenticated_actor()?;
    request.actor_member_id = actor_member_id;
    state
        .application()?
        .record_manual_allotment_result(request)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn estimate_profit(
    state: tauri::State<'_, AppState>,
    mut request: EstimateProfitRequest,
) -> Result<EstimatedProfitDto, String> {
    let (actor_member_id, _) = state.authenticated_actor()?;
    request.actor_member_id = actor_member_id;
    state
        .application()?
        .estimate_profit(request)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn get_security_status(state: tauri::State<'_, AppState>) -> Result<SecurityStatusDto, String> {
    state.authenticated_actor()?;
    state
        .application()?
        .security_status()
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn get_lookup_authorization_status(
    state: tauri::State<'_, AppState>,
    application_id: String,
) -> Result<LookupAuthorizationStatusDto, String> {
    state.authenticated_actor()?;
    state
        .application()?
        .get_lookup_authorization_status(&application_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn authorize_real_investor_lookup(
    state: tauri::State<'_, AppState>,
    mut request: LookupAuthorizationRequest,
) -> Result<LookupAuthorizationStatusDto, String> {
    let (actor_member_id, role) = state.authenticated_actor()?;
    if role != sanket_domain::Role::Owner {
        return Err(AUTHENTICATION_ERROR.into());
    }
    request.actor_member_id = actor_member_id;
    state
        .application()?
        .authorize_real_investor_lookup(request)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn get_upstox_connection_status(state: tauri::State<'_, AppState>) -> ProviderConnectionStatusDto {
    if state.authenticated_actor().is_err() {
        return ProviderConnectionStatusDto::not_connected();
    }
    status(&state.provider_credentials)
}

#[tauri::command]
fn connect_upstox_analytics_token(
    state: tauri::State<'_, AppState>,
    mut request: ConnectUpstoxAnalyticsTokenRequest,
) -> ProviderConnectionStatusDto {
    if state.manager_actor().is_err() {
        return ProviderConnectionStatusDto::not_connected();
    }
    let token = std::mem::take(&mut request.token);
    request.token.clear();
    store_token(&state.provider_credentials, SecretValue::new(token))
}

#[tauri::command]
fn replace_upstox_analytics_token(
    state: tauri::State<'_, AppState>,
    mut request: ConnectUpstoxAnalyticsTokenRequest,
) -> ProviderConnectionStatusDto {
    if state.manager_actor().is_err() {
        return ProviderConnectionStatusDto::not_connected();
    }
    let token = std::mem::take(&mut request.token);
    request.token.clear();
    store_token(&state.provider_credentials, SecretValue::new(token))
}

#[tauri::command]
fn disconnect_upstox(state: tauri::State<'_, AppState>) -> ProviderConnectionStatusDto {
    if state.manager_actor().is_err() {
        return ProviderConnectionStatusDto::not_connected();
    }
    disconnect(&state.provider_credentials)
}

#[tauri::command]
fn list_available_ipos(
    state: tauri::State<'_, AppState>,
    query: IpoListQuery,
) -> Result<IpoCatalogueDto, String> {
    state.authenticated_actor()?;
    upstox::list_available_ipos(&state, query)
}

#[tauri::command]
fn refresh_ipo_catalog(
    state: tauri::State<'_, AppState>,
    query: IpoListQuery,
) -> Result<IpoCatalogueDto, String> {
    state.authenticated_actor()?;
    upstox::refresh_ipo_catalog(&state, query)
}

#[tauri::command]
fn get_ipo_details(
    state: tauri::State<'_, AppState>,
    source_ipo_id: String,
) -> Result<IpoCatalogItemDto, String> {
    state.authenticated_actor()?;
    upstox::get_ipo_details(&state, source_ipo_id)
}

pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let app_data_dir = app.path().app_data_dir()?;
            let settings = sanket_device_settings::DeviceSettingsStore::load_or_initialize(
                &app_data_dir.join("settings.json"),
            )?;
            let index_path = app_data_dir.join("index.sqlite3");
            let vault_root = app_data_dir.join("member-vault");
            let local_index = sanket_local_index::LocalIndex::open(&index_path)?;
            let schema_version = local_index.schema_version()?;
            let mode = service::resolve_security_mode();
            let worker = worker::spawn_allotment_worker(
                settings.device_id.clone(),
                vault_root.clone(),
                index_path.clone(),
                mode,
            );
            let upstox_cache = app_data_dir.join("public-cache/upstox-ipo.json");
            app.manage(
                AppState::build(settings.device_id, schema_version, vault_root, index_path)
                    .with_upstox(upstox::UpstoxService::new(upstox_cache))
                    .with_worker(worker),
            );
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_app_status,
            bootstrap_owner,
            login,
            logout,
            issue_invite,
            complete_signup,
            approve_account,
            revoke_account,
            get_auth_status,
            onboard_member,
            add_friend,
            archive_friend,
            list_members,
            list_friends,
            check_recommendation,
            submit_investment,
            record_historical_application,
            void_submitted_session,
            get_dashboard,
            list_allotment_candidates,
            start_allotment_check,
            get_allotment_report,
            cancel_allotment_job,
            record_manual_allotment,
            estimate_profit,
            get_security_status,
            get_lookup_authorization_status,
            authorize_real_investor_lookup,
            get_upstox_connection_status,
            connect_upstox_analytics_token,
            replace_upstox_analytics_token,
            disconnect_upstox,
            list_available_ipos,
            refresh_ipo_catalog,
            get_ipo_details
        ])
        .run(tauri::generate_context!())
        .expect("Sanket IPO desktop runtime failed");
}

#[cfg(test)]
mod native_auth_command_tests {
    use super::*;

    fn owner_state() -> AppState {
        let root = tempfile::tempdir().expect("temporary auth root");
        let state = AppState::build(
            "device-native-auth-test".into(),
            1,
            root.path().join("vault"),
            root.path().join("index.sqlite"),
        );
        let auth = state.auth_service.as_ref().expect("auth service");
        auth.bootstrap_owner_at(
            auth::OwnerBootstrapRequest {
                account_id: "owner-1".into(),
                email: "owner@example.invalid".into(),
                password: "owner-password".into(),
            },
            100,
        )
        .expect("bootstrap owner");
        std::mem::forget(root);
        state
    }

    #[test]
    fn compatibility_constructor_has_no_auth_vault_or_session() {
        let state = AppState::new("device-status-only".into(), 1);
        assert!(state.auth_service.is_none());
        assert!(
            state
                .active_session_token
                .lock()
                .expect("session lock")
                .is_none()
        );
    }

    #[test]
    fn rejected_login_cannot_install_native_session() {
        let state = owner_state();
        let response = login_with_state(
            &state,
            auth::LoginRequest {
                login: "owner@example.invalid".into(),
                password: "wrong-password".into(),
            },
        )
        .expect("generic login response");
        assert!(!response.accepted);
        assert!(
            state
                .active_session_token
                .lock()
                .expect("session lock")
                .is_none()
        );
    }

    #[test]
    fn second_owner_bootstrap_maps_to_sign_in_message() {
        let state = owner_state();
        let auth = state.auth_service.as_ref().expect("auth service");
        let result = map_auth_error(auth.bootstrap_owner_at(
            auth::OwnerBootstrapRequest {
                account_id: "owner-2".into(),
                email: "second-owner@example.invalid".into(),
                password: "second-password".into(),
            },
            101,
        ));

        assert_eq!(
            result,
            Err("Owner account already exists. Sign in instead.".to_owned())
        );
        assert_eq!(auth.reconstruct_state().unwrap().owner_count(), 1);
    }

    #[test]
    fn accepted_login_is_visible_only_through_native_auth_status() {
        let state = owner_state();
        let response = login_with_state(
            &state,
            auth::LoginRequest {
                login: "owner@example.invalid".into(),
                password: "owner-password".into(),
            },
        )
        .expect("login response");
        assert!(response.accepted);
        assert!(response.session_token.is_some());
        let status = auth_status_for_state(&state);
        assert!(status.ready);
        assert!(status.authenticated);
        assert_eq!(status.account_id.as_deref(), Some("owner-1"));
    }

    #[test]
    fn onboarding_binds_member_identity_to_authenticated_account() {
        let request = OnboardMemberRequest {
            member_id: "ui-generated-member-id".into(),
            display_name: "Owner".into(),
            email: "owner@example.invalid".into(),
            role: "OWNER".into(),
            primary_account_label: None,
            broker: None,
            upi_id: String::new(),
            pan: String::new(),
            consented: true,
        };
        let bound = bind_onboarding_actor(request, "owner-1".into());
        assert_eq!(bound.member_id, "owner-1");
    }

    #[test]
    fn production_security_status_uses_os_keyring_label() {
        assert_eq!(
            security_status_label(sanket_identity_security::RuntimeSecurityMode::ProductionSecure),
            "OS KEYRING (PRODUCTION_SECURE)"
        );
    }

    #[test]
    fn unavailable_auth_state_exposes_integrity_startup_blocker() {
        let mut state = AppState::new("device-status-only".into(), 1);
        state.auth_initialization_error = Some("authentication unavailable: anchor verification failed".into());
        let status = auth_status_for_state(&state);
        assert!(!status.ready);
        assert!(!status.authenticated);
        assert_eq!(status.initialization_error, Some("AUTH_STARTUP_INTEGRITY_BLOCKED"));
    }

    #[test]
    fn unavailable_auth_state_fails_closed_for_actor_validation() {
        let state = AppState::new("device-status-only".into(), 1);
        assert_eq!(
            state.authenticated_actor(),
            Err(AUTHENTICATION_ERROR.into())
        );
    }
}
