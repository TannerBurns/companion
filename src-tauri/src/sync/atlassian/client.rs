//! Atlassian API client with OAuth 2.0 + PKCE support

use reqwest::Client;
use sha2::{Sha256, Digest};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use rand::RngCore;
use serde::Deserialize;

use super::types::{AtlassianError, AtlassianTokens, CloudResource, JiraIssue, ConfluencePage};
use crate::sync::oauth::spawn_oauth_callback_listener_ready;

const ATLASSIAN_AUTHORIZE_URL: &str = "https://auth.atlassian.com/authorize";
const ATLASSIAN_TOKEN_URL: &str = "https://auth.atlassian.com/oauth/token";
const ATLASSIAN_RESOURCES_URL: &str = "https://api.atlassian.com/oauth/token/accessible-resources";
const REDIRECT_PORT: u16 = 8375;

pub struct AtlassianClient {
    http: Client,
    client_id: String,
    client_secret: String,
    access_token: Option<String>,
    cloud_id: Option<String>,
}

impl AtlassianClient {
    pub fn new(client_id: String, client_secret: String) -> Self {
        Self {
            http: Client::new(),
            client_id,
            client_secret,
            access_token: None,
            cloud_id: None,
        }
    }
    
    pub fn with_token(mut self, access_token: String, cloud_id: String) -> Self {
        self.access_token = Some(access_token);
        self.cloud_id = Some(cloud_id);
        self
    }
    
    /// Generate PKCE code verifier and challenge
    fn generate_pkce() -> (String, String) {
        let mut verifier_bytes = [0u8; 32];
        rand::rngs::OsRng.fill_bytes(&mut verifier_bytes);
        let verifier = URL_SAFE_NO_PAD.encode(verifier_bytes);
        
        let mut hasher = Sha256::new();
        hasher.update(verifier.as_bytes());
        let challenge = URL_SAFE_NO_PAD.encode(hasher.finalize());
        
        (verifier, challenge)
    }
    
    /// Generate OAuth authorization URL with PKCE
    pub fn get_auth_url(&self, state: &str, code_challenge: &str) -> String {
        let scopes = [
            "read:jira-work",
            "read:jira-user", 
            "read:confluence-content.all",
            "read:confluence-space.summary",
            "offline_access",
        ].join(" ");
        
        format!(
            "{}?audience=api.atlassian.com&client_id={}&scope={}&redirect_uri=http://localhost:{}/atlassian/callback&state={}&response_type=code&prompt=consent&code_challenge={}&code_challenge_method=S256",
            ATLASSIAN_AUTHORIZE_URL,
            self.client_id,
            urlencoding::encode(&scopes),
            REDIRECT_PORT,
            state,
            code_challenge,
        )
    }
    
    pub async fn start_oauth_flow(&self) -> Result<(AtlassianTokens, Vec<CloudResource>), AtlassianError> {
        let state = uuid::Uuid::new_v4().to_string();
        let (code_verifier, code_challenge) = Self::generate_pkce();
        let auth_url = self.get_auth_url(&state, &code_challenge);
        
        // Bind the callback listener BEFORE opening the browser to avoid race conditions
        let rx = spawn_oauth_callback_listener_ready(REDIRECT_PORT, state).await
            .map_err(|e| AtlassianError::OAuth(format!("Failed to start callback listener: {}", e)))?;
        
        open::that(&auth_url).map_err(|e| AtlassianError::OAuth(e.to_string()))?;
        
        let code = rx.await
            .map_err(|_| AtlassianError::OAuth("Callback cancelled".into()))?
            .map_err(|e| AtlassianError::OAuth(e.to_string()))?;
        
        let tokens = self.exchange_code(&code, &code_verifier).await?;
        let resources = self.get_accessible_resources(&tokens.access_token).await?;
        
        Ok((tokens, resources))
    }
    
    async fn exchange_code(&self, code: &str, code_verifier: &str) -> Result<AtlassianTokens, AtlassianError> {
        #[derive(Deserialize)]
        struct TokenResponse {
            access_token: String,
            refresh_token: Option<String>,
            expires_in: i64,
            scope: String,
        }
        
        let response = self.http
            .post(ATLASSIAN_TOKEN_URL)
            .json(&serde_json::json!({
                "grant_type": "authorization_code",
                "client_id": self.client_id,
                "client_secret": self.client_secret,
                "code": code,
                "redirect_uri": format!("http://localhost:{}/atlassian/callback", REDIRECT_PORT),
                "code_verifier": code_verifier,
            }))
            .send()
            .await?;
        
        if !response.status().is_success() {
            return Err(AtlassianError::OAuth(format!(
                "Token exchange failed with status: {}",
                response.status()
            )));
        }
        
        let token_response: TokenResponse = response.json().await?;
        
        Ok(AtlassianTokens {
            access_token: token_response.access_token,
            refresh_token: token_response.refresh_token,
            expires_in: token_response.expires_in,
            scope: token_response.scope,
        })
    }
    
