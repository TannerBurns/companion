use super::auth::{get_access_token, CachedToken, GeminiAuth, ServiceAccountCredentials};
use super::types::{
    Content, GeminiError, GenerateRequest, GenerateResponse, GenerationConfig, Part, GEMINI_API_URL,
};
use chrono::{Duration, Utc};
use reqwest::Client;
use serde::Deserialize;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Client for interacting with the Gemini API.
pub struct GeminiClient {
    http: Client,
    auth: GeminiAuth,
    model: String,
    /// Cached OAuth2 access token (only used for service account auth)
    token_cache: Arc<RwLock<Option<CachedToken>>>,
}

impl GeminiClient {
    /// Create a new client with API key authentication.
    pub fn new(api_key: String) -> Self {
        Self {
            http: Client::new(),
            auth: GeminiAuth::ApiKey(api_key),
            model: "gemini-3-pro-preview".to_string(),
            token_cache: Arc::new(RwLock::new(None)),
        }
    }

    /// Create a new client with service account authentication.
    pub fn new_with_service_account(credentials: ServiceAccountCredentials) -> Self {
        Self {
            http: Client::new(),
            auth: GeminiAuth::ServiceAccount(Box::new(credentials)),
            model: "gemini-3-pro-preview".to_string(),
            token_cache: Arc::new(RwLock::new(None)),
        }
    }

    /// Set the model to use for generation.
    pub fn with_model(mut self, model: &str) -> Self {
        self.model = model.to_string();
        self
    }

    /// Get an access token, using cache if available.
    async fn get_cached_access_token(
        &self,
        credentials: &ServiceAccountCredentials,
    ) -> Result<String, GeminiError> {
        // Check cache first
        {
            let cache = self.token_cache.read().await;
            if let Some(cached) = cache.as_ref() {
                // Refresh 5 minutes before expiry
                if cached.expires_at > Utc::now() + Duration::minutes(5) {
                    tracing::debug!("Using cached access token");
                    return Ok(cached.access_token.clone());
                }
            }
        }

        // Get new token
        let token = get_access_token(&self.http, credentials).await?;
        let access_token = token.access_token.clone();

        // Cache it
        {
            let mut cache = self.token_cache.write().await;
            *cache = Some(token);
        }

        Ok(access_token)
    }

    /// Generate content using the Gemini API.
    pub async fn generate(
        &self,
        request: GenerateRequest,
    ) -> Result<GenerateResponse, GeminiError> {
        let (url, req_builder) = match &self.auth {
            GeminiAuth::ApiKey(key) => {
                let url = format!("{}/{}:generateContent", GEMINI_API_URL, self.model);
                tracing::debug!("Using API key authentication with URL: {}", url);
                let req = self.http.post(&url).json(&request).query(&[("key", key)]);
                (url, req)
            }
            GeminiAuth::ServiceAccount(credentials) => {
                let region = credentials.region();

                let url = if region == "global" {
                    format!(
                        "https://aiplatform.googleapis.com/v1/projects/{}/locations/{}/publishers/google/models/{}:generateContent",
                        credentials.project_id, region, self.model
                    )
                } else {
                    format!(
                        "https://{}-aiplatform.googleapis.com/v1/projects/{}/locations/{}/publishers/google/models/{}:generateContent",
                        region, credentials.project_id, region, self.model
                    )
                };

                tracing::debug!("Vertex AI request to: {}", url);

                let token = self.get_cached_access_token(credentials).await?;
                let req = self.http.post(&url).json(&request).bearer_auth(token);
                (url, req)
            }
        };

        tracing::debug!("Making Gemini API request to: {}", url);

        let response = req_builder.send().await?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await?;
            tracing::error!("Gemini API error ({}): {}", status, error_text);

            let error_msg = if let Ok(json) = serde_json::from_str::<serde_json::Value>(&error_text)
            {
                if let Some(message) = json
                    .get("error")
                    .and_then(|e| e.get("message"))
                    .and_then(|m| m.as_str())
                {
                    message.to_string()
                } else {
                    error_text
                }
            } else {
                error_text
            };

            return Err(GeminiError::Api(format!(
                "HTTP {}: {}",
                status.as_u16(),
                error_msg
            )));
        }

