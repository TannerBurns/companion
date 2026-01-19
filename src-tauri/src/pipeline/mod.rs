use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::Arc;
use tauri::{AppHandle, Emitter};
use tokio::sync::RwLock;

const MAX_HISTORY_SIZE: usize = 50;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineTask {
    pub id: String,
    pub task_type: PipelineTaskType,
    pub status: TaskStatus,
    pub message: String,
    pub progress: Option<f32>, // 0.0 to 1.0
    pub started_at: i64,
    pub completed_at: Option<i64>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum PipelineTaskType {
    SyncSlack,
    SyncJira,
    SyncConfluence,
    AiSummarize,
    AiCategorize,
    GenerateDailyDigest,
    GenerateWeeklyDigest,
}

impl PipelineTaskType {
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::SyncSlack => "Syncing Slack",
            Self::SyncJira => "Syncing Jira",
            Self::SyncConfluence => "Syncing Confluence",
            Self::AiSummarize => "Summarizing content",
            Self::AiCategorize => "Categorizing items",
            Self::GenerateDailyDigest => "Generating daily digest",
            Self::GenerateWeeklyDigest => "Generating weekly digest",
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            Self::SyncSlack => "🔄",
            Self::SyncJira => "🔄",
            Self::SyncConfluence => "🔄",
            Self::AiSummarize => "✨",
            Self::AiCategorize => "🏷️",
            Self::GenerateDailyDigest => "📰",
            Self::GenerateWeeklyDigest => "📊",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    Pending,
    Running,
    Completed,
    Failed,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PipelineState {
    pub active_tasks: Vec<PipelineTask>,
    pub recent_history: Vec<PipelineTask>,
    pub is_busy: bool,
}

pub struct PipelineManager {
    state: Arc<RwLock<PipelineStateInner>>,
    app_handle: Option<AppHandle>,
}

struct PipelineStateInner {
    active_tasks: Vec<PipelineTask>,
    history: VecDeque<PipelineTask>,
}

impl PipelineManager {
    pub fn new() -> Self {
        Self {
            state: Arc::new(RwLock::new(PipelineStateInner {
                active_tasks: Vec::new(),
                history: VecDeque::with_capacity(MAX_HISTORY_SIZE),
            })),
            app_handle: None,
        }
    }

    pub fn set_app_handle(&mut self, app_handle: AppHandle) {
        self.app_handle = Some(app_handle);
    }

    pub async fn start_task(&self, task_type: PipelineTaskType, message: String) -> String {
        let id = uuid::Uuid::new_v4().to_string();
        let task = PipelineTask {
            id: id.clone(),
            task_type,
            status: TaskStatus::Running,
            message,
            progress: Some(0.0),
            started_at: chrono::Utc::now().timestamp(),
            completed_at: None,
            error: None,
        };

        {
            let mut state = self.state.write().await;
            state.active_tasks.push(task.clone());
        }

        self.emit_update().await;
        id
    }

    pub async fn update_progress(&self, task_id: &str, progress: f32, message: Option<String>) {
        {
            let mut state = self.state.write().await;
            if let Some(task) = state.active_tasks.iter_mut().find(|t| t.id == task_id) {
                task.progress = Some(progress.clamp(0.0, 1.0));
                if let Some(msg) = message {
                    task.message = msg;
                }
            }
        }
        self.emit_update().await;
    }

    pub async fn complete_task(&self, task_id: &str, message: Option<String>) {
        self.finish_task(task_id, TaskStatus::Completed, message, None).await;
    }

    pub async fn fail_task(&self, task_id: &str, error: String) {
        self.finish_task(task_id, TaskStatus::Failed, None, Some(error)).await;
    }

    async fn finish_task(
        &self,
        task_id: &str,
        status: TaskStatus,
        message: Option<String>,
        error: Option<String>,
    ) {
        {
            let mut state = self.state.write().await;
            if let Some(idx) = state.active_tasks.iter().position(|t| t.id == task_id) {
                let mut task = state.active_tasks.remove(idx);
                task.status = status;
                task.completed_at = Some(chrono::Utc::now().timestamp());
                task.progress = Some(1.0);
                if let Some(msg) = message {
                    task.message = msg;
                }
                task.error = error;

                if state.history.len() >= MAX_HISTORY_SIZE {
                    state.history.pop_front();
                }
                state.history.push_back(task);
            }
        }
        self.emit_update().await;
    }

    pub async fn get_state(&self) -> PipelineState {
        let state = self.state.read().await;
        PipelineState {
            active_tasks: state.active_tasks.clone(),
            recent_history: state.history.iter().rev().take(10).cloned().collect(),
            is_busy: !state.active_tasks.is_empty(),
        }
    }

    pub async fn get_status_message(&self) -> String {
        let state = self.state.read().await;
        if state.active_tasks.is_empty() {
            "Companion".to_string()
        } else if state.active_tasks.len() == 1 {
            let task = &state.active_tasks[0];
            format!("⟳ {}", task.message)
        } else {
            format!("⟳ {} tasks running", state.active_tasks.len())
        }
    }

    pub async fn is_busy(&self) -> bool {
        let state = self.state.read().await;
        !state.active_tasks.is_empty()
    }

    async fn emit_update(&self) {
        if let Some(app_handle) = &self.app_handle {
            let state = self.get_state().await;
            let _ = app_handle.emit("pipeline:update", &state);
        }
    }
}

impl Default for PipelineManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_pipeline_task_lifecycle() {
        let manager = PipelineManager::new();
        let task_id = manager
            .start_task(PipelineTaskType::SyncSlack, "Syncing messages".to_string())
            .await;

        let state = manager.get_state().await;
        assert_eq!(state.active_tasks.len(), 1);
        assert!(state.is_busy);

        manager
            .update_progress(&task_id, 0.5, Some("50% complete".to_string()))
            .await;
        manager
            .complete_task(&task_id, Some("Done".to_string()))
            .await;

        let state = manager.get_state().await;
        assert_eq!(state.active_tasks.len(), 0);
        assert!(!state.is_busy);
        assert_eq!(state.recent_history.len(), 1);
    }

    #[tokio::test]
    async fn test_fail_task_stores_error() {
        let manager = PipelineManager::new();
        let task_id = manager
            .start_task(PipelineTaskType::SyncJira, "Syncing issues".to_string())
            .await;

        manager.fail_task(&task_id, "Connection failed".to_string()).await;

        let state = manager.get_state().await;
        assert_eq!(state.active_tasks.len(), 0);
        assert_eq!(state.recent_history.len(), 1);
        assert_eq!(state.recent_history[0].status, TaskStatus::Failed);
        assert_eq!(
            state.recent_history[0].error,
            Some("Connection failed".to_string())
        );
    }

    #[tokio::test]
    async fn test_status_message_idle() {
        let manager = PipelineManager::new();
        let msg = manager.get_status_message().await;
        assert_eq!(msg, "Companion");
    }

    #[tokio::test]
    async fn test_status_message_single_task() {
        let manager = PipelineManager::new();
        manager
            .start_task(PipelineTaskType::AiSummarize, "Processing items".to_string())
            .await;

        let msg = manager.get_status_message().await;
        assert_eq!(msg, "⟳ Processing items");
    }

    #[tokio::test]
    async fn test_status_message_multiple_tasks() {
        let manager = PipelineManager::new();
        manager
            .start_task(PipelineTaskType::SyncSlack, "Task 1".to_string())
            .await;
        manager
            .start_task(PipelineTaskType::SyncJira, "Task 2".to_string())
            .await;

        let msg = manager.get_status_message().await;
        assert_eq!(msg, "⟳ 2 tasks running");
    }

    #[test]
    fn test_task_type_display_names() {
        assert_eq!(PipelineTaskType::SyncSlack.display_name(), "Syncing Slack");
        assert_eq!(PipelineTaskType::SyncJira.display_name(), "Syncing Jira");
        assert_eq!(PipelineTaskType::SyncConfluence.display_name(), "Syncing Confluence");
        assert_eq!(PipelineTaskType::AiSummarize.display_name(), "Summarizing content");
        assert_eq!(PipelineTaskType::AiCategorize.display_name(), "Categorizing items");
        assert_eq!(PipelineTaskType::GenerateDailyDigest.display_name(), "Generating daily digest");
        assert_eq!(PipelineTaskType::GenerateWeeklyDigest.display_name(), "Generating weekly digest");
    }

    #[test]
    fn test_task_type_icons() {
        assert_eq!(PipelineTaskType::SyncSlack.icon(), "🔄");
        assert_eq!(PipelineTaskType::AiSummarize.icon(), "✨");
        assert_eq!(PipelineTaskType::AiCategorize.icon(), "🏷️");
        assert_eq!(PipelineTaskType::GenerateDailyDigest.icon(), "📰");
        assert_eq!(PipelineTaskType::GenerateWeeklyDigest.icon(), "📊");
    }

    #[tokio::test]
    async fn test_progress_clamps_to_valid_range() {
        let manager = PipelineManager::new();
        let task_id = manager
            .start_task(PipelineTaskType::SyncSlack, "Test".to_string())
            .await;

        manager.update_progress(&task_id, 1.5, None).await;
        let state = manager.get_state().await;
        assert_eq!(state.active_tasks[0].progress, Some(1.0));

        manager.update_progress(&task_id, -0.5, None).await;
        let state = manager.get_state().await;
        assert_eq!(state.active_tasks[0].progress, Some(0.0));
    }

    #[tokio::test]
    async fn test_update_nonexistent_task_is_noop() {
        let manager = PipelineManager::new();
        manager.update_progress("nonexistent", 0.5, None).await;
        manager.complete_task("nonexistent", None).await;
        let state = manager.get_state().await;
        assert!(state.active_tasks.is_empty());
        assert!(state.recent_history.is_empty());
    }

    #[test]
    fn test_pipeline_state_default() {
        let state = PipelineState::default();
        assert!(state.active_tasks.is_empty());
        assert!(state.recent_history.is_empty());
        assert!(!state.is_busy);
    }

    #[test]
    fn test_task_status_serialization() {
        assert_eq!(
            serde_json::to_string(&TaskStatus::Running).unwrap(),
            "\"running\""
        );
        assert_eq!(
            serde_json::to_string(&TaskStatus::Completed).unwrap(),
            "\"completed\""
        );
        assert_eq!(
            serde_json::to_string(&TaskStatus::Failed).unwrap(),
            "\"failed\""
        );
        assert_eq!(
            serde_json::to_string(&TaskStatus::Pending).unwrap(),
            "\"pending\""
        );
    }

    #[test]
    fn test_task_type_serialization() {
        assert_eq!(
            serde_json::to_string(&PipelineTaskType::SyncSlack).unwrap(),
            "\"sync_slack\""
        );
        assert_eq!(
            serde_json::to_string(&PipelineTaskType::GenerateDailyDigest).unwrap(),
            "\"generate_daily_digest\""
        );
    }
}
