// Library entry point for Tauri
pub mod commands;
pub mod crypto;
pub mod db;
pub mod ai;
pub mod sync;

use db::Database;
use crypto::CryptoService;

pub struct AppState {
    pub db: Database,
    pub crypto: CryptoService,
}