        Ok(response.json().await?)
    }

    /// Verify the connection by making a simple API call.
    pub async fn verify_connection(&self) -> Result<(), GeminiError> {
        tracing::info!("Verifying Gemini connection...");

        let request = GenerateRequest {
            contents: vec![Content {
                role: "user".to_string(),
                parts: vec![Part::Text {
                    text: "Hello".to_string(),
                }],
            }],
            tools: None,
            generation_config: Some(GenerationConfig {
                temperature: Some(0.0),
                max_output_tokens: Some(10),
                response_mime_type: None,
            }),
        };

        match self.generate(request).await {
            Ok(_) => {
                tracing::info!("Gemini connection verified successfully");
                Ok(())
            }
            Err(e) => {
                tracing::error!("Gemini connection verification failed: {}", e);
                Err(e)
            }
        }
    }

    /// Generate text from a prompt.
    pub async fn generate_text(&self, prompt: &str) -> Result<String, GeminiError> {
        let request = GenerateRequest {
            contents: vec![Content {
                role: "user".to_string(),
                parts: vec![Part::Text {
                    text: prompt.to_string(),
                }],
            }],
            tools: None,
            generation_config: Some(GenerationConfig {
                temperature: Some(0.7),
                max_output_tokens: None,
                response_mime_type: None,
            }),
        };

        let response = self.generate(request).await?;

        response
            .candidates
            .first()
            .and_then(|c| c.content.parts.first())
            .and_then(|p| match p {
                Part::Text { text } => Some(text.clone()),
                _ => None,
            })
            .ok_or_else(|| GeminiError::Parse("No text in response".into()))
    }

    /// Generate JSON output from a prompt.
    pub async fn generate_json<T: for<'de> Deserialize<'de>>(
        &self,
        prompt: &str,
    ) -> Result<T, GeminiError> {
        let request = GenerateRequest {
            contents: vec![Content {
                role: "user".to_string(),
                parts: vec![Part::Text {
                    text: prompt.to_string(),
                }],
            }],
            tools: None,
            generation_config: Some(GenerationConfig {
                temperature: Some(0.3),
                max_output_tokens: None,
                response_mime_type: Some("application/json".to_string()),
            }),
        };

        let response = self.generate(request).await?;

        let text = response
            .candidates
            .first()
            .and_then(|c| c.content.parts.first())
            .and_then(|p| match p {
                Part::Text { text } => Some(text.clone()),
                _ => None,
            })
            .ok_or_else(|| GeminiError::Parse("No text in response".into()))?;

        serde_json::from_str(&text).map_err(|e| GeminiError::Parse(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gemini_client_new() {
        let client = GeminiClient::new("test-api-key".into());
        match &client.auth {
            GeminiAuth::ApiKey(key) => assert_eq!(key, "test-api-key"),
            _ => panic!("Expected ApiKey auth"),
        }
        assert_eq!(client.model, "gemini-3-pro-preview");
    }

    #[test]
    fn test_gemini_client_with_model() {
        let client = GeminiClient::new("key".into()).with_model("gemini-1.5-pro");
        assert_eq!(client.model, "gemini-1.5-pro");
    }

    #[test]
    fn test_gemini_client_with_service_account() {
        let credentials = ServiceAccountCredentials {
            account_type: Some("service_account".to_string()),
            project_id: "test-project".to_string(),
            private_key_id: None,
            private_key: "-----BEGIN RSA PRIVATE KEY-----\ntest\n-----END RSA PRIVATE KEY-----"
                .to_string(),
            client_email: "test@test.iam.gserviceaccount.com".to_string(),
            client_id: None,
            auth_uri: None,
            token_uri: "https://oauth2.googleapis.com/token".to_string(),
            auth_provider_x509_cert_url: None,
            client_x509_cert_url: None,
            vertex_region: None,
        };
        let client = GeminiClient::new_with_service_account(credentials);
        match &client.auth {
            GeminiAuth::ServiceAccount(creds) => {
                assert_eq!(creds.project_id, "test-project");
                assert_eq!(creds.client_email, "test@test.iam.gserviceaccount.com");
                assert_eq!(creds.region(), "global");
            }
            _ => panic!("Expected ServiceAccount auth"),
        }
    }

    #[test]
    fn test_gemini_client_with_custom_region() {
        let credentials = ServiceAccountCredentials {
            account_type: Some("service_account".to_string()),
            project_id: "test-project".to_string(),
            private_key_id: None,
            private_key: "-----BEGIN RSA PRIVATE KEY-----\ntest\n-----END RSA PRIVATE KEY-----"
                .to_string(),
            client_email: "test@test.iam.gserviceaccount.com".to_string(),
            client_id: None,
            auth_uri: None,
            token_uri: "https://oauth2.googleapis.com/token".to_string(),
            auth_provider_x509_cert_url: None,
            client_x509_cert_url: None,
            vertex_region: Some("europe-west1".to_string()),
        };
        let client = GeminiClient::new_with_service_account(credentials);
        match &client.auth {
            GeminiAuth::ServiceAccount(creds) => {
                assert_eq!(creds.region(), "europe-west1");
            }
            _ => panic!("Expected ServiceAccount auth"),
        }
    }
}
