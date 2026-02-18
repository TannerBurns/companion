//! Slack integration module
//!
//! This module provides OAuth authentication and data synchronization
//! for Slack workspaces.

mod client;
mod sync;
mod types;

pub use client::SlackClient;
pub use sync::SlackSyncService;
pub use types::{
    SlackAuthInfo, SlackChannel, SlackChannelSelection, SlackConnectionStatus, SlackError,
    SlackMessage, SlackTokens, SlackUser, SyncResult,
};
