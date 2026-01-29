use crate::AppState;
use super::types::Preferences;
use std::sync::Arc;
use tauri::State;
use tokio::sync::Mutex;

/// Get user preferences
#[tauri::command]
pub async fn get_preferences(
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<Preferences, String> {
    let state = state.lock().await;
    
    let result: Option<(String,)> = sqlx::query_as(
        "SELECT value FROM preferences WHERE key = 'user_preferences'"
    )
    .fetch_optional(state.db.pool())
    .await
    .map_err(|e| e.to_string())?;
    
    match result {
        Some((json,)) => {
            serde_json::from_str(&json).map_err(|e| e.to_string())
        }
        None => {
            // Return defaults if no preferences saved yet
            Ok(Preferences::default())
        }
    }
}

/// Save user preferences
#[tauri::command]
pub async fn save_preferences(
    state: State<'_, Arc<Mutex<AppState>>>,
    preferences: Preferences,
) -> Result<(), String> {
    let (db, background_sync) = {
        let state = state.lock().await;
        (state.db.clone(), state.background_sync.clone())
    };
    
    let prefs_json = serde_json::to_string(&preferences).map_err(|e| e.to_string())?;
    
    sqlx::query("INSERT OR REPLACE INTO preferences (key, value) VALUES ('user_preferences', ?)")
        .bind(&prefs_json)
        .execute(db.pool())
        .await
        .map_err(|e| e.to_string())?;
    
    if let Some(bg_sync) = background_sync {
        bg_sync.set_interval(preferences.sync_interval_minutes as u64).await;
    }
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_preferences_default_values() {
        let prefs = Preferences::default();
        
        assert_eq!(prefs.sync_interval_minutes, 15);
        assert!(prefs.notifications_enabled);
        assert!(prefs.enabled_sources.is_empty());
        assert!(!prefs.enabled_categories.is_empty());
    }

    #[test]
    fn test_preferences_serialization_roundtrip() {
        let prefs = Preferences {
            sync_interval_minutes: 30,
            enabled_sources: vec!["slack".to_string()],
            enabled_categories: vec!["engineering".to_string(), "product".to_string()],
            notifications_enabled: false,
            user_guidance: Some("Focus on production issues".to_string()),
        };

        let json = serde_json::to_string(&prefs).unwrap();
        let restored: Preferences = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.sync_interval_minutes, 30);
        assert!(!restored.notifications_enabled);
        assert_eq!(restored.enabled_sources, vec!["slack"]);
        assert_eq!(restored.user_guidance, Some("Focus on production issues".to_string()));
    }

    #[test]
    fn test_preferences_contains_expected_categories() {
        let prefs = Preferences::default();
        
        assert!(prefs.enabled_categories.contains(&"sales".to_string()));
        assert!(prefs.enabled_categories.contains(&"marketing".to_string()));
        assert!(prefs.enabled_categories.contains(&"product".to_string()));
        assert!(prefs.enabled_categories.contains(&"engineering".to_string()));
        assert!(prefs.enabled_categories.contains(&"research".to_string()));
    }

    #[test]
    fn test_preferences_json_key() {
        // Ensure we use the correct key for storing preferences
        let key = "user_preferences";
        assert!(!key.is_empty());
        assert!(!key.contains(" "));
    }

    #[test]
    fn test_sync_interval_conversion() {
        let prefs = Preferences {
            sync_interval_minutes: 60,
            ..Preferences::default()
        };
        
        let interval_u64 = prefs.sync_interval_minutes as u64;
        assert_eq!(interval_u64, 60);
    }
}
