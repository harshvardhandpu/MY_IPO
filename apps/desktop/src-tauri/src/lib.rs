pub mod provider_credentials;
pub mod service;
pub mod worker;

use std::path::PathBuf;

use serde::Serialize;
use tauri::Manager;

use provider_credentials::{
    ConnectUpstoxAnalyticsTokenRequest, OsProviderCredentialStore, ProviderConnectionStatusDto,
    SecretValue, disconnect, status, store_token,
};
use service::{
    AddFriendRequest, AddFriendResponse, AllotmentCandidateRow, AllotmentJobReport, Application,
    CheckRequest, CheckResponse, Dashboard, EstimateProfitRequest, EstimatedProfitDto, FriendRow,
    HistoricalApplicationRequest, HistoricalApplicationResponse, ManualAllotmentRequest, MemberRow,
    OnboardMemberRequest, OnboardMemberResponse, SecurityStatusDto, StartAllotmentRequest,
    SubmitRequest, SubmitResponse, VoidSessionRequest, VoidSessionResponse,
};

#[derive(Clone, Debug)]
pub struct AppState {
    app_version: &'static str,
    device_id: String,
    projection_schema_version: u32,
    vault_root: PathBuf,
    index_path: PathBuf,
    worker: Option<worker::AllotmentWorkerHandle>,
    provider_credentials: OsProviderCredentialStore,
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
        Self {
            app_version: env!("CARGO_PKG_VERSION"),
            device_id,
            projection_schema_version,
            vault_root,
            index_path,
            worker: None,
            provider_credentials: OsProviderCredentialStore,
        }
    }

    pub fn with_worker(mut self, worker: worker::AllotmentWorkerHandle) -> Self {
        self.worker = Some(worker);
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
            os_keyring_status: if std::env::var("SANKET_SECURITY_MODE")
                .map(|s| s.to_ascii_uppercase().contains("PRODUCTION"))
                .unwrap_or(false)
            {
                "OS KEYRING (PRODUCTION_SECURE)"
            } else {
                "DEV SYNTHETIC / IN-MEMORY — OS KEYRING REQUIRED BEFORE REAL PAN"
            },
        }
    }
}

#[tauri::command]
fn get_app_status(state: tauri::State<'_, AppState>) -> AppStatus {
    state.status()
}

#[tauri::command]
fn onboard_member(
    state: tauri::State<'_, AppState>,
    request: OnboardMemberRequest,
) -> Result<OnboardMemberResponse, String> {
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
    request: AddFriendRequest,
) -> Result<AddFriendResponse, String> {
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
    state
        .application()?
        .archive_friend(&friend_id, &owner_member_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn list_members(state: tauri::State<'_, AppState>) -> Result<Vec<MemberRow>, String> {
    state
        .application()?
        .list_members()
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn list_friends(state: tauri::State<'_, AppState>) -> Result<Vec<FriendRow>, String> {
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
    state
        .application()?
        .check(request)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn submit_investment(
    state: tauri::State<'_, AppState>,
    request: SubmitRequest,
) -> Result<SubmitResponse, String> {
    state
        .application()?
        .submit(request)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn record_historical_application(
    state: tauri::State<'_, AppState>,
    request: HistoricalApplicationRequest,
) -> Result<HistoricalApplicationResponse, String> {
    state
        .application()?
        .record_historical_application(request)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn void_submitted_session(
    state: tauri::State<'_, AppState>,
    request: VoidSessionRequest,
) -> Result<VoidSessionResponse, String> {
    state
        .application()?
        .void_submitted_session(request)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn get_dashboard(state: tauri::State<'_, AppState>) -> Result<Dashboard, String> {
    state.application()?.dashboard().map_err(|e| e.to_string())
}

#[tauri::command]
fn list_allotment_candidates(
    state: tauri::State<'_, AppState>,
) -> Result<Vec<AllotmentCandidateRow>, String> {
    state
        .application()?
        .list_allotment_candidates()
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn start_allotment_check(
    state: tauri::State<'_, AppState>,
    request: StartAllotmentRequest,
) -> Result<AllotmentJobReport, String> {
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
    state
        .application()?
        .get_allotment_report(&job_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn record_manual_allotment(
    state: tauri::State<'_, AppState>,
    request: ManualAllotmentRequest,
) -> Result<service::AllotmentReportRow, String> {
    state
        .application()?
        .record_manual_allotment_result(request)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn estimate_profit(
    state: tauri::State<'_, AppState>,
    request: EstimateProfitRequest,
) -> Result<EstimatedProfitDto, String> {
    state
        .application()?
        .estimate_profit(request)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn get_security_status(state: tauri::State<'_, AppState>) -> Result<SecurityStatusDto, String> {
    state
        .application()?
        .security_status()
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn get_upstox_connection_status(state: tauri::State<'_, AppState>) -> ProviderConnectionStatusDto {
    status(&state.provider_credentials)
}

#[tauri::command]
fn connect_upstox_analytics_token(
    state: tauri::State<'_, AppState>,
    mut request: ConnectUpstoxAnalyticsTokenRequest,
) -> ProviderConnectionStatusDto {
    let token = std::mem::take(&mut request.token);
    request.token.clear();
    store_token(&state.provider_credentials, SecretValue::new(token))
}

#[tauri::command]
fn replace_upstox_analytics_token(
    state: tauri::State<'_, AppState>,
    mut request: ConnectUpstoxAnalyticsTokenRequest,
) -> ProviderConnectionStatusDto {
    let token = std::mem::take(&mut request.token);
    request.token.clear();
    store_token(&state.provider_credentials, SecretValue::new(token))
}

#[tauri::command]
fn disconnect_upstox(state: tauri::State<'_, AppState>) -> ProviderConnectionStatusDto {
    disconnect(&state.provider_credentials)
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
            let mode = std::env::var("SANKET_SECURITY_MODE")
                .map(|value| sanket_identity_security::RuntimeSecurityMode::parse(&value))
                .unwrap_or(sanket_identity_security::RuntimeSecurityMode::DevelopmentSynthetic);
            let worker = worker::spawn_allotment_worker(
                settings.device_id.clone(),
                vault_root.clone(),
                index_path.clone(),
                mode,
            );
            app.manage(
                AppState::build(settings.device_id, schema_version, vault_root, index_path)
                    .with_worker(worker),
            );
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_app_status,
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
            get_upstox_connection_status,
            connect_upstox_analytics_token,
            replace_upstox_analytics_token,
            disconnect_upstox
        ])
        .run(tauri::generate_context!())
        .expect("Sanket IPO desktop runtime failed");
}
