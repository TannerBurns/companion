//! Sync queue for offline operation support.
//!
//! Queues sync requests when offline and processes them when back online.

use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use tokio::sync::Mutex;

/// A request to sync a specific source
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncRequest {
    pub id: String,
    pub source: String,
    pub created_at: i64,
    pub retry_count: u32,
}

impl SyncRequest {
    pub fn new(source: String) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            source,
            created_at: chrono::Utc::now().timestamp(),
            retry_count: 0,
        }
    }
}

/// Queue for managing pending sync requests
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

    /// Add a sync request to the queue
    pub async fn enqueue(&self, request: SyncRequest) {
        let mut queue = self.queue.lock().await;
        
        // Don't add duplicate requests for the same source
        if !queue.iter().any(|r| r.source == request.source) {
            queue.push_back(request);
            tracing::info!("Sync request queued, total pending: {}", queue.len());
        }
    }

    /// Get the next request from the queue
    pub async fn dequeue(&self) -> Option<SyncRequest> {
        let mut queue = self.queue.lock().await;
        queue.pop_front()
    }

    /// Peek at the next request without removing it
    pub async fn peek(&self) -> Option<SyncRequest> {
        let queue = self.queue.lock().await;
        queue.front().cloned()
    }

    /// Get the number of pending requests
    pub async fn len(&self) -> usize {
        self.queue.lock().await.len()
    }

    /// Check if the queue is empty
    pub async fn is_empty(&self) -> bool {
        self.queue.lock().await.is_empty()
    }

    /// Get all pending requests
    pub async fn get_pending(&self) -> Vec<SyncRequest> {
        self.queue.lock().await.iter().cloned().collect()
    }

    /// Re-queue a failed request if under retry limit
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

    /// Process all queued requests with a processor function
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
                    
                    // Re-queue if under retry limit
                    self.requeue_failed(request).await;
                }
            }
        }

        (successes, failures)
    }

    /// Clear all pending requests
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
        
        // Enqueue requests
        queue.enqueue(SyncRequest::new("slack".to_string())).await;
        queue.enqueue(SyncRequest::new("jira".to_string())).await;
        
        assert_eq!(queue.len().await, 2);
        
        // Dequeue
        let req = queue.dequeue().await.unwrap();
        assert_eq!(req.source, "slack");
        assert_eq!(queue.len().await, 1);
    }

    #[tokio::test]
    async fn test_no_duplicates() {
        let queue = SyncQueue::new();
        
        queue.enqueue(SyncRequest::new("slack".to_string())).await;
        queue.enqueue(SyncRequest::new("slack".to_string())).await;
        
        // Should only have one request
        assert_eq!(queue.len().await, 1);
    }

    #[tokio::test]
    async fn test_retry_limit() {
        let queue = SyncQueue::with_max_retries(2);
        
        let mut request = SyncRequest::new("slack".to_string());
        request.retry_count = 2;
        
        // Should fail - exceeded max retries
        assert!(!queue.requeue_failed(request).await);
        assert!(queue.is_empty().await);
    }
}
