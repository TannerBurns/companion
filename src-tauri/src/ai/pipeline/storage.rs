use std::collections::HashMap;
use sqlx::{Pool, Sqlite};
use super::topics::{generate_topic_id, merge_message_ids};
use super::super::prompts::GroupedAnalysisResult;

/// Fetch message IDs for a topic from the database.
pub async fn fetch_message_ids_from_db(
    pool: &Pool<Sqlite>,
    topic_id: &str,
) -> Result<Vec<String>, String> {
    let result: Option<(Option<String>,)> = sqlx::query_as(
        "SELECT entities FROM ai_summaries WHERE id = ?"
    )
    .bind(topic_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| e.to_string())?;

    match result {
        Some((Some(entities_json),)) => {
            let entities: serde_json::Value = serde_json::from_str(&entities_json)
                .map_err(|e| e.to_string())?;
            let message_ids: Vec<String> = entities.get("message_ids")
                .and_then(|v| serde_json::from_value(v.clone()).ok())
                .unwrap_or_default();
            Ok(message_ids)
        }
        _ => Ok(vec![]),
    }
}

/// Store processing results to the database.
/// 
/// The `generated_at` timestamp is set to noon (12:00) of the target date to ensure
/// items appear when querying for that day's digest regardless of timezone.
pub async fn store_results(
    pool: &Pool<Sqlite>,
    result: &GroupedAnalysisResult,
    date_str: &str,
    existing_message_ids_map: &mut HashMap<String, Vec<String>>,
) -> Result<i32, String> {
    // Use noon of the target date for generated_at, so items appear in that day's digest
    let target_date = chrono::NaiveDate::parse_from_str(date_str, "%Y-%m-%d")
        .map_err(|e| format!("Invalid date format: {}", e))?;
    let noon_utc = target_date
        .and_hms_opt(12, 0, 0)
        .ok_or("Invalid date")?
        .and_utc()
        .timestamp_millis();
    let generated_at = noon_utc;
    let mut stored_count = 0;

    // Store topic groups
    for group in &result.groups {
        let ai_recognized_existing = group.topic_id.is_some();
        let topic_id = group.topic_id.clone()
            .unwrap_or_else(|| generate_topic_id(&group.topic, date_str));
        
        let existing: Option<(String,)> = sqlx::query_as(
            "SELECT id FROM ai_summaries WHERE id = ?"
        )
        .bind(&topic_id)
        .fetch_optional(pool)
        .await
        .map_err(|e| e.to_string())?;

        let should_update = existing.is_some() && ai_recognized_existing;

        let merged_message_ids = if should_update {
            let existing_ids = match existing_message_ids_map.get(&topic_id) {
                Some(ids) if !ids.is_empty() => ids.clone(),
                _ => {
                    // Empty vector or missing entry - fall back to database fetch
                    // This handles cases where message_ids JSON was missing/invalid in original data
                    tracing::debug!("Topic {} has empty or missing message_ids in local map, fetching from database", topic_id);
                    let db_ids = fetch_message_ids_from_db(pool, &topic_id).await.unwrap_or_else(|e| {
                        tracing::error!("Failed to fetch message IDs for topic {}: {}", topic_id, e);
                        vec![]
                    });
                    if !db_ids.is_empty() {
                        tracing::info!("Recovered {} message IDs from database for topic {}", db_ids.len(), topic_id);
                    }
                    db_ids
                }
            };
            merge_message_ids(&existing_ids, &group.message_ids)
        } else {
            group.message_ids.clone()
        };
        
        let final_topic_id = if existing.is_some() && !ai_recognized_existing {
            let unique_suffix = &uuid::Uuid::new_v4().to_string()[..8];
            let new_id = format!("{}_{}", topic_id, unique_suffix);
            tracing::warn!(
                "Topic ID collision for '{}', generating unique ID: {}",
                group.topic,
                new_id
            );
            new_id
        } else {
            topic_id
        };

        let entities_json = serde_json::to_string(&serde_json::json!({
            "topic": &group.topic,
            "channels": &group.channels,
            "people": &group.people,
            "message_ids": &merged_message_ids
        })).unwrap_or_default();

        if should_update {
            let existing_count = merged_message_ids.len().saturating_sub(group.message_ids.len());
            tracing::info!(
                "Updating existing topic: {} (merging {} existing + {} new = {} total message_ids)", 
                group.topic, 
                existing_count,
                group.message_ids.len(),
                merged_message_ids.len()
            );
            sqlx::query(
                "UPDATE ai_summaries 
                 SET summary = ?, highlights = ?, category = ?, category_confidence = ?, importance_score = ?, entities = ?, generated_at = ?
                 WHERE id = ?"
            )
            .bind(&group.summary)
            .bind(serde_json::to_string(&group.highlights).unwrap_or_default())
            .bind(&group.category)
            .bind(0.9)
            .bind(group.importance_score)
            .bind(&entities_json)
            .bind(generated_at)
            .bind(&final_topic_id)
            .execute(pool)
            .await
            .map_err(|e| e.to_string())?;
        } else {
            tracing::info!("Creating new topic: {} (id: {})", group.topic, final_topic_id);
            sqlx::query(
                "INSERT INTO ai_summaries (id, content_item_id, summary_type, summary, highlights, category, category_confidence, importance_score, entities, generated_at)
                 VALUES (?, NULL, 'group', ?, ?, ?, ?, ?, ?, ?)"
            )
            .bind(&final_topic_id)
            .bind(&group.summary)
            .bind(serde_json::to_string(&group.highlights).unwrap_or_default())
            .bind(&group.category)
            .bind(0.9)
            .bind(group.importance_score)
            .bind(&entities_json)
            .bind(generated_at)
            .execute(pool)
            .await
            .map_err(|e| e.to_string())?;
        }

        stored_count += 1;

        // Mark individual messages as processed
        for msg_id in &group.message_ids {
            let placeholder_id = uuid::Uuid::new_v4().to_string();
            let _ = sqlx::query(
                "INSERT OR IGNORE INTO ai_summaries (id, content_item_id, summary_type, summary, category, importance_score, generated_at)
                 VALUES (?, ?, 'item', ?, ?, ?, ?)"
            )
            .bind(&placeholder_id)
            .bind(msg_id)
            .bind(format!("Part of group: {}", group.topic))
            .bind(&group.category)
            .bind(group.importance_score)
            .bind(generated_at)
            .execute(pool)
            .await;
        }
    }

    // Store ungrouped items
    for ungrouped in &result.ungrouped {
        let summary_id = uuid::Uuid::new_v4().to_string();
        
        sqlx::query(
            "INSERT OR IGNORE INTO ai_summaries (id, content_item_id, summary_type, summary, category, importance_score, generated_at)
             VALUES (?, ?, 'item', ?, ?, ?, ?)"
        )
        .bind(&summary_id)
        .bind(&ungrouped.message_id)
        .bind(&ungrouped.summary)
        .bind(&ungrouped.category)
        .bind(ungrouped.importance_score)
        .bind(generated_at)
        .execute(pool)
        .await
        .map_err(|e| e.to_string())?;

        stored_count += 1;
    }

    // Store daily summary
    let daily_digest_id = format!("daily_{}", date_str);
    let existing_daily: Option<(String,)> = sqlx::query_as(
        "SELECT id FROM ai_summaries WHERE id = ?"
    )
    .bind(&daily_digest_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| e.to_string())?;

    if existing_daily.is_some() {
        tracing::info!("Updating daily summary for {}", date_str);
        sqlx::query(
            "UPDATE ai_summaries SET summary = ?, highlights = ?, generated_at = ? WHERE id = ?"
        )
        .bind(&result.daily_summary)
        .bind(serde_json::to_string(&result.key_themes).unwrap_or_default())
        .bind(generated_at)
        .bind(&daily_digest_id)
        .execute(pool)
        .await
        .map_err(|e| e.to_string())?;
    } else {
        tracing::info!("Creating daily summary for {}", date_str);
        sqlx::query(
            "INSERT INTO ai_summaries (id, summary_type, summary, highlights, generated_at)
             VALUES (?, 'daily', ?, ?, ?)"
        )
        .bind(&daily_digest_id)
        .bind(&result.daily_summary)
        .bind(serde_json::to_string(&result.key_themes).unwrap_or_default())
        .bind(generated_at)
        .execute(pool)
        .await
        .map_err(|e| e.to_string())?;
    }

    Ok(stored_count)
}
