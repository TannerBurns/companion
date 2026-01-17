//! Slack data types and error definitions

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum SlackError {
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),
    
    #[error("OAuth error: {0}")]
    OAuth(String),
    
    #[error("API error: {0}")]
    Api(String),
    
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),
    
    #[error("Crypto error: {0}")]
    Crypto(String),
    
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SlackTokens {
    #[serde(alias = "access_token")]
    pub access_token: String,
    #[serde(alias = "token_type")]
    pub token_type: String,
    pub scope: String,
    #[serde(alias = "team_id")]
    pub team_id: String,
    #[serde(alias = "team_name")]
    pub team_name: String,
    #[serde(alias = "user_id")]
    pub user_id: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct OAuthResponse {
    pub ok: bool,
    #[allow(dead_code)]
    pub access_token: Option<String>,
    #[allow(dead_code)]
    pub token_type: Option<String>,
    #[allow(dead_code)]
    pub scope: Option<String>,
    pub team: Option<TeamInfo>,
    pub authed_user: Option<AuthedUser>,
    pub error: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct TeamInfo {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct AuthedUser {
    pub id: String,
    pub access_token: Option<String>,
    pub token_type: Option<String>,
    pub scope: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SlackChannel {
    pub id: String,
    pub name: String,
    pub is_private: bool,
    pub is_im: bool,
    pub is_mpim: bool,
    /// For DMs (is_im=true), this is the user ID of the other person
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub member_count: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub purpose: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub topic: Option<String>,
}

/// Represents a user's selection of Slack channels for syncing
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SlackChannelSelection {
    pub channel_id: String,
    pub channel_name: String,
    pub is_private: bool,
    pub is_im: bool,
    pub is_mpim: bool,
    pub team_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub member_count: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub purpose: Option<String>,
    pub enabled: bool,
}

/// Slack connection status
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SlackConnectionStatus {
    pub connected: bool,
    pub team_id: Option<String>,
    pub team_name: Option<String>,
    pub user_id: Option<String>,
    pub selected_channel_count: i32,
}

/// Auth test response info
#[derive(Debug, Clone)]
pub struct SlackAuthInfo {
    pub team_id: String,
    pub team_name: String,
    pub user_id: String,
    pub user_name: String,
}

/// Slack user info
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SlackUser {
    pub id: String,
    pub name: String,
    pub real_name: Option<String>,
    pub display_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlackMessage {
    pub ts: String,
    pub user: Option<String>,
    pub text: String,
    pub thread_ts: Option<String>,
    pub reply_count: Option<i32>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SyncResult {
    pub source: String,
    pub items_synced: i32,
    pub errors: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_slack_tokens_serialization() {
        let tokens = SlackTokens {
            access_token: "xoxp-123".into(),
            token_type: "bearer".into(),
            scope: "channels:read".into(),
            team_id: "T123".into(),
            team_name: "Test Team".into(),
            user_id: "U123".into(),
        };
        
        let json = serde_json::to_string(&tokens).unwrap();
        assert!(json.contains("xoxp-123"));
        assert!(json.contains("Test Team"));
        
        let parsed: SlackTokens = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.access_token, "xoxp-123");
        assert_eq!(parsed.team_name, "Test Team");
    }

    #[test]
    fn test_slack_channel_serialization() {
        let channel = SlackChannel {
            id: "C123".into(),
            name: "general".into(),
            is_private: false,
            is_im: false,
            is_mpim: false,
            user: None,
            member_count: Some(42),
            purpose: Some("General discussion".into()),
            topic: None,
        };
        
        let json = serde_json::to_string(&channel).unwrap();
        let parsed: SlackChannel = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.id, "C123");
        assert_eq!(parsed.name, "general");
        assert!(!parsed.is_private);
        assert_eq!(parsed.member_count, Some(42));
    }

    #[test]
    fn test_slack_dm_channel_with_user() {
        let channel = SlackChannel {
            id: "D123".into(),
            name: "".into(),
            is_private: false,
            is_im: true,
            is_mpim: false,
            user: Some("U456".into()),
            member_count: None,
            purpose: None,
            topic: None,
        };
        
        let json = serde_json::to_string(&channel).unwrap();
        let parsed: SlackChannel = serde_json::from_str(&json).unwrap();
        assert!(parsed.is_im);
        assert_eq!(parsed.user, Some("U456".into()));
    }

    #[test]
    fn test_slack_message_with_thread() {
        let msg = SlackMessage {
            ts: "1234567890.123456".into(),
            user: Some("U123".into()),
            text: "Hello world".into(),
            thread_ts: Some("1234567890.000000".into()),
            reply_count: Some(5),
        };
        
        let json = serde_json::to_string(&msg).unwrap();
        let parsed: SlackMessage = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.ts, "1234567890.123456");
        assert_eq!(parsed.thread_ts, Some("1234567890.000000".into()));
        assert_eq!(parsed.reply_count, Some(5));
    }

    #[test]
    fn test_slack_message_without_thread() {
        let msg = SlackMessage {
            ts: "1234567890.123456".into(),
            user: None,
            text: "Bot message".into(),
            thread_ts: None,
            reply_count: None,
        };
        
        let json = serde_json::to_string(&msg).unwrap();
        let parsed: SlackMessage = serde_json::from_str(&json).unwrap();
        assert!(parsed.user.is_none());
        assert!(parsed.thread_ts.is_none());
    }

    #[test]
    fn test_sync_result_serialization() {
        let result = SyncResult {
            source: "slack".into(),
            items_synced: 42,
            errors: vec!["Error 1".into()],
        };
        
        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("\"source\":\"slack\""));
        assert!(json.contains("\"items_synced\":42"));
    }

    #[test]
    fn test_slack_error_display() {
        let err = SlackError::OAuth("Invalid token".into());
        assert_eq!(err.to_string(), "OAuth error: Invalid token");
        
        let err = SlackError::Api("rate_limited".into());
        assert_eq!(err.to_string(), "API error: rate_limited");
    }

    #[test]
    fn test_slack_channel_selection_serialization() {
        let selection = SlackChannelSelection {
            channel_id: "C123".into(),
            channel_name: "general".into(),
            is_private: false,
            is_im: false,
            is_mpim: false,
            team_id: "T123".into(),
            member_count: Some(50),
            purpose: Some("General chat".into()),
            enabled: true,
        };
        
        let json = serde_json::to_string(&selection).unwrap();
        assert!(json.contains("channelId")); // camelCase
        assert!(json.contains("C123"));
        
        let parsed: SlackChannelSelection = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.channel_id, "C123");
        assert_eq!(parsed.team_id, "T123");
        assert!(parsed.enabled);
    }

    #[test]
    fn test_slack_connection_status_serialization() {
        let status = SlackConnectionStatus {
            connected: true,
            team_id: Some("T123".into()),
            team_name: Some("Test Workspace".into()),
            user_id: Some("U456".into()),
            selected_channel_count: 5,
        };
        
        let json = serde_json::to_string(&status).unwrap();
        assert!(json.contains("teamId")); // camelCase
        assert!(json.contains("selectedChannelCount"));
        
        let parsed: SlackConnectionStatus = serde_json::from_str(&json).unwrap();
        assert!(parsed.connected);
        assert_eq!(parsed.team_id, Some("T123".into()));
        assert_eq!(parsed.selected_channel_count, 5);
    }

    #[test]
    fn test_slack_connection_status_disconnected() {
        let status = SlackConnectionStatus {
            connected: false,
            team_id: None,
            team_name: None,
            user_id: None,
            selected_channel_count: 0,
        };
        
        let json = serde_json::to_string(&status).unwrap();
        let parsed: SlackConnectionStatus = serde_json::from_str(&json).unwrap();
        assert!(!parsed.connected);
        assert!(parsed.team_id.is_none());
    }

    #[test]
    fn test_slack_user_serialization() {
        let user = SlackUser {
            id: "U123".into(),
            name: "johndoe".into(),
            real_name: Some("John Doe".into()),
            display_name: Some("John".into()),
        };
        
        let json = serde_json::to_string(&user).unwrap();
        assert!(json.contains("realName")); // camelCase
        
        let parsed: SlackUser = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.id, "U123");
        assert_eq!(parsed.name, "johndoe");
        assert_eq!(parsed.real_name, Some("John Doe".into()));
    }

    #[test]
    fn test_slack_user_minimal() {
        let user = SlackUser {
            id: "U123".into(),
            name: "bot".into(),
            real_name: None,
            display_name: None,
        };
        
        let json = serde_json::to_string(&user).unwrap();
        let parsed: SlackUser = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.id, "U123");
        assert!(parsed.real_name.is_none());
    }
}