    async fn get_accessible_resources(&self, access_token: &str) -> Result<Vec<CloudResource>, AtlassianError> {
        #[derive(Deserialize)]
        struct ResourceResponse {
            id: String,
            name: String,
            url: String,
            scopes: Vec<String>,
        }
        
        let response = self.http
            .get(ATLASSIAN_RESOURCES_URL)
            .bearer_auth(access_token)
            .send()
            .await?;
        
        if !response.status().is_success() {
            return Err(AtlassianError::Api(format!("HTTP {}", response.status())));
        }
        
        let resources: Vec<ResourceResponse> = response.json().await?;
        
        Ok(resources.into_iter().map(|r| CloudResource {
            id: r.id,
            name: r.name,
            url: r.url,
            scopes: r.scopes,
        }).collect())
    }
    
    /// Search Jira issues using JQL
    pub async fn search_issues(&self, jql: &str, start_at: i32, max_results: i32) -> Result<Vec<JiraIssue>, AtlassianError> {
        let token = self.access_token.as_ref()
            .ok_or_else(|| AtlassianError::OAuth("Not authenticated".into()))?;
        let cloud_id = self.cloud_id.as_ref()
            .ok_or_else(|| AtlassianError::OAuth("No cloud instance selected".into()))?;
        
        let url = format!(
            "https://api.atlassian.com/ex/jira/{}/rest/api/3/search",
            cloud_id
        );
        
        let start_at_str = start_at.to_string();
        let max_results_str = max_results.to_string();
        
        let response = self.http
            .get(&url)
            .bearer_auth(token)
            .query(&[
                ("jql", jql),
                ("startAt", &start_at_str),
                ("maxResults", &max_results_str),
                ("fields", "summary,description,status,assignee,reporter,project,created,updated"),
            ])
            .send()
            .await?;
        
        if !response.status().is_success() {
            return Err(AtlassianError::Api(format!("HTTP {}", response.status())));
        }
        
        let json: serde_json::Value = response.json().await?;
        
        let issues = json["issues"]
            .as_array()
            .map(|issues| {
                issues.iter().filter_map(|i| {
                    let fields = &i["fields"];
                    Some(JiraIssue {
                        id: i["id"].as_str()?.to_string(),
                        key: i["key"].as_str()?.to_string(),
                        summary: fields["summary"].as_str().unwrap_or_default().to_string(),
                        description: fields["description"]["content"][0]["content"][0]["text"]
                            .as_str()
                            .map(String::from),
                        status: fields["status"]["name"].as_str().unwrap_or_default().to_string(),
                        assignee: fields["assignee"]["displayName"].as_str().map(String::from),
                        reporter: fields["reporter"]["displayName"].as_str().unwrap_or_default().to_string(),
                        project_key: fields["project"]["key"].as_str().unwrap_or_default().to_string(),
                        created: fields["created"].as_str().unwrap_or_default().to_string(),
                        updated: fields["updated"].as_str().unwrap_or_default().to_string(),
                        url: format!(
                            "https://{}.atlassian.net/browse/{}",
                            cloud_id,
                            i["key"].as_str().unwrap_or_default()
                        ),
                    })
                }).collect()
            })
            .unwrap_or_default();
        
        Ok(issues)
    }
    
