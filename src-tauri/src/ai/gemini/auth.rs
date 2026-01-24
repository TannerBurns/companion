use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc, Duration};
use jsonwebtoken::{encode, EncodingKey, Header, Algorithm};
use reqwest::Client;
use super::types::{GeminiError, DEFAULT_VERTEX_REGION};

/// Google Cloud Service Account credentials parsed from JSON file.
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
    /// Get the Vertex AI region, defaulting to "global".
    pub fn region(&self) -> &str {
        self.vertex_region.as_deref().unwrap_or(DEFAULT_VERTEX_REGION)
    }
}

/// Authentication method for Gemini API.
#[derive(Debug, Clone)]
pub enum GeminiAuth {
    ApiKey(String),
    ServiceAccount(Box<ServiceAccountCredentials>),
}

/// JWT claims for Google OAuth2.
#[derive(Debug, Serialize)]
struct JwtClaims {
    iss: String,
    scope: String,
    aud: String,
    iat: i64,
    exp: i64,
}

/// Cached access token with expiration.
#[derive(Debug, Clone)]
pub struct CachedToken {
    pub access_token: String,
    pub expires_at: DateTime<Utc>,
}

/// Response from Google OAuth2 token endpoint.
#[derive(Debug, Deserialize)]
struct TokenResponse {
    access_token: String,
    expires_in: i64,
    #[allow(dead_code)]
    token_type: String,
}

/// Get an OAuth2 access token for service account authentication.
///
/// Generates a JWT, exchanges it for an access token, and returns the token.
pub async fn get_access_token(
    http: &Client,
    credentials: &ServiceAccountCredentials,
) -> Result<CachedToken, GeminiError> {
    tracing::info!("Generating new OAuth2 access token for service account: {}", credentials.client_email);

    let now = Utc::now();
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
    let response = http
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

    Ok(CachedToken {
        access_token: token_response.access_token,
        expires_at: Utc::now() + Duration::seconds(token_response.expires_in),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

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
        assert_eq!(creds.region(), "global");
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
    fn test_service_account_region_default() {
        let creds = ServiceAccountCredentials {
            account_type: Some("service_account".to_string()),
            project_id: "test-project".to_string(),
            private_key_id: None,
            private_key: "key".to_string(),
            client_email: "test@test.iam.gserviceaccount.com".to_string(),
            client_id: None,
            auth_uri: None,
            token_uri: "https://oauth2.googleapis.com/token".to_string(),
            auth_provider_x509_cert_url: None,
            client_x509_cert_url: None,
            vertex_region: None,
        };
        
        assert_eq!(creds.region(), "global");
    }

    #[test]
    fn test_service_account_region_custom() {
        let creds = ServiceAccountCredentials {
            account_type: Some("service_account".to_string()),
            project_id: "test-project".to_string(),
            private_key_id: None,
            private_key: "key".to_string(),
            client_email: "test@test.iam.gserviceaccount.com".to_string(),
            client_id: None,
            auth_uri: None,
            token_uri: "https://oauth2.googleapis.com/token".to_string(),
            auth_provider_x509_cert_url: None,
            client_x509_cert_url: None,
            vertex_region: Some("europe-west1".to_string()),
        };
        
        assert_eq!(creds.region(), "europe-west1");
    }

    #[test]
    fn test_cached_token_creation() {
        let token = CachedToken {
            access_token: "test_token".to_string(),
            expires_at: Utc::now() + Duration::hours(1),
        };
        
        assert_eq!(token.access_token, "test_token");
        assert!(token.expires_at > Utc::now());
    }

    #[test]
    fn test_gemini_auth_variants() {
        let api_key_auth = GeminiAuth::ApiKey("test-key".to_string());
        match api_key_auth {
            GeminiAuth::ApiKey(key) => assert_eq!(key, "test-key"),
            _ => panic!("Expected ApiKey"),
        }

        let creds = ServiceAccountCredentials {
            account_type: None,
            project_id: "proj".to_string(),
            private_key_id: None,
            private_key: "key".to_string(),
            client_email: "email".to_string(),
            client_id: None,
            auth_uri: None,
            token_uri: "uri".to_string(),
            auth_provider_x509_cert_url: None,
            client_x509_cert_url: None,
            vertex_region: None,
        };
        let sa_auth = GeminiAuth::ServiceAccount(Box::new(creds));
        match sa_auth {
            GeminiAuth::ServiceAccount(c) => assert_eq!(c.project_id, "proj"),
            _ => panic!("Expected ServiceAccount"),
        }
    }
}
