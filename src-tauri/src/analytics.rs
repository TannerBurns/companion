//! Analytics tracking for usage patterns and performance metrics.
//!
//! All analytics are stored locally - no data is sent externally.

use crate::db::Database;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// A tracked analytics event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalyticsEvent {
    pub event_type: String,
    pub event_data: serde_json::Value,
}

impl AnalyticsEvent {
    pub fn new(event_type: impl Into<String>, event_data: serde_json::Value) -> Self {
        Self {
            event_type: event_type.into(),
            event_data,
        }
    }
}

/// Usage summary over a time period
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageSummary {
    pub total_syncs: i32,
    pub total_ai_requests: i32,
    pub total_views: i32,
    pub total_source_clicks: i32,
    pub days: i32,
}

/// Analytics tracking service
pub struct AnalyticsService {
    db: Arc<Database>,
}

impl AnalyticsService {
    pub fn new(db: Arc<Database>) -> Self {
        Self { db }
    }

    /// Track a generic event
    pub async fn track(&self, event: AnalyticsEvent) -> Result<(), sqlx::Error> {
        let now = chrono::Utc::now().timestamp();

        sqlx::query(
            "INSERT INTO analytics (event_type, event_data, created_at) VALUES (?, ?, ?)",
        )
        .bind(&event.event_type)
        .bind(serde_json::to_string(&event.event_data).unwrap_or_default())
        .bind(now)
        .execute(self.db.pool())
        .await?;

        Ok(())
    }

    /// Track a view event
    pub async fn track_view(&self, view_name: &str) -> Result<(), sqlx::Error> {
        self.track(AnalyticsEvent::new(
            "view",
            serde_json::json!({ "view": view_name }),
        ))
        .await
    }

    /// Track a sync event
    pub async fn track_sync(
        &self,
        source: &str,
        items: i32,
        duration_ms: i64,
    ) -> Result<(), sqlx::Error> {
        self.track(AnalyticsEvent::new(
            "sync",
            serde_json::json!({
                "source": source,
                "items_synced": items,
                "duration_ms": duration_ms
            }),
        ))
        .await
    }

    /// Track an AI request
    pub async fn track_ai_request(
        &self,
        model: &str,
        tokens: i32,
        latency_ms: i64,
    ) -> Result<(), sqlx::Error> {
        self.track(AnalyticsEvent::new(
            "ai_request",
            serde_json::json!({
                "model": model,
                "tokens": tokens,
                "latency_ms": latency_ms
            }),
        ))
        .await
    }

    /// Track when user clicks through to a source
    pub async fn track_source_click(&self, source: &str, item_id: &str) -> Result<(), sqlx::Error> {
        self.track(AnalyticsEvent::new(
            "source_click",
            serde_json::json!({
                "source": source,
                "item_id": item_id
            }),
        ))
        .await
    }

    /// Track when user categorizes an item
    pub async fn track_categorization(
        &self,
        item_id: &str,
        from_category: Option<&str>,
        to_category: &str,
    ) -> Result<(), sqlx::Error> {
        self.track(AnalyticsEvent::new(
            "categorization",
            serde_json::json!({
                "item_id": item_id,
                "from_category": from_category,
                "to_category": to_category
            }),
        ))
        .await
    }

    /// Get usage summary for a time period
    pub async fn get_summary(&self, days: i32) -> Result<UsageSummary, sqlx::Error> {
        let since = chrono::Utc::now().timestamp() - (days as i64 * 86400);

        let total_syncs: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM analytics WHERE event_type = 'sync' AND created_at >= ?",
        )
        .bind(since)
        .fetch_one(self.db.pool())
        .await?;

        let total_ai_requests: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM analytics WHERE event_type = 'ai_request' AND created_at >= ?",
        )
        .bind(since)
        .fetch_one(self.db.pool())
        .await?;

        let total_views: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM analytics WHERE event_type = 'view' AND created_at >= ?",
        )
        .bind(since)
        .fetch_one(self.db.pool())
        .await?;

        let total_source_clicks: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM analytics WHERE event_type = 'source_click' AND created_at >= ?",
        )
        .bind(since)
        .fetch_one(self.db.pool())
        .await?;

        Ok(UsageSummary {
            total_syncs: total_syncs.0 as i32,
            total_ai_requests: total_ai_requests.0 as i32,
            total_views: total_views.0 as i32,
            total_source_clicks: total_source_clicks.0 as i32,
            days,
        })
    }

    /// Get event counts grouped by type
    pub async fn get_event_counts(
        &self,
        days: i32,
    ) -> Result<Vec<(String, i64)>, sqlx::Error> {
        let since = chrono::Utc::now().timestamp() - (days as i64 * 86400);

        let counts: Vec<(String, i64)> = sqlx::query_as(
            "SELECT event_type, COUNT(*) FROM analytics WHERE created_at >= ? GROUP BY event_type",
        )
        .bind(since)
        .fetch_all(self.db.pool())
        .await?;

        Ok(counts)
    }

    /// Get sync performance metrics
    pub async fn get_sync_metrics(&self, days: i32) -> Result<SyncMetrics, sqlx::Error> {
        let since = chrono::Utc::now().timestamp() - (days as i64 * 86400);

        // Get average duration
        let avg_duration: (Option<f64>,) = sqlx::query_as(
            r#"
            SELECT AVG(CAST(json_extract(event_data, '$.duration_ms') AS REAL))
            FROM analytics
            WHERE event_type = 'sync' AND created_at >= ?
            "#,
        )
        .bind(since)
        .fetch_one(self.db.pool())
        .await?;

        // Get total items synced
        let total_items: (Option<i64>,) = sqlx::query_as(
            r#"
            SELECT SUM(CAST(json_extract(event_data, '$.items_synced') AS INTEGER))
            FROM analytics
            WHERE event_type = 'sync' AND created_at >= ?
            "#,
        )
        .bind(since)
        .fetch_one(self.db.pool())
        .await?;

        Ok(SyncMetrics {
            avg_duration_ms: avg_duration.0.unwrap_or(0.0) as i64,
            total_items_synced: total_items.0.unwrap_or(0) as i32,
            days,
        })
    }
}

/// Sync performance metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncMetrics {
    pub avg_duration_ms: i64,
    pub total_items_synced: i32,
    pub days: i32,
}
