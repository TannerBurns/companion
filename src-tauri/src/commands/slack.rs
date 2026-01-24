//! Slack integration commands

use crate::AppState;
use crate::sync::{SlackClient, SlackTokens, SlackChannel, SlackChannelSelection, SlackConnectionStatus, SlackUser};
use std::sync::Arc;
use tauri::State;
use tokio::sync::Mutex;

type ChannelRow = (String, String, i32, i32, i32, String, Option<i32>, Option<String>, i32);

#[tauri::command]
pub async fn connect_slack(
    state: State<'_, Arc<Mutex<AppState>>>,
    token: String,
) -> Result<SlackTokens, String> {
    let client = SlackClient::new(String::new(), String::new())
        .with_token(token.clone());
    
    let auth_info = client.test_auth().await.map_err(|e| e.to_string())?;
    
    let mut detected_scopes = Vec::new();
    
    let scope_tests = [
        ("public_channel", "channels:read"),
        ("private_channel", "groups:read"),
        ("im", "im:read"),
        ("mpim", "mpim:read"),
    ];
    
    for (channel_type, scope_name) in scope_tests {
        let resp = client.http_client()
            .get("https://slack.com/api/conversations.list")
            .bearer_auth(&token)
            .query(&[("types", channel_type), ("limit", "1")])
            .send()
            .await;
        if let Ok(resp) = resp {
            if let Ok(json) = resp.json::<serde_json::Value>().await {
                if json["ok"].as_bool().unwrap_or(false) {
                    detected_scopes.push(scope_name);
                }
            }
        }
    }
    
    let resp = client.http_client()
        .get("https://slack.com/api/users.list")
        .bearer_auth(&token)
        .query(&[("limit", "1")])
        .send()
        .await;
    if let Ok(resp) = resp {
        if let Ok(json) = resp.json::<serde_json::Value>().await {
            if json["ok"].as_bool().unwrap_or(false) {
                detected_scopes.push("users:read");
            }
        }
    }
    
    let tokens = SlackTokens {
        access_token: token,
        token_type: "bearer".to_string(),
        scope: detected_scopes.join(","),
        team_id: auth_info.team_id,
        team_name: auth_info.team_name,
        team_domain: auth_info.team_domain,
        user_id: auth_info.user_id,
    };
    
    let state = state.lock().await;
    let encrypted = state.crypto
        .encrypt_string(&serde_json::to_string(&tokens).unwrap())
        .map_err(|e| e.to_string())?;
    
    let now = chrono::Utc::now().timestamp();
    sqlx::query(
        "INSERT INTO credentials (id, service, encrypted_data, created_at, updated_at)
         VALUES ('slack', 'slack', ?, ?, ?)
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
    
    tracing::info!("Slack connected for team: {}", tokens.team_name);
    Ok(tokens)
}

#[tauri::command]
pub async fn list_slack_channels(
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<Vec<SlackChannel>, String> {
    let state = state.lock().await;
    
    let result: Option<(String,)> = sqlx::query_as(
        "SELECT encrypted_data FROM credentials WHERE id = 'slack'"
    )
    .fetch_optional(state.db.pool())
    .await
    .map_err(|e| e.to_string())?;
    
    let encrypted = result.ok_or("Slack not connected")?;
    let tokens_json = state.crypto
        .decrypt_string(&encrypted.0)
        .map_err(|e| e.to_string())?;
    let tokens: SlackTokens = serde_json::from_str(&tokens_json)
        .map_err(|e| e.to_string())?;
    
    let client = SlackClient::new(String::new(), String::new())
        .with_token(tokens.access_token)
        .with_team_id(tokens.team_id);
    
    let channels = client.list_channels().await.map_err(|e| e.to_string())?;
    Ok(channels)
}

#[tauri::command]
pub async fn list_slack_users(
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<Vec<SlackUser>, String> {
    let state = state.lock().await;
    
    let result: Option<(String,)> = sqlx::query_as(
        "SELECT encrypted_data FROM credentials WHERE id = 'slack'"
    )
    .fetch_optional(state.db.pool())
    .await
    .map_err(|e| e.to_string())?;
    
    let encrypted = result.ok_or("Slack not connected")?;
    let tokens_json = state.crypto
        .decrypt_string(&encrypted.0)
        .map_err(|e| e.to_string())?;
    let tokens: SlackTokens = serde_json::from_str(&tokens_json)
        .map_err(|e| e.to_string())?;
    
    let client = SlackClient::new(String::new(), String::new())
        .with_token(tokens.access_token)
        .with_team_id(tokens.team_id);
    
    let users = client.list_users().await.map_err(|e| e.to_string())?;
    
    tracing::info!("Listed {} Slack users", users.len());
    Ok(users)
}

#[tauri::command]
pub async fn save_slack_channels(
    state: State<'_, Arc<Mutex<AppState>>>,
    channels: Vec<SlackChannelSelection>,
) -> Result<(), String> {
    let state = state.lock().await;
    let now = chrono::Utc::now().timestamp_millis();
    
    sqlx::query("DELETE FROM slack_selected_channels")
        .execute(state.db.pool())
        .await
        .map_err(|e| e.to_string())?;
    
    for channel in &channels {
        sqlx::query(
            "INSERT INTO slack_selected_channels (id, channel_id, channel_name, is_private, is_im, is_mpim, team_id, member_count, purpose, enabled, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
        )
        .bind(uuid::Uuid::new_v4().to_string())
        .bind(&channel.channel_id)
        .bind(&channel.channel_name)
        .bind(channel.is_private as i32)
        .bind(channel.is_im as i32)
        .bind(channel.is_mpim as i32)
        .bind(&channel.team_id)
        .bind(channel.member_count)
        .bind(&channel.purpose)
        .bind(channel.enabled as i32)
        .bind(now)
        .bind(now)
        .execute(state.db.pool())
        .await
        .map_err(|e| e.to_string())?;
    }
    
    tracing::info!("Saved {} Slack channels for syncing", channels.len());
    Ok(())
}

#[tauri::command]
pub async fn get_saved_slack_channels(
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<Vec<SlackChannelSelection>, String> {
    let state = state.lock().await;
    
    let rows: Vec<ChannelRow> = sqlx::query_as(
        "SELECT channel_id, channel_name, is_private, is_im, is_mpim, team_id, member_count, purpose, enabled 
         FROM slack_selected_channels"
    )
    .fetch_all(state.db.pool())
    .await
    .map_err(|e| e.to_string())?;
    
    let channels: Vec<SlackChannelSelection> = rows.into_iter().map(|row| {
        SlackChannelSelection {
            channel_id: row.0,
            channel_name: row.1,
            is_private: row.2 != 0,
            is_im: row.3 != 0,
            is_mpim: row.4 != 0,
            team_id: row.5,
            member_count: row.6,
            purpose: row.7,
            enabled: row.8 != 0,
        }
    }).collect();
    
    Ok(channels)
}

#[tauri::command]
pub async fn remove_slack_channel(
    state: State<'_, Arc<Mutex<AppState>>>,
    channel_id: String,
) -> Result<(), String> {
    let state = state.lock().await;
    
    sqlx::query("DELETE FROM slack_selected_channels WHERE channel_id = ?")
        .bind(&channel_id)
        .execute(state.db.pool())
        .await
        .map_err(|e| e.to_string())?;
    
    tracing::info!("Removed Slack channel from sync: {}", channel_id);
    Ok(())
}

#[tauri::command]
pub async fn get_slack_connection_status(
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<SlackConnectionStatus, String> {
    let state = state.lock().await;
    
    let result: Option<(String,)> = sqlx::query_as(
        "SELECT encrypted_data FROM credentials WHERE id = 'slack'"
    )
    .fetch_optional(state.db.pool())
    .await
    .map_err(|e| e.to_string())?;
    
    match result {
        Some(encrypted) => {
            let tokens_json = state.crypto
                .decrypt_string(&encrypted.0)
                .map_err(|e| e.to_string())?;
            let tokens: SlackTokens = serde_json::from_str(&tokens_json)
                .map_err(|e| e.to_string())?;
            
            let count: (i64,) = sqlx::query_as(
                "SELECT COUNT(*) FROM slack_selected_channels WHERE enabled = 1"
            )
            .fetch_one(state.db.pool())
            .await
            .map_err(|e| e.to_string())?;
            
            Ok(SlackConnectionStatus {
                connected: true,
                team_id: Some(tokens.team_id),
                team_name: Some(tokens.team_name),
                user_id: Some(tokens.user_id),
                selected_channel_count: count.0 as i32,
            })
        }
        None => {
            Ok(SlackConnectionStatus {
                connected: false,
                team_id: None,
                team_name: None,
                user_id: None,
                selected_channel_count: 0,
            })
        }
    }
}

#[tauri::command]
pub async fn disconnect_slack(
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<(), String> {
    let state = state.lock().await;
    
    sqlx::query("DELETE FROM credentials WHERE id = 'slack'")
        .execute(state.db.pool())
        .await
        .map_err(|e| e.to_string())?;
    
    sqlx::query("DELETE FROM slack_selected_channels")
        .execute(state.db.pool())
        .await
        .map_err(|e| e.to_string())?;
    
    sqlx::query("DELETE FROM sync_state WHERE source = 'slack'")
        .execute(state.db.pool())
        .await
        .map_err(|e| e.to_string())?;
    
    Ok(())
}
