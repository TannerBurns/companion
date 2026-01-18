use reqwest::Client;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use std::sync::Arc;
use tokio::sync::RwLock;
use chrono::{DateTime, Utc, Duration};
use jsonwebtoken::{encode, EncodingKey, Header, Algorithm};

const GEMINI_API_URL: &str = "https://generativelanguage.googleapis.com/v1beta/models";

/// Default region for Vertex AI. Users can override this.
/// "global" uses the non-regional endpoint (aiplatform.googleapis.com)
/// Specific regions use regional endpoints ({region}-aiplatform.googleapis.com)
const DEFAULT_VERTEX_REGION: &str = "global";

#[derive(Error, Debug)]
pub enum GeminiError {
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("API error: {0}")]
    Api(String),
    #[error("Parse error: {0}")]
    Parse(String),
    #[error("JWT error: {0}")]
    Jwt(String),
    #[error("Auth error: {0}")]
    Auth(String),
}

/// Google Cloud Service Account credentials parsed from JSON file
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ServiceAccountCredentials {
    #[serde(rename = "type")]
    pub account_type: Option<String>,
    pub project_id: String,
    pub private_key_id: Option<String>,
    pub private_key: String,
    pub client_email: String,
    pub client_id: Option<String>,
    pub auth_uri: Option<String>,
    pub token_uri: String,
    pub auth_provider_x509_cert_url: Option<String>,
    pub client_x509_cert_url: Option<String>,
    /// Vertex AI region (e.g., "us-central1", "europe-west1")
    #[serde(default)]
    pub vertex_region: Option<String>,
}

impl ServiceAccountCredentials {
    pub fn region(&self) -> &str {
        self.vertex_region.as_deref().unwrap_or(DEFAULT_VERTEX_REGION)
    }
}

/// Authentication method for Gemini API
#[derive(Debug, Clone)]
pub enum GeminiAuth {
    ApiKey(String),
    ServiceAccount(Box<ServiceAccountCredentials>),
}

/// JWT claims for Google OAuth2
#[derive(Debug, Serialize)]
struct JwtClaims {
    iss: String,
    scope: String,
    aud: String,
    iat: i64,
    exp: i64,
}

/// Cached access token with expiration
#[derive(Debug, Clone)]
struct CachedToken {
    access_token: String,
    expires_at: DateTime<Utc>,
}

