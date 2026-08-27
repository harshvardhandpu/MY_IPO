use std::time::Duration;

use sanket_domain::SyncStatus;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SyncPolicy {
    pub push_debounce: Duration,
    pub active_fetch_interval: Duration,
}

impl Default for SyncPolicy {
    fn default() -> Self {
        Self {
            push_debounce: Duration::from_secs(5),
            active_fetch_interval: Duration::from_secs(20),
        }
    }
}

pub trait SyncEngine: Send + Sync {
    type Error;

    fn status(&self) -> SyncStatus;
    fn queue_local_change(&self) -> Result<(), Self::Error>;
    fn sync_now(&self) -> Result<(), Self::Error>;
}