    /// Search Confluence pages using CQL
    pub async fn search_pages(&self, cql: &str, start: i32, limit: i32) -> Result<Vec<ConfluencePage>, AtlassianError> {
        let token = self.access_token.as_ref()
            .ok_or_else(|| AtlassianError::OAuth("Not authenticated".into()))?;
        let cloud_id = self.cloud_id.as_ref()
            .ok_or_else(|| AtlassianError::OAuth("No cloud instance selected".into()))?;
        
        let url = format!(
            "https://api.atlassian.com/ex/confluence/{}/wiki/rest/api/content/search",
            cloud_id
        );
        
        let start_str = start.to_string();
        let limit_str = limit.to_string();
        
        let response = self.http
            .get(&url)
            .bearer_auth(token)
            .query(&[
                ("cql", cql),
                ("start", &start_str),
                ("limit", &limit_str),
                ("expand", "body.storage,space,version"),
            ])
            .send()
            .await?;
        
        if !response.status().is_success() {
            return Err(AtlassianError::Api(format!("HTTP {}", response.status())));
        }
        
        let json: serde_json::Value = response.json().await?;
        
        let pages = json["results"]
            .as_array()
            .map(|pages| {
                pages.iter().filter_map(|p| {
                    Some(ConfluencePage {
                        id: p["id"].as_str()?.to_string(),
                        title: p["title"].as_str().unwrap_or_default().to_string(),
                        space_key: p["space"]["key"].as_str().unwrap_or_default().to_string(),
                        body: p["body"]["storage"]["value"].as_str().map(String::from),
                        author: p["version"]["by"]["displayName"].as_str().unwrap_or_default().to_string(),
                        created: p["version"]["when"].as_str().unwrap_or_default().to_string(),
                        updated: p["version"]["when"].as_str().unwrap_or_default().to_string(),
                        url: format!(
                            "https://{}.atlassian.net/wiki{}",
                            cloud_id,
                            p["_links"]["webui"].as_str().unwrap_or_default()
                        ),
                    })
                }).collect()
            })
            .unwrap_or_default();
        
        Ok(pages)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_client() {
        let client = AtlassianClient::new("client_id".into(), "secret".into());
        assert!(client.access_token.is_none());
        assert!(client.cloud_id.is_none());
    }

    #[test]
    fn test_with_token() {
        let client = AtlassianClient::new("client_id".into(), "secret".into())
            .with_token("access-token".into(), "cloud-123".into());
        assert_eq!(client.access_token, Some("access-token".into()));
        assert_eq!(client.cloud_id, Some("cloud-123".into()));
    }

    #[test]
    fn test_generate_pkce_format() {
        let (verifier, challenge) = AtlassianClient::generate_pkce();
        
        // Verifier should be base64url encoded 32 bytes = 43 chars
        assert!(verifier.len() >= 43);
        assert!(!verifier.contains('+'));
        assert!(!verifier.contains('/'));
        
        // Challenge should be SHA256 of verifier, base64url encoded = 43 chars
        assert!(challenge.len() >= 43);
        assert!(!challenge.contains('+'));
        assert!(!challenge.contains('/'));
        
        // Verifier and challenge should be different
        assert_ne!(verifier, challenge);
    }

    #[test]
    fn test_pkce_is_unique() {
        let (v1, c1) = AtlassianClient::generate_pkce();
        let (v2, c2) = AtlassianClient::generate_pkce();
        
        assert_ne!(v1, v2);
        assert_ne!(c1, c2);
    }

    #[test]
    fn test_get_auth_url_contains_required_params() {
        let client = AtlassianClient::new("test-client-id".into(), "secret".into());
        let url = client.get_auth_url("random-state", "challenge123");
        
        assert!(url.starts_with("https://auth.atlassian.com/authorize"));
        assert!(url.contains("client_id=test-client-id"));
        assert!(url.contains("state=random-state"));
        assert!(url.contains("code_challenge=challenge123"));
        assert!(url.contains("code_challenge_method=S256"));
        assert!(url.contains("redirect_uri=http://localhost:8375/atlassian/callback"));
        assert!(url.contains("response_type=code"));
        assert!(url.contains("offline_access"));
    }

    #[tokio::test]
    async fn test_search_issues_requires_auth() {
        let client = AtlassianClient::new("id".into(), "secret".into());
        let result = client.search_issues("project = TEST", 0, 50).await;
        
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, AtlassianError::OAuth(_)));
    }

    #[tokio::test]
    async fn test_search_issues_requires_cloud_id() {
        let client = AtlassianClient::new("id".into(), "secret".into());
        // Has token but no cloud_id
        let client = AtlassianClient {
            access_token: Some("token".into()),
            cloud_id: None,
            ..client
        };
        let result = client.search_issues("project = TEST", 0, 50).await;
        
        assert!(result.is_err());
        let err = result.unwrap_err();
        match err {
            AtlassianError::OAuth(msg) => assert!(msg.contains("cloud")),
            _ => panic!("Expected OAuth error about cloud_id"),
        }
    }

    #[tokio::test]
    async fn test_search_pages_requires_auth() {
        let client = AtlassianClient::new("id".into(), "secret".into());
        let result = client.search_pages("type = page", 0, 25).await;
        
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, AtlassianError::OAuth(_)));
    }
}
