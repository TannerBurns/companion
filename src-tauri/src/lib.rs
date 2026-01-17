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
use sync::SyncQueue;
use std::sync::Arc;

pub struct AppState {
    pub db: Database,
    pub crypto: CryptoService,
    pub notifications: Option<NotificationService>,
    pub analytics: Option<AnalyticsService>,
    pub pipeline: Arc<tokio::sync::Mutex<PipelineManager>>,
    pub sync_queue: SyncQueue,
}
