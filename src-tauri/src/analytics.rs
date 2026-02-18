//! Local analytics tracking. No data is sent externally.

use crate::db::Database;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageSummary {
    pub total_syncs: i32,
    pub total_ai_requests: i32,
    pub total_views: i32,
    pub total_source_clicks: i32,
    pub days: i32,
}

pub struct AnalyticsService {
    db: Arc<Database>,
}

impl AnalyticsService {
    pub fn new(db: Arc<Database>) -> Self {
        Self { db }
    }

    pub async fn track(&self, event: AnalyticsEvent) -> Result<(), sqlx::Error> {
        let now = chrono::Utc::now().timestamp();

        sqlx::query("INSERT INTO analytics (event_type, event_data, created_at) VALUES (?, ?, ?)")
            .bind(&event.event_type)
            .bind(serde_json::to_string(&event.event_data).unwrap_or_default())
            .bind(now)
            .execute(self.db.pool())
            .await?;

        Ok(())
    }

    pub async fn track_view(&self, view_name: &str) -> Result<(), sqlx::Error> {
        self.track(AnalyticsEvent::new(
            "view",
            serde_json::json!({ "view": view_name }),
        ))
        .await
    }

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

    pub async fn get_event_counts(&self, days: i32) -> Result<Vec<(String, i64)>, sqlx::Error> {
        let since = chrono::Utc::now().timestamp() - (days as i64 * 86400);

        let counts: Vec<(String, i64)> = sqlx::query_as(
            "SELECT event_type, COUNT(*) FROM analytics WHERE created_at >= ? GROUP BY event_type",
        )
        .bind(since)
        .fetch_all(self.db.pool())
        .await?;

        Ok(counts)
    }

    pub async fn get_sync_metrics(&self, days: i32) -> Result<SyncMetrics, sqlx::Error> {
        let since = chrono::Utc::now().timestamp() - (days as i64 * 86400);

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncMetrics {
    pub avg_duration_ms: i64,
    pub total_items_synced: i32,
    pub days: i32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_analytics_event_new() {
        let event = AnalyticsEvent::new("view", serde_json::json!({"page": "home"}));
        assert_eq!(event.event_type, "view");
        assert_eq!(event.event_data["page"], "home");
    }

    #[test]
    fn test_analytics_event_new_with_string() {
        let event = AnalyticsEvent::new(String::from("sync"), serde_json::json!({}));
        assert_eq!(event.event_type, "sync");
    }

    #[test]
    fn test_analytics_event_serialization() {
        let event = AnalyticsEvent::new("click", serde_json::json!({"target": "button"}));
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("\"event_type\":\"click\""));
        assert!(json.contains("\"target\":\"button\""));
    }

    #[test]
    fn test_analytics_event_deserialization() {
        let json = r#"{"event_type":"sync","event_data":{"source":"slack"}}"#;
        let event: AnalyticsEvent = serde_json::from_str(json).unwrap();
        assert_eq!(event.event_type, "sync");
        assert_eq!(event.event_data["source"], "slack");
    }

    #[test]
    fn test_usage_summary_serialization() {
        let summary = UsageSummary {
            total_syncs: 10,
            total_ai_requests: 5,
            total_views: 100,
            total_source_clicks: 25,
            days: 7,
        };
        let json = serde_json::to_string(&summary).unwrap();
        assert!(json.contains("\"total_syncs\":10"));
        assert!(json.contains("\"days\":7"));
    }

    #[test]
    fn test_sync_metrics_serialization() {
        let metrics = SyncMetrics {
            avg_duration_ms: 1500,
            total_items_synced: 42,
            days: 30,
        };
        let json = serde_json::to_string(&metrics).unwrap();
        assert!(json.contains("\"avg_duration_ms\":1500"));
        assert!(json.contains("\"total_items_synced\":42"));
    }

    #[test]
    fn test_sync_metrics_deserialization() {
        let json = r#"{"avg_duration_ms":2000,"total_items_synced":100,"days":14}"#;
        let metrics: SyncMetrics = serde_json::from_str(json).unwrap();
        assert_eq!(metrics.avg_duration_ms, 2000);
        assert_eq!(metrics.total_items_synced, 100);
        assert_eq!(metrics.days, 14);
    }
}
