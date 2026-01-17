use tauri::AppHandle;
use tauri_plugin_notification::NotificationExt;

pub struct NotificationService {
    app_handle: AppHandle,
}

impl NotificationService {
    pub fn new(app_handle: AppHandle) -> Self {
        Self { app_handle }
    }

    pub fn notify_daily_digest(&self, item_count: i32) -> Result<(), String> {
        if item_count == 0 {
            return Ok(());
        }

        self.app_handle
            .notification()
            .builder()
            .title("Daily Digest Ready")
            .body(format!(
                "{} new {} to review",
                item_count,
                if item_count == 1 { "item" } else { "items" }
            ))
            .show()
            .map_err(|e| e.to_string())
    }

    pub fn notify_important_item(&self, title: &str, source: &str) -> Result<(), String> {
        self.app_handle
            .notification()
            .builder()
            .title("Important Update")
            .body(format!("[{}] {}", source, title))
            .show()
            .map_err(|e| e.to_string())
    }

    pub fn notify_sync_complete(&self, items_synced: i32) -> Result<(), String> {
        if items_synced > 0 {
            self.app_handle
                .notification()
                .builder()
                .title("Sync Complete")
                .body(format!(
                    "{} new {} synced",
                    items_synced,
                    if items_synced == 1 { "item" } else { "items" }
                ))
                .show()
                .map_err(|e| e.to_string())
        } else {
            Ok(())
        }
    }

    pub fn notify(&self, title: &str, body: &str) -> Result<(), String> {
        self.app_handle
            .notification()
            .builder()
            .title(title)
            .body(body)
            .show()
            .map_err(|e| e.to_string())
    }

    pub fn notify_weekly_digest(&self, item_count: i32) -> Result<(), String> {
        if item_count == 0 {
            return Ok(());
        }

        self.app_handle
            .notification()
            .builder()
            .title("Weekly Summary Ready")
            .body(format!(
                "Your week in review: {} total {}",
                item_count,
                if item_count == 1 { "item" } else { "items" }
            ))
            .show()
            .map_err(|e| e.to_string())
    }

    pub fn notify_error(&self, source: &str, error: &str) -> Result<(), String> {
        self.app_handle
            .notification()
            .builder()
            .title(format!("Sync Error: {}", source))
            .body(error)
            .show()
            .map_err(|e| e.to_string())
    }
}
