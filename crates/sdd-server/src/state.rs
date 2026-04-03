use sdd_core::{ScanResult, ScanState, ScanStatus};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Static configuration read from env vars at startup.
#[derive(Clone)]
pub struct AppConfig {
    pub requirements_path: PathBuf,
    pub tasks_path: PathBuf,
    pub source_path: PathBuf,
}

/// Shared mutable server state protected by a read-write lock.
pub struct AppState {
    /// Most recent completed scan result, `None` before the first scan finishes.
    pub scan_result: Option<ScanResult>,
    /// Current scan lifecycle state returned by `GET /scan`.
    pub scan_status: ScanStatus,
    /// Static configuration.
    pub config: AppConfig,
    /// Handle to the currently running scan task, used for cancellation.
    pub current_scan_handle: Option<tokio::task::AbortHandle>,
}

impl AppState {
    pub fn new(config: AppConfig) -> Self {
        Self {
            scan_result: None,
            scan_status: ScanStatus {
                status: ScanState::Idle,
                started_at: None,
                completed_at: None,
                duration: None,
            },
            config,
            current_scan_handle: None,
        }
    }
}

pub type SharedState = Arc<RwLock<AppState>>;
