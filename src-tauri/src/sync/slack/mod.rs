//! Slack integration module
//!
//! This module provides OAuth authentication and data synchronization
//! for Slack workspaces.

mod types;
mod client;
mod sync;

pub use types::{SlackError, SlackTokens, SlackChannel, SlackMessage, SyncResult};
pub use client::SlackClient;
pub use sync::SlackSyncService;
