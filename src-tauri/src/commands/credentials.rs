use crate::AppState;
use crate::sync::{AtlassianClient, AtlassianTokens, CloudResource};
use crate::ai::{GeminiClient, ServiceAccountCredentials};
use std::sync::Arc;
use tauri::State;
use tokio::sync::Mutex;

/// Helper function to get Gemini API key from either service account or API key credentials
pub async fn get_gemini_client(
    db: std::sync::Arc<crate::db::Database>,
    crypto: std::sync::Arc<crate::crypto::CryptoService>,
) -> Option<String> {
    // Try service account first
    let service_account: Option<(String,)> = sqlx::query_as(
        "SELECT encrypted_data FROM credentials WHERE id = 'gemini_service_account'"
    )
    .fetch_optional(db.pool())
    .await
    .ok()?;
    
    if let Some((encrypted_json,)) = service_account {
        if let Ok(json_content) = crypto.decrypt_string(&encrypted_json) {
            if let Ok(_credentials) = serde_json::from_str::<ServiceAccountCredentials>(&json_content) {
                // Return with prefix so ProcessingPipeline can identify auth type
                return Some(format!("SERVICE_ACCOUNT:{}", json_content));
            }
        }
    }
    
    // Try API key
    let api_key: Option<(String,)> = sqlx::query_as(
        "SELECT encrypted_data FROM credentials WHERE id = 'gemini'"
    )
    .fetch_optional(db.pool())
    .await
    .ok()?;
    
    if let Some((encrypted_key,)) = api_key {
        if let Ok(key) = crypto.decrypt_string(&encrypted_key) {
            return Some(key);
        }
    }
    
    None
}

/// Save an API key for a service
#[tauri::command]
pub async fn save_api_key(
    state: State<'_, Arc<Mutex<AppState>>>,
    service: String,
    api_key: String,
) -> Result<(), String> {
    let state = state.lock().await;
    
    let encrypted = state.crypto
        .encrypt_string(&api_key)
        .map_err(|e| e.to_string())?;
    
    let now = chrono::Utc::now().timestamp();
    sqlx::query(
        "INSERT INTO credentials (id, service, encrypted_data, created_at, updated_at) 
         VALUES (?, ?, ?, ?, ?)
         ON CONFLICT(id) DO UPDATE SET encrypted_data = ?, updated_at = ?"
    )
    .bind(&service)
    .bind(&service)
    .bind(&encrypted)
    .bind(now)
    .bind(now)
    .bind(&encrypted)
    .bind(now)
    .execute(state.db.pool())
    .await
    .map_err(|e| e.to_string())?;
    
    // When saving a Gemini API key, delete any existing service account credentials
    // so the API key takes priority (get_gemini_client checks service account first)
    if service == "gemini" {
        sqlx::query("DELETE FROM credentials WHERE id = 'gemini_service_account'")
            .execute(state.db.pool())
            .await
            .map_err(|e| e.to_string())?;
    }
    
    tracing::info!("Saved encrypted API key for service: {}", service);
    Ok(())
}

/// Check if an API key exists for a service
#[tauri::command]
pub async fn has_api_key(
    state: State<'_, Arc<Mutex<AppState>>>,
    service: String,
) -> Result<bool, String> {
    let state = state.lock().await;
    
    let result: Option<(String,)> = sqlx::query_as(
        "SELECT encrypted_data FROM credentials WHERE id = ?"
    )
    .bind(&service)
    .fetch_optional(state.db.pool())
    .await
    .map_err(|e| e.to_string())?;
    
    Ok(result.is_some())
}

/// Save Gemini service account credentials
#[tauri::command]
pub async fn save_gemini_credentials(
    state: State<'_, Arc<Mutex<AppState>>>,
    json_content: String,
    region: Option<String>,
) -> Result<(), String> {
    let mut credentials: ServiceAccountCredentials = serde_json::from_str(&json_content)
        .map_err(|e| format!("Invalid service account JSON: {}", e))?;
    
    if let Some(r) = region {
        if !r.is_empty() {
            credentials.vertex_region = Some(r);
        }
    }
    
    let json_with_region = serde_json::to_string(&credentials)
        .map_err(|e| format!("Failed to serialize credentials: {}", e))?;
    
    let state = state.lock().await;
    
    let encrypted = state.crypto
        .encrypt_string(&json_with_region)
        .map_err(|e| e.to_string())?;
    
    let now = chrono::Utc::now().timestamp();
    
    sqlx::query(
        "INSERT INTO credentials (id, service, encrypted_data, created_at, updated_at) 
         VALUES ('gemini_service_account', 'gemini', ?, ?, ?)
         ON CONFLICT(id) DO UPDATE SET encrypted_data = ?, updated_at = ?"
    )
    .bind(&encrypted)
    .bind(now)
    .bind(now)
    .bind(&encrypted)
    .bind(now)
    .execute(state.db.pool())
    .await
    .map_err(|e| e.to_string())?;
    
    sqlx::query("DELETE FROM credentials WHERE id = 'gemini'")
        .execute(state.db.pool())
        .await
        .map_err(|e| e.to_string())?;
    
    tracing::info!("Saved encrypted Gemini service account credentials (region: {})", 
        credentials.region());
    Ok(())
}

