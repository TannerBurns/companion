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
pub struct SlackTokens {
    pub access_token: String,
    pub token_type: String,
    pub scope: String,
    pub team_id: String,
    pub team_name: String,
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
pub struct SlackChannel {
    pub id: String,
    pub name: String,
    pub is_private: bool,
    pub is_im: bool,
    pub is_mpim: bool,
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
        };
        
        let json = serde_json::to_string(&channel).unwrap();
        let parsed: SlackChannel = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.id, "C123");
        assert_eq!(parsed.name, "general");
        assert!(!parsed.is_private);
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
}
