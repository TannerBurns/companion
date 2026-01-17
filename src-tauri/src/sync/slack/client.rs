//! Slack API client with OAuth support

use reqwest::Client;
use super::types::{SlackError, SlackTokens, SlackChannel, SlackMessage, OAuthResponse};
use crate::sync::oauth::spawn_oauth_callback_listener;

const SLACK_AUTHORIZE_URL: &str = "https://slack.com/oauth/v2/authorize";
const SLACK_TOKEN_URL: &str = "https://slack.com/api/oauth.v2.access";
const SLACK_API_BASE: &str = "https://slack.com/api";
const REDIRECT_PORT: u16 = 8374;

pub struct SlackClient {
    http: Client,
    client_id: String,
    client_secret: String,
    access_token: Option<String>,
}

impl SlackClient {
    pub fn new(client_id: String, client_secret: String) -> Self {
        Self {
            http: Client::new(),
            client_id,
            client_secret,
            access_token: None,
        }
    }
    
    pub fn with_token(mut self, access_token: String) -> Self {
        self.access_token = Some(access_token);
        self
    }
    
    /// Generate OAuth authorization URL
    pub fn get_auth_url(&self, state: &str) -> String {
        let scopes = [
            "channels:history",
            "channels:read", 
            "groups:history",
            "groups:read",
            "im:history",
            "im:read",
            "mpim:history",
            "mpim:read",
            "users:read",
            "search:read",
        ].join(",");
        
        format!(
            "{}?client_id={}&scope={}&redirect_uri=http://localhost:{}/slack/callback&state={}&user_scope={}",
            SLACK_AUTHORIZE_URL,
            self.client_id,
            "", // Bot scopes (empty for user-only)
            REDIRECT_PORT,
            state,
            scopes,
        )
    }
    
    pub async fn start_oauth_flow(&self) -> Result<SlackTokens, SlackError> {
        let state = uuid::Uuid::new_v4().to_string();
        let auth_url = self.get_auth_url(&state);
        
        open::that(&auth_url).map_err(|e| SlackError::OAuth(e.to_string()))?;
        let rx = spawn_oauth_callback_listener(REDIRECT_PORT, state);
        
        let code = rx.await
            .map_err(|_| SlackError::OAuth("Callback cancelled".into()))?
            .map_err(|e| SlackError::OAuth(e.to_string()))?;
        
        self.exchange_code(&code).await
    }
    
    /// Exchange authorization code for tokens
    async fn exchange_code(&self, code: &str) -> Result<SlackTokens, SlackError> {
        let response = self.http
            .post(SLACK_TOKEN_URL)
            .form(&[
                ("client_id", self.client_id.as_str()),
                ("client_secret", self.client_secret.as_str()),
                ("code", code),
                ("redirect_uri", &format!("http://localhost:{}/slack/callback", REDIRECT_PORT)),
            ])
            .send()
            .await?;
        
        if !response.status().is_success() {
            return Err(SlackError::OAuth(format!(
                "Token exchange failed with status: {}",
                response.status()
            )));
        }
        
        let oauth_response: OAuthResponse = response.json().await?;
        
        if !oauth_response.ok {
            return Err(SlackError::OAuth(oauth_response.error.unwrap_or_default()));
        }
        
        let user = oauth_response.authed_user
            .ok_or_else(|| SlackError::OAuth("No user token".into()))?;
        let team = oauth_response.team
            .ok_or_else(|| SlackError::OAuth("No team info".into()))?;
        
        Ok(SlackTokens {
            access_token: user.access_token.ok_or_else(|| SlackError::OAuth("No access token".into()))?,
            token_type: user.token_type.unwrap_or_else(|| "bearer".into()),
            scope: user.scope.unwrap_or_default(),
            team_id: team.id,
            team_name: team.name,
            user_id: user.id,
        })
    }
    
    /// Fetch list of channels
    pub async fn list_channels(&self) -> Result<Vec<SlackChannel>, SlackError> {
        let token = self.access_token.as_ref()
            .ok_or_else(|| SlackError::OAuth("Not authenticated".into()))?;
        
        let mut all_channels = Vec::new();
        let mut cursor: Option<String> = None;
        
        loop {
            let mut params = vec![
                ("types", "public_channel,private_channel,mpim,im"),
                ("limit", "200"),
            ];
            
            let cursor_str;
            if let Some(ref c) = cursor {
                cursor_str = c.clone();
                params.push(("cursor", &cursor_str));
            }
            
            let response = self.http
                .get(format!("{}/conversations.list", SLACK_API_BASE))
                .bearer_auth(token)
                .query(&params)
                .send()
                .await?;
            
            if !response.status().is_success() {
                return Err(SlackError::Api(format!("HTTP {}", response.status())));
            }
            
            let json: serde_json::Value = response.json().await?;
            
            if !json["ok"].as_bool().unwrap_or(false) {
                return Err(SlackError::Api(
                    json["error"].as_str().unwrap_or("Unknown error").to_string()
                ));
            }
            
            if let Some(channels) = json["channels"].as_array() {
                for ch in channels {
                    all_channels.push(SlackChannel {
                        id: ch["id"].as_str().unwrap_or_default().to_string(),
                        name: ch["name"].as_str().unwrap_or_default().to_string(),
                        is_private: ch["is_private"].as_bool().unwrap_or(false),
                        is_im: ch["is_im"].as_bool().unwrap_or(false),
                        is_mpim: ch["is_mpim"].as_bool().unwrap_or(false),
                    });
                }
            }
            
            cursor = json["response_metadata"]["next_cursor"]
                .as_str()
                .filter(|c| !c.is_empty())
                .map(String::from);
            
            if cursor.is_none() {
                break;
            }
        }
        
        Ok(all_channels)
    }
    
