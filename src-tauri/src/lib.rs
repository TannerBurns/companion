// Library entry point for Tauri
pub mod ai;
pub mod analytics;
pub mod commands;
pub mod crypto;
pub mod db;
pub mod notifications;
pub mod pipeline;
pub mod sync;
pub mod tray;

use analytics::AnalyticsService;
use crypto::CryptoService;
use db::Database;
use notifications::NotificationService;
use pipeline::PipelineManager;
use std::sync::atomic::{AtomicBool, AtomicI64};
use std::sync::Arc;
use sync::{BackgroundSyncService, SyncQueue};

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
