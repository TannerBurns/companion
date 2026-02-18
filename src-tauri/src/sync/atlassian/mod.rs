//! Atlassian integration module
//!
//! This module provides OAuth 2.0 (3LO) authentication with PKCE and data
//! synchronization for Jira and Confluence.

mod client;
mod sync;
mod types;

pub use client::AtlassianClient;
pub use sync::AtlassianSyncService;
pub use types::{AtlassianError, AtlassianTokens, CloudResource, ConfluencePage, JiraIssue};
