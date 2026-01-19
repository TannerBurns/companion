// Library entry point for Tauri
pub mod commands;
pub mod crypto;
pub mod db;
pub mod ai;
pub mod sync;
pub mod notifications;
pub mod analytics;
pub mod pipeline;
pub mod tray;

use db::Database;
use crypto::CryptoService;
use notifications::NotificationService;
use analytics::AnalyticsService;
use pipeline::PipelineManager;
use sync::{SyncQueue, BackgroundSyncService};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicI64};

pub struct AppState {
    pub db: Arc<Database>,
    pub crypto: CryptoService,
    pub notifications: Option<NotificationService>,
    pub analytics: Option<AnalyticsService>,
    pub pipeline: Arc<tokio::sync::Mutex<PipelineManager>>,
    pub sync_queue: SyncQueue,
    /// Prevents concurrent sync operations from racing on topic updates.
    pub sync_lock: Arc<tokio::sync::Mutex<()>>,
    pub background_sync: Option<Arc<BackgroundSyncService>>,
    pub is_syncing: Arc<AtomicBool>,
    pub next_sync_at: Arc<AtomicI64>,
}