/// Verify Gemini connection works with current credentials
#[tauri::command]
pub async fn verify_gemini_connection(
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<(), String> {
    let (db, crypto) = {
        let state = state.lock().await;
        (state.db.clone(), state.crypto.clone())
    };
    
    // Try service account first
    let service_account: Option<(String,)> = sqlx::query_as(
        "SELECT encrypted_data FROM credentials WHERE id = 'gemini_service_account'"
    )
    .fetch_optional(db.pool())
    .await
    .map_err(|e| e.to_string())?;
    
    if let Some((encrypted_json,)) = service_account {
        tracing::info!("Verifying Gemini connection with service account...");
        
        let json_content = crypto.decrypt_string(&encrypted_json)
            .map_err(|e| format!("Failed to decrypt credentials: {}", e))?;
        
        let credentials: ServiceAccountCredentials = serde_json::from_str(&json_content)
            .map_err(|e| format!("Invalid service account JSON: {}", e))?;
        
        tracing::info!("Using service account: {}", credentials.client_email);
        
        let client = GeminiClient::new_with_service_account(credentials);
        client.verify_connection().await
            .map_err(|e| e.to_string())?;
        
        return Ok(());
    }
    
    // Try API key
    let api_key: Option<(String,)> = sqlx::query_as(
        "SELECT encrypted_data FROM credentials WHERE id = 'gemini'"
    )
    .fetch_optional(db.pool())
    .await
    .map_err(|e| e.to_string())?;
    
    if let Some((encrypted_key,)) = api_key {
        tracing::info!("Verifying Gemini connection with API key...");
        
        let key = crypto.decrypt_string(&encrypted_key)
            .map_err(|e| format!("Failed to decrypt API key: {}", e))?;
        
        let client = GeminiClient::new(key);
        client.verify_connection().await
            .map_err(|e| e.to_string())?;
        
        return Ok(());
    }
    
    Err("No Gemini credentials configured".to_string())
}

/// Get the current Gemini authentication type
#[tauri::command]
pub async fn get_gemini_auth_type(
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<String, String> {
    let state = state.lock().await;
    
    // Check for service account first
    let service_account: Option<(String,)> = sqlx::query_as(
        "SELECT encrypted_data FROM credentials WHERE id = 'gemini_service_account'"
    )
    .fetch_optional(state.db.pool())
    .await
    .map_err(|e| e.to_string())?;
    
    if service_account.is_some() {
        return Ok("service_account".to_string());
    }
    
    // Check for API key
    let api_key: Option<(String,)> = sqlx::query_as(
        "SELECT encrypted_data FROM credentials WHERE id = 'gemini'"
    )
    .fetch_optional(state.db.pool())
    .await
    .map_err(|e| e.to_string())?;
    
    if api_key.is_some() {
        return Ok("api_key".to_string());
    }
    
    Ok("none".to_string())
}

/// Connect to Atlassian using OAuth
#[tauri::command]
pub async fn connect_atlassian(
    state: State<'_, Arc<Mutex<AppState>>>,
    client_id: String,
    client_secret: String,
) -> Result<(AtlassianTokens, Vec<CloudResource>), String> {
    let client = AtlassianClient::new(client_id, client_secret);
    let (tokens, resources) = client.start_oauth_flow().await.map_err(|e| e.to_string())?;
    
    // Store tokens
    let state = state.lock().await;
    let encrypted = state.crypto
        .encrypt_string(&serde_json::to_string(&tokens).unwrap())
        .map_err(|e| e.to_string())?;
    
    let now = chrono::Utc::now().timestamp();
    sqlx::query(
        "INSERT INTO credentials (id, service, encrypted_data, created_at, updated_at)
         VALUES ('atlassian', 'atlassian', ?, ?, ?)
         ON CONFLICT(id) DO UPDATE SET encrypted_data = ?, updated_at = ?"
    )
    .bind(&encrypted)
    .bind(now)
    .bind(now)
    .bind(&encrypted)
    .bind(now)
    .execute(state.db.pool())
    .await
    .map_err(|e| e.to_string())?;
    
    tracing::info!("Atlassian connected with {} cloud resources", resources.len());
    Ok((tokens, resources))
}

/// Select an Atlassian cloud resource
#[tauri::command]
pub async fn select_atlassian_resource(
    state: State<'_, Arc<Mutex<AppState>>>,
    cloud_id: String,
) -> Result<(), String> {
    let state = state.lock().await;
    
    sqlx::query(
        "INSERT INTO preferences (key, value) VALUES ('atlassian_cloud_id', ?)
         ON CONFLICT(key) DO UPDATE SET value = ?"
    )
    .bind(&cloud_id)
    .bind(&cloud_id)
    .execute(state.db.pool())
    .await
    .map_err(|e| e.to_string())?;
    
    tracing::info!("Selected Atlassian cloud resource: {}", cloud_id);
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_service_account_prefix() {
        let json = r#"{"client_email":"test@example.com"}"#;
        let prefixed = format!("SERVICE_ACCOUNT:{}", json);
        
        assert!(prefixed.starts_with("SERVICE_ACCOUNT:"));
        assert!(prefixed.contains("client_email"));
    }

    #[test]
    fn test_auth_type_values() {
        let auth_types = vec!["none", "api_key", "service_account"];
        
        for auth_type in &auth_types {
            assert!(!auth_type.is_empty());
        }
        
        assert_eq!(auth_types.len(), 3);
    }

    #[test]
    fn test_credential_id_conventions() {
        // Test that credential IDs follow expected conventions
        let gemini_api_key_id = "gemini";
        let gemini_service_account_id = "gemini_service_account";
        let atlassian_id = "atlassian";
        
        assert!(gemini_service_account_id.starts_with(gemini_api_key_id));
        assert_ne!(gemini_api_key_id, gemini_service_account_id);
        assert!(!atlassian_id.contains("gemini"));
    }

    #[test]
    fn test_timestamp_generation() {
        let now = chrono::Utc::now().timestamp();
        
        // Timestamp should be positive and reasonable (after 2020)
        assert!(now > 1577836800); // 2020-01-01
        assert!(now < 2524608000); // 2050-01-01
    }
}
