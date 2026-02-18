use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use tokio::sync::Mutex;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncRequest {
    pub id: String,
    pub source: String,
    pub created_at: i64,
    pub retry_count: u32,
    /// For historical resyncs: the date string (e.g. "2026-02-17")
    pub date: Option<String>,
    /// For historical resyncs: the timezone offset in minutes
    pub timezone_offset: Option<i32>,
}

impl SyncRequest {
    pub fn new(source: String) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            source,
            created_at: chrono::Utc::now().timestamp(),
            retry_count: 0,
            date: None,
            timezone_offset: None,
        }
    }

    /// Create a request for a historical day resync.
    /// Uses `source = "historical:YYYY-MM-DD"` for deduplication.
    pub fn historical(date: String, timezone_offset: i32) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            source: format!("historical:{}", date),
            created_at: chrono::Utc::now().timestamp(),
            retry_count: 0,
            date: Some(date),
            timezone_offset: Some(timezone_offset),
        }
    }
}

pub struct SyncQueue {
    queue: Mutex<VecDeque<SyncRequest>>,
    max_retries: u32,
}

impl SyncQueue {
    pub fn new() -> Self {
        Self {
            queue: Mutex::new(VecDeque::new()),
            max_retries: 3,
        }
    }

    pub fn with_max_retries(max_retries: u32) -> Self {
        Self {
            queue: Mutex::new(VecDeque::new()),
            max_retries,
        }
    }

    pub async fn enqueue(&self, request: SyncRequest) {
        let mut queue = self.queue.lock().await;
        if !queue.iter().any(|r| r.source == request.source) {
            queue.push_back(request);
            tracing::info!("Sync request queued, total pending: {}", queue.len());
        }
    }

    pub async fn dequeue(&self) -> Option<SyncRequest> {
        self.queue.lock().await.pop_front()
    }

    pub async fn peek(&self) -> Option<SyncRequest> {
        self.queue.lock().await.front().cloned()
    }

    pub async fn len(&self) -> usize {
        self.queue.lock().await.len()
    }

    pub async fn is_empty(&self) -> bool {
        self.queue.lock().await.is_empty()
    }

    pub async fn get_pending(&self) -> Vec<SyncRequest> {
        self.queue.lock().await.iter().cloned().collect()
    }

    pub async fn requeue_failed(&self, mut request: SyncRequest) -> bool {
        request.retry_count += 1;

        if request.retry_count <= self.max_retries {
            let mut queue = self.queue.lock().await;
            queue.push_back(request);
            true
        } else {
            tracing::warn!(
                "Sync request for {} exceeded max retries, dropping",
                request.source
            );
            false
        }
    }

    pub async fn process_all<F, Fut>(&self, mut processor: F) -> (usize, usize)
    where
        F: FnMut(SyncRequest) -> Fut,
        Fut: std::future::Future<Output = Result<(), String>>,
    {
        let mut successes = 0;
        let mut failures = 0;

        while let Some(request) = self.dequeue().await {
            match processor(request.clone()).await {
                Ok(()) => {
                    successes += 1;
                    tracing::info!("Successfully processed sync request for {}", request.source);
                }
                Err(e) => {
                    failures += 1;
                    tracing::error!("Failed to process sync request: {}", e);
                    self.requeue_failed(request).await;
                }
            }
        }

        (successes, failures)
    }

    pub async fn clear(&self) {
        self.queue.lock().await.clear();
    }
}

impl Default for SyncQueue {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_queue_operations() {
        let queue = SyncQueue::new();
        queue.enqueue(SyncRequest::new("slack".to_string())).await;
        queue.enqueue(SyncRequest::new("jira".to_string())).await;
        assert_eq!(queue.len().await, 2);

        let req = queue.dequeue().await.unwrap();
        assert_eq!(req.source, "slack");
        assert_eq!(queue.len().await, 1);
    }

    #[tokio::test]
    async fn test_no_duplicates() {
        let queue = SyncQueue::new();
        queue.enqueue(SyncRequest::new("slack".to_string())).await;
        queue.enqueue(SyncRequest::new("slack".to_string())).await;
        assert_eq!(queue.len().await, 1);
    }

    #[tokio::test]
    async fn test_retry_limit() {
        let queue = SyncQueue::with_max_retries(2);
        let mut request = SyncRequest::new("slack".to_string());
        request.retry_count = 2;
        assert!(!queue.requeue_failed(request).await);
        assert!(queue.is_empty().await);
    }
}
