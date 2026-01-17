//! Atlassian integration module
//!
//! This module provides OAuth 2.0 (3LO) authentication with PKCE and data
//! synchronization for Jira and Confluence.

mod types;
mod client;
mod sync;

pub use types::{AtlassianError, AtlassianTokens, CloudResource, JiraIssue, ConfluencePage};
pub use client::AtlassianClient;
pub use sync::AtlassianSyncService;