/// Response from Google OAuth2 token endpoint
#[derive(Debug, Deserialize)]
struct TokenResponse {
    access_token: String,
    expires_in: i64,
    #[allow(dead_code)]
    token_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tool {
    pub function_declarations: Vec<FunctionDeclaration>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionDeclaration {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerateRequest {
    pub contents: Vec<Content>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<Tool>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub generation_config: Option<GenerationConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Content {
    pub role: String,
    #[serde(default)]
    pub parts: Vec<Part>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Part {
    Text { text: String },
    FunctionCall { function_call: FunctionCall },
    FunctionResponse { function_response: FunctionResponse },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionCall {
    pub name: String,
    pub args: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionResponse {
    pub name: String,
    pub response: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerationConfig {
    pub temperature: Option<f32>,
    pub max_output_tokens: Option<i32>,
    pub response_mime_type: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GenerateResponse {
    pub candidates: Vec<Candidate>,
    pub usage_metadata: Option<UsageMetadata>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Candidate {
    pub content: Content,
    pub finish_reason: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UsageMetadata {
    pub prompt_token_count: i32,
    pub candidates_token_count: i32,
    pub total_token_count: i32,
}

pub struct GeminiClient {
    http: Client,
    auth: GeminiAuth,
    model: String,
    /// Cached OAuth2 access token (only used for service account auth)
    token_cache: Arc<RwLock<Option<CachedToken>>>,
}

impl GeminiClient {
    /// Create a new client with API key authentication
    pub fn new(api_key: String) -> Self {
        Self {
            http: Client::new(),
            auth: GeminiAuth::ApiKey(api_key),
            model: "gemini-3-pro-preview".to_string(),
            token_cache: Arc::new(RwLock::new(None)),
        }
    }

    /// Create a new client with service account authentication
    pub fn new_with_service_account(credentials: ServiceAccountCredentials) -> Self {
        Self {
            http: Client::new(),
            auth: GeminiAuth::ServiceAccount(Box::new(credentials)),
            model: "gemini-3-pro-preview".to_string(),
            token_cache: Arc::new(RwLock::new(None)),
        }
    }

    pub fn with_model(mut self, model: &str) -> Self {
        self.model = model.to_string();
        self
    }

    /// Get OAuth2 access token for service account auth
    /// Caches token and refreshes when expired (with 5 min buffer)
    async fn get_access_token(&self, credentials: &ServiceAccountCredentials) -> Result<String, GeminiError> {
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

        tracing::info!("Generating new OAuth2 access token for service account: {}", credentials.client_email);

        // Generate new token
        let now = Utc::now();
        // Use cloud-platform scope which has broader access
        let claims = JwtClaims {
            iss: credentials.client_email.clone(),
            scope: "https://www.googleapis.com/auth/cloud-platform".to_string(),
            aud: credentials.token_uri.clone(),
            iat: now.timestamp(),
            exp: (now + Duration::hours(1)).timestamp(),
        };

        let header = Header::new(Algorithm::RS256);
        let key = EncodingKey::from_rsa_pem(credentials.private_key.as_bytes())
            .map_err(|e| {
                tracing::error!("Invalid private key format: {}", e);
                GeminiError::Jwt(format!("Invalid private key format. Ensure you're using a valid service account JSON file. Error: {}", e))
            })?;
        
        let jwt = encode(&header, &claims, &key)
            .map_err(|e| {
                tracing::error!("Failed to encode JWT: {}", e);
                GeminiError::Jwt(format!("Failed to sign JWT token: {}", e))
            })?;

        tracing::debug!("Exchanging JWT for access token at: {}", credentials.token_uri);

        // Exchange JWT for access token
        let response = self.http
            .post(&credentials.token_uri)
            .form(&[
                ("grant_type", "urn:ietf:params:oauth:grant-type:jwt-bearer"),
                ("assertion", &jwt),
            ])
            .send()
            .await
            .map_err(|e| {
                tracing::error!("HTTP request to token endpoint failed: {}", e);
                GeminiError::Http(e)
            })?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            tracing::error!("Token exchange failed ({}): {}", status, error_text);
            return Err(GeminiError::Auth(format!(
                "OAuth token exchange failed (HTTP {}): {}",
                status.as_u16(),
                error_text
            )));
        }

        let token_response: TokenResponse = response.json().await
            .map_err(|e| {
                tracing::error!("Failed to parse token response: {}", e);
                GeminiError::Parse(format!("Failed to parse token response: {}", e))
            })?;

        tracing::info!("Successfully obtained access token (expires in {} seconds)", token_response.expires_in);

        // Cache the token
        let cached = CachedToken {
            access_token: token_response.access_token.clone(),
            expires_at: Utc::now() + Duration::seconds(token_response.expires_in),
        };

        {
            let mut cache = self.token_cache.write().await;
            *cache = Some(cached);
        }

        Ok(token_response.access_token)
    }

    pub async fn generate(&self, request: GenerateRequest) -> Result<GenerateResponse, GeminiError> {
        let (url, req_builder) = match &self.auth {
            GeminiAuth::ApiKey(key) => {
                // Use standard Generative Language API with API key
                let url = format!(
                    "{}/{}:generateContent",
                    GEMINI_API_URL, self.model
                );
                tracing::debug!("Using API key authentication with URL: {}", url);
                let req = self.http.post(&url).json(&request).query(&[("key", key)]);
                (url, req)
            }
            GeminiAuth::ServiceAccount(credentials) => {
                // Use Vertex AI endpoint for service account authentication
                let region = credentials.region();
                
                // If region is "global", use the non-regional endpoint
                // Otherwise, use the regional endpoint
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
                
                let token = self.get_access_token(credentials).await?;
                let req = self.http.post(&url).json(&request).bearer_auth(token);
                (url, req)
            }
        };

        tracing::debug!("Making Gemini API request to: {}", url);
        let req_builder = req_builder;

        let response = req_builder.send().await?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await?;
            tracing::error!("Gemini API error ({}): {}", status, error_text);
            
            // Parse the error to provide a more helpful message
            let error_msg = if let Ok(json) = serde_json::from_str::<serde_json::Value>(&error_text) {
                if let Some(message) = json.get("error").and_then(|e| e.get("message")).and_then(|m| m.as_str()) {
                    message.to_string()
                } else {
                    error_text
                }
            } else {
                error_text
            };
            
            return Err(GeminiError::Api(format!("HTTP {}: {}", status.as_u16(), error_msg)));
        }

        Ok(response.json().await?)
    }

    /// Verify the connection by making a simple API call
    pub async fn verify_connection(&self) -> Result<(), GeminiError> {
        tracing::info!("Verifying Gemini connection...");
        
        let request = GenerateRequest {
            contents: vec![Content {
                role: "user".to_string(),
                parts: vec![Part::Text { text: "Hello".to_string() }],
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

    /// Simple text generation
    pub async fn generate_text(&self, prompt: &str) -> Result<String, GeminiError> {
        let request = GenerateRequest {
            contents: vec![Content {
                role: "user".to_string(),
                parts: vec![Part::Text { text: prompt.to_string() }],
            }],
            tools: None,
            generation_config: Some(GenerationConfig {
                temperature: Some(0.7),
                max_output_tokens: None, // Allow full output from model
                response_mime_type: None,
            }),
        };

        let response = self.generate(request).await?;
        
        response.candidates
            .first()
            .and_then(|c| c.content.parts.first())
            .and_then(|p| match p {
                Part::Text { text } => Some(text.clone()),
                _ => None,
            })
            .ok_or_else(|| GeminiError::Parse("No text in response".into()))
    }

    /// Generate with JSON output
    pub async fn generate_json<T: for<'de> Deserialize<'de>>(
        &self,
        prompt: &str,
    ) -> Result<T, GeminiError> {
        let request = GenerateRequest {
            contents: vec![Content {
                role: "user".to_string(),
                parts: vec![Part::Text { text: prompt.to_string() }],
            }],
            tools: None,
            generation_config: Some(GenerationConfig {
                temperature: Some(0.3),
                max_output_tokens: None, // Allow full output from model
                response_mime_type: Some("application/json".to_string()),
            }),
        };

        let response = self.generate(request).await?;
        
        let text = response.candidates
            .first()
            .and_then(|c| c.content.parts.first())
            .and_then(|p| match p {
                Part::Text { text } => Some(text.clone()),
                _ => None,
            })
            .ok_or_else(|| GeminiError::Parse("No text in response".into()))?;

        serde_json::from_str(&text)
            .map_err(|e| GeminiError::Parse(e.to_string()))
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
        let client = GeminiClient::new("key".into())
            .with_model("gemini-1.5-pro");
        assert_eq!(client.model, "gemini-1.5-pro");
    }

    #[test]
    fn test_gemini_client_with_service_account() {
        let credentials = ServiceAccountCredentials {
            account_type: Some("service_account".to_string()),
            project_id: "test-project".to_string(),
            private_key_id: None,
            private_key: "-----BEGIN RSA PRIVATE KEY-----\ntest\n-----END RSA PRIVATE KEY-----".to_string(),
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
                assert_eq!(creds.region(), "global"); // Default region
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
            private_key: "-----BEGIN RSA PRIVATE KEY-----\ntest\n-----END RSA PRIVATE KEY-----".to_string(),
            client_email: "test@test.iam.gserviceaccount.com".to_string(),
            client_id: None,
            auth_uri: None,
            token_uri: "https://oauth2.googleapis.com/token".to_string(),
            auth_provider_x509_cert_url: None,
            client_x509_cert_url: None,
            vertex_region: Some("europe-west1".to_string()),
        };
        assert_eq!(credentials.region(), "europe-west1");
    }

    #[test]
    fn test_service_account_credentials_parse() {
        let json = r#"{
            "type": "service_account",
            "project_id": "my-project",
            "private_key_id": "key123",
            "private_key": "-----BEGIN RSA PRIVATE KEY-----\ntest\n-----END RSA PRIVATE KEY-----",
            "client_email": "sa@my-project.iam.gserviceaccount.com",
            "client_id": "123456789",
            "auth_uri": "https://accounts.google.com/o/oauth2/auth",
            "token_uri": "https://oauth2.googleapis.com/token",
            "auth_provider_x509_cert_url": "https://www.googleapis.com/oauth2/v1/certs",
            "client_x509_cert_url": "https://www.googleapis.com/robot/v1/metadata/x509/sa%40my-project.iam.gserviceaccount.com"
        }"#;
        
        let creds: ServiceAccountCredentials = serde_json::from_str(json).unwrap();
        assert_eq!(creds.project_id, "my-project");
        assert_eq!(creds.client_email, "sa@my-project.iam.gserviceaccount.com");
        assert_eq!(creds.token_uri, "https://oauth2.googleapis.com/token");
        assert_eq!(creds.region(), "global"); // Default when not specified
    }

    #[test]
    fn test_service_account_credentials_parse_with_region() {
        let json = r#"{
            "type": "service_account",
            "project_id": "my-project",
            "private_key": "-----BEGIN RSA PRIVATE KEY-----\ntest\n-----END RSA PRIVATE KEY-----",
            "client_email": "sa@my-project.iam.gserviceaccount.com",
            "token_uri": "https://oauth2.googleapis.com/token",
            "vertex_region": "asia-northeast1"
        }"#;
        
        let creds: ServiceAccountCredentials = serde_json::from_str(json).unwrap();
        assert_eq!(creds.region(), "asia-northeast1");
    }

    #[test]
    fn test_gemini_error_display() {
        let err = GeminiError::Api("rate_limited".into());
        assert_eq!(err.to_string(), "API error: rate_limited");

        let err = GeminiError::Parse("invalid json".into());
        assert_eq!(err.to_string(), "Parse error: invalid json");
    }

    #[test]
    fn test_gemini_error_jwt_display() {
        let err = GeminiError::Jwt("invalid key format".into());
        assert_eq!(err.to_string(), "JWT error: invalid key format");
    }

    #[test]
    fn test_gemini_error_auth_display() {
        let err = GeminiError::Auth("token exchange failed".into());
        assert_eq!(err.to_string(), "Auth error: token exchange failed");
    }

    #[test]
    fn test_content_deserialization_missing_parts() {
        // When parts is missing from JSON, it should default to empty vec
        let json = r#"{"role": "model"}"#;
        let content: Content = serde_json::from_str(json).unwrap();
        assert_eq!(content.role, "model");
        assert!(content.parts.is_empty());
    }

    #[test]
    fn test_content_deserialization_with_parts() {
        let json = r#"{"role": "user", "parts": [{"text": "hello"}]}"#;
        let content: Content = serde_json::from_str(json).unwrap();
        assert_eq!(content.role, "user");
        assert_eq!(content.parts.len(), 1);
    }

    #[test]
    fn test_content_serialization() {
        let content = Content {
            role: "user".into(),
            parts: vec![Part::Text { text: "Hello".into() }],
        };

        let json = serde_json::to_string(&content).unwrap();
        assert!(json.contains("\"role\":\"user\""));
        assert!(json.contains("\"text\":\"Hello\""));
    }

    #[test]
    fn test_generate_request_skips_none_fields() {
        let request = GenerateRequest {
            contents: vec![],
            tools: None,
            generation_config: None,
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(!json.contains("tools"));
        assert!(!json.contains("generation_config"));
    }

    #[test]
    fn test_generate_request_includes_optional_fields() {
        let request = GenerateRequest {
            contents: vec![],
            tools: Some(vec![Tool {
                function_declarations: vec![FunctionDeclaration {
                    name: "test_fn".into(),
                    description: "A test function".into(),
                    parameters: serde_json::json!({}),
                }],
            }]),
            generation_config: Some(GenerationConfig {
                temperature: Some(0.5),
                max_output_tokens: Some(1024),
                response_mime_type: None,
            }),
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("tools"));
        assert!(json.contains("test_fn"));
        assert!(json.contains("generation_config"));
        assert!(json.contains("\"temperature\":0.5"));
    }

    #[test]
    fn test_generate_response_deserialization() {
        let json = r#"{
            "candidates": [{
                "content": {
                    "role": "model",
                    "parts": [{"text": "Hello back!"}]
                },
                "finish_reason": "STOP"
            }],
            "usage_metadata": {
                "prompt_token_count": 10,
                "candidates_token_count": 5,
                "total_token_count": 15
            }
        }"#;

        let response: GenerateResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.candidates.len(), 1);
        assert_eq!(response.candidates[0].finish_reason, Some("STOP".into()));
        
        let usage = response.usage_metadata.unwrap();
        assert_eq!(usage.prompt_token_count, 10);
        assert_eq!(usage.total_token_count, 15);
    }

    #[test]
    fn test_part_text_serialization() {
        let part = Part::Text { text: "Hello".into() };
        let json = serde_json::to_string(&part).unwrap();
        assert_eq!(json, r#"{"text":"Hello"}"#);
    }

    #[test]
    fn test_part_function_call_serialization() {
        let part = Part::FunctionCall {
            function_call: FunctionCall {
                name: "get_weather".into(),
                args: serde_json::json!({"location": "NYC"}),
            },
        };
        let json = serde_json::to_string(&part).unwrap();
        assert!(json.contains("function_call"));
        assert!(json.contains("get_weather"));
    }
}