    /// Fetch messages from a channel
    pub async fn get_channel_history(
        &self,
        channel_id: &str,
        oldest: Option<&str>,
        limit: usize,
    ) -> Result<Vec<SlackMessage>, SlackError> {
        let token = self.access_token.as_ref()
            .ok_or_else(|| SlackError::OAuth("Not authenticated".into()))?;
        
        let limit_str = limit.to_string();
        let mut params = vec![
            ("channel", channel_id),
            ("limit", &limit_str),
        ];
        
        if let Some(ts) = oldest {
            params.push(("oldest", ts));
        }
        
        let response = self.http
            .get(format!("{}/conversations.history", SLACK_API_BASE))
            .bearer_auth(token)
            .query(&params)
            .send()
            .await?;
        
        if !response.status().is_success() {
            return Err(SlackError::Api(format!("HTTP {}", response.status())));
        }
        
        let json: serde_json::Value = response.json().await?;
        
        if !json["ok"].as_bool().unwrap_or(false) {
            return Err(SlackError::Api(
                json["error"].as_str().unwrap_or("Unknown error").to_string()
            ));
        }
        
        let messages = json["messages"]
            .as_array()
            .map(|msgs| {
                msgs.iter()
                    .map(|m| SlackMessage {
                        ts: m["ts"].as_str().unwrap_or_default().to_string(),
                        user: m["user"].as_str().map(String::from),
                        text: m["text"].as_str().unwrap_or_default().to_string(),
                        thread_ts: m["thread_ts"].as_str().map(String::from),
                        reply_count: m["reply_count"].as_i64().map(|n| n as i32),
                    })
                    .collect()
            })
            .unwrap_or_default();
        
        Ok(messages)
    }
    
    /// Fetch thread replies
    pub async fn get_thread_replies(
        &self,
        channel_id: &str,
        thread_ts: &str,
    ) -> Result<Vec<SlackMessage>, SlackError> {
        let token = self.access_token.as_ref()
            .ok_or_else(|| SlackError::OAuth("Not authenticated".into()))?;
        
        let response = self.http
            .get(format!("{}/conversations.replies", SLACK_API_BASE))
            .bearer_auth(token)
            .query(&[
                ("channel", channel_id),
                ("ts", thread_ts),
            ])
            .send()
            .await?;
        
        if !response.status().is_success() {
            return Err(SlackError::Api(format!("HTTP {}", response.status())));
        }
        
        let json: serde_json::Value = response.json().await?;
        
        if !json["ok"].as_bool().unwrap_or(false) {
            return Err(SlackError::Api(
                json["error"].as_str().unwrap_or("Unknown error").to_string()
            ));
        }
        
        let messages = json["messages"]
            .as_array()
            .map(|msgs| {
                msgs.iter()
                    .map(|m| SlackMessage {
                        ts: m["ts"].as_str().unwrap_or_default().to_string(),
                        user: m["user"].as_str().map(String::from),
                        text: m["text"].as_str().unwrap_or_default().to_string(),
                        thread_ts: m["thread_ts"].as_str().map(String::from),
                        reply_count: m["reply_count"].as_i64().map(|n| n as i32),
                    })
                    .collect()
            })
            .unwrap_or_default();
        
        Ok(messages)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_client() {
        let client = SlackClient::new("client_id".into(), "secret".into());
        assert!(client.access_token.is_none());
    }

    #[test]
    fn test_with_token() {
        let client = SlackClient::new("client_id".into(), "secret".into())
            .with_token("xoxp-token".into());
        assert_eq!(client.access_token, Some("xoxp-token".into()));
    }

    #[test]
    fn test_get_auth_url_contains_required_params() {
        let client = SlackClient::new("test-client-id".into(), "secret".into());
        let url = client.get_auth_url("random-state-123");
        
        assert!(url.starts_with("https://slack.com/oauth/v2/authorize"));
        assert!(url.contains("client_id=test-client-id"));
        assert!(url.contains("state=random-state-123"));
        assert!(url.contains("redirect_uri=http://localhost:8374/slack/callback"));
        assert!(url.contains("user_scope="));
        assert!(url.contains("channels:read"));
        assert!(url.contains("channels:history"));
    }

    #[tokio::test]
    async fn test_list_channels_requires_auth() {
        let client = SlackClient::new("id".into(), "secret".into());
        let result = client.list_channels().await;
        
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, SlackError::OAuth(_)));
    }

    #[tokio::test]
    async fn test_get_channel_history_requires_auth() {
        let client = SlackClient::new("id".into(), "secret".into());
        let result = client.get_channel_history("C123", None, 100).await;
        
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, SlackError::OAuth(_)));
    }

    #[tokio::test]
    async fn test_get_thread_replies_requires_auth() {
        let client = SlackClient::new("id".into(), "secret".into());
        let result = client.get_thread_replies("C123", "1234567890.123456").await;
        
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, SlackError::OAuth(_)));
    }
}
