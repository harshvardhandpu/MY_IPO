//! Durable local allotment worker.
//!
//! Jobs live in events/SQLite; this thread is only an executor. Reopening the app
//! reconciles expired/unowned jobs, so process restarts do not duplicate work.

use std::path::PathBuf;
use std::sync::mpsc::{self, RecvTimeoutError, Sender};
use std::time::Duration;

use sanket_identity_security::RuntimeSecurityMode;

use crate::service::Application;

#[derive(Clone, Debug)]
pub struct AllotmentWorkerHandle {
    wake: Sender<()>,
}

impl AllotmentWorkerHandle {
    pub fn notify(&self) {
        let _ = self.wake.send(());
    }
}

pub fn spawn_allotment_worker(
    device_id: String,
    vault_root: PathBuf,
    index_path: PathBuf,
    mode: RuntimeSecurityMode,
) -> AllotmentWorkerHandle {
    let (wake, rx) = mpsc::channel();
    std::thread::Builder::new()
        .name("sanket-allotment-worker".into())
        .spawn(move || {
            loop {
                match Application::with_mode(
                    device_id.clone(),
                    vault_root.clone(),
                    index_path.clone(),
                    mode,
                ) {
                    Ok(app) => match app.resumable_allotment_job_ids() {
                        Ok(job_ids) => {
                            for job_id in job_ids {
                                if let Err(error) = app.run_allotment_job_once(&job_id) {
                                    // Error text is sanitized at source; never log request bodies.
                                    eprintln!("allotment worker job_id={job_id} failed: {error}");
                                }
                            }
                        }
                        Err(error) => eprintln!("allotment worker reconcile failed: {error}"),
                    },
                    Err(error) => eprintln!("allotment worker startup blocked: {error}"),
                }

                match rx.recv_timeout(Duration::from_secs(2)) {
                    Ok(()) | Err(RecvTimeoutError::Timeout) => continue,
                    Err(RecvTimeoutError::Disconnected) => break,
                }
            }
        })
        .expect("allotment worker thread should start");
    AllotmentWorkerHandle { wake }
}
