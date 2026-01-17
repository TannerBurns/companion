-- User-selected Slack channels for syncing
CREATE TABLE IF NOT EXISTS slack_selected_channels (
    id TEXT PRIMARY KEY,
    channel_id TEXT NOT NULL UNIQUE,
    channel_name TEXT NOT NULL,
    is_private INTEGER NOT NULL DEFAULT 0,
    is_im INTEGER NOT NULL DEFAULT 0,
    is_mpim INTEGER NOT NULL DEFAULT 0,
    team_id TEXT NOT NULL,
    member_count INTEGER,
    purpose TEXT,
    enabled INTEGER NOT NULL DEFAULT 1,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_slack_channels_team ON slack_selected_channels(team_id);
CREATE INDEX IF NOT EXISTS idx_slack_channels_enabled ON slack_selected_channels(enabled);
