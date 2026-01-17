pub mod schema;
mod migrations;

use sqlx::{sqlite::SqlitePoolOptions, SqlitePool};
use tauri::{AppHandle, Manager};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum DbError {
    #[error("SQLx error: {0}")]
    Sqlx(#[from] sqlx::Error),
    
    #[error("Migration error: {0}")]
    Migration(#[from] sqlx::migrate::MigrateError),
    
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

pub struct Database {
    pool: SqlitePool,
}

impl Database {
    pub async fn new(app_handle: &AppHandle) -> Result<Self, DbError> {
        // Get app data directory
        let app_dir = app_handle
            .path()
            .app_data_dir()
            .expect("Failed to get app data dir");
        
        // Ensure directory exists
        std::fs::create_dir_all(&app_dir)?;
        
        let db_path = app_dir.join("companion.db");
        let db_url = format!("sqlite:{}?mode=rwc", db_path.display());
        
        tracing::info!("Opening database at: {}", db_path.display());
        
        // Create connection pool
        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect(&db_url)
            .await?;
        
        // Run migrations
        sqlx::migrate!("./migrations")
            .run(&pool)
            .await?;
        
        tracing::info!("Database initialized successfully");
        
        Ok(Self { pool })
    }
    
    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }
}
