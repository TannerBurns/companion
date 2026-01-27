//! Data synchronization module

pub mod oauth;
pub mod slack;
pub mod atlassian;
pub mod background;
pub mod queue;

// Re-export commonly used types
pub use slack::{SlackClient, SlackSyncService, SlackTokens, SyncResult, SlackChannel, SlackChannelSelection, SlackConnectionStatus, SlackUser};
pub use atlassian::{AtlassianClient, AtlassianSyncService, AtlassianTokens, CloudResource};
pub use background::{BackgroundSyncService, sync_slack_now, sync_slack_historical_day, get_last_sync_at};
pub use queue::{SyncQueue, SyncRequest};
