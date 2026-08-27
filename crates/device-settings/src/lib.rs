use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use thiserror::Error;
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;
use uuid::Uuid;

const SETTINGS_SCHEMA_VERSION: u16 = 1;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeviceSettings {
    pub schema_version: u16,
    pub device_id: String,
    pub created_at: String,
}

pub struct DeviceSettingsStore;

#[derive(Debug, Error)]
pub enum DeviceSettingsError {
    #[error("device settings I/O failed: {0}")]
    Io(#[from] std::io::Error),
    #[error("device settings JSON is invalid: {0}")]
    Json(#[from] serde_json::Error),
    #[error("device settings timestamp failed: {0}")]
    Timestamp(#[from] time::error::Format),
    #[error("unsupported device settings schema {0}")]
    UnsupportedSchema(u16),
    #[error("device identity cannot be empty")]
    EmptyDeviceIdentity,
}

impl DeviceSettingsStore {
    pub fn load_or_initialize(path: &Path) -> Result<DeviceSettings, DeviceSettingsError> {
        let created_at = OffsetDateTime::now_utc().format(&Rfc3339)?;
        Self::load_or_initialize_with(path, || Uuid::now_v7().to_string(), || created_at)
    }

    pub fn load_or_initialize_with<Id, Clock>(
        path: &Path,
        generate_id: Id,
        now: Clock,
    ) -> Result<DeviceSettings, DeviceSettingsError>
    where
        Id: FnOnce() -> String,
        Clock: FnOnce() -> String,
    {
        if path.exists() {
            return Self::read(path);
        }
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let settings = DeviceSettings {
            schema_version: SETTINGS_SCHEMA_VERSION,
            device_id: generate_id(),
            created_at: now(),
        };
        validate(&settings)?;
        write_atomically(path, &serde_json::to_vec_pretty(&settings)?)?;
        Ok(settings)
    }

    fn read(path: &Path) -> Result<DeviceSettings, DeviceSettingsError> {
        let settings: DeviceSettings = serde_json::from_slice(&fs::read(path)?)?;
        validate(&settings)?;
        Ok(settings)
    }
}

fn validate(settings: &DeviceSettings) -> Result<(), DeviceSettingsError> {
    if settings.schema_version != SETTINGS_SCHEMA_VERSION {
        return Err(DeviceSettingsError::UnsupportedSchema(
            settings.schema_version,
        ));
    }
    if settings.device_id.trim().is_empty() {
        return Err(DeviceSettingsError::EmptyDeviceIdentity);
    }
    Ok(())
}

fn write_atomically(path: &Path, bytes: &[u8]) -> Result<(), std::io::Error> {
    let temporary_path = temporary_path(path);
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(&temporary_path)?;
    if let Err(error) = (|| {
        file.write_all(bytes)?;
        file.sync_all()?;
        fs::rename(&temporary_path, path)?;
        Ok::<(), std::io::Error>(())
    })() {
        let _ = fs::remove_file(&temporary_path);
        return Err(error);
    }
    Ok(())
}

fn temporary_path(path: &Path) -> PathBuf {
    let mut name = path.as_os_str().to_owned();
    name.push(format!(".{}.tmp", std::process::id()));
    PathBuf::from(name)
}
