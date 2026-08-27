use sanket_device_settings::DeviceSettingsStore;
use sanket_domain::SyncStatus;
use sanket_local_index::LocalIndex;
use serde::Serialize;
use tauri::Manager;

#[derive(Clone, Debug)]
pub struct AppState {
    device_id: String,
    projection_schema_version: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct AppStatus {
    pub app_version: &'static str,
    pub device_id: String,
    pub projection_schema_version: u32,
    pub sync_status: SyncStatus,
}

impl AppState {
    pub fn new(device_id: String, projection_schema_version: u32) -> Self {
        Self {
            device_id,
            projection_schema_version,
        }
    }

    pub fn status(&self) -> AppStatus {
        AppStatus {
            app_version: env!("CARGO_PKG_VERSION"),
            device_id: self.device_id.clone(),
            projection_schema_version: self.projection_schema_version,
            sync_status: SyncStatus::Pending,
        }
    }
}

#[tauri::command]
fn get_app_status(state: tauri::State<'_, AppState>) -> AppStatus {
    state.status()
}

pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let app_data_dir = app.path().app_data_dir()?;
            let settings =
                DeviceSettingsStore::load_or_initialize(&app_data_dir.join("settings.json"))?;
            let local_index = LocalIndex::open(&app_data_dir.join("index.sqlite3"))?;
            let schema_version = local_index.schema_version()?;
            app.manage(AppState::new(settings.device_id, schema_version));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![get_app_status])
        .run(tauri::generate_context!())
        .expect("Sanket IPO desktop runtime failed");
}
