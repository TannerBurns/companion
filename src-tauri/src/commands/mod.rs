//! Command handlers for Tauri IPC.
//!
//! This module contains all the Tauri command handlers organized by functionality:
//! - `types` - Shared data types
//! - `digest` - Daily and weekly digest retrieval
//! - `sync` - Data synchronization
//! - `credentials` - API key and credential management
//! - `preferences` - User preferences
//! - `analytics` - Event tracking and analytics
//! - `data` - Data management and factory reset
//! - `slack` - Slack-specific commands

mod analytics;
mod credentials;
mod data;
mod digest;
mod preferences;
mod slack;
mod sync;
mod types;

// Re-export all commands using wildcard to include Tauri's internal __cmd__ symbols
pub use analytics::*;
pub use credentials::*;
pub use data::*;
pub use digest::*;
pub use preferences::*;
pub use slack::*;
pub use sync::*;

// Re-export types for use by other modules
pub use types::{
    AnalyticsSummary, CategorySummary, ClearDataResult, DataStats, DigestItem, DigestResponse,
    Preferences, SourceStatus, SyncResult, SyncStatus,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_types_reexported() {
        // Verify types are accessible
        let _prefs = Preferences::default();
        let _stats = DataStats {
            content_items: 0,
            ai_summaries: 0,
            slack_users: 0,
            sync_states: 0,
        };
    }

    #[test]
    fn test_digest_item_accessible() {
        let item = DigestItem {
            id: "test".to_string(),
            title: "Test".to_string(),
            summary: "Summary".to_string(),
            highlights: None,
            category: "test".to_string(),
            source: "slack".to_string(),
            source_url: None,
            importance_score: 0.5,
            created_at: 0,
            channels: None,
            people: None,
            message_count: None,
        };
        
        assert_eq!(item.id, "test");
    }

    #[test]
    fn test_sync_types_accessible() {
        let status = SyncStatus {
            is_syncing: false,
            last_sync_at: None,
            next_sync_at: None,
            sources: vec![],
        };
        
        assert!(!status.is_syncing);
        
        let result = SyncResult {
            items_synced: 0,
            channels_processed: 0,
            errors: vec![],
        };
        
        assert_eq!(result.items_synced, 0);
    }
}
