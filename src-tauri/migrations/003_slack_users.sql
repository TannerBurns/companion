-- Slack user cache for resolving user IDs to display names
CREATE TABLE IF NOT EXISTS slack_users (
    user_id TEXT PRIMARY KEY,
    team_id TEXT NOT NULL,
    username TEXT NOT NULL,
    real_name TEXT,
    display_name TEXT,
    updated_at INTEGER NOT NULL
);

-- Index for efficient team-based lookups
CREATE INDEX IF NOT EXISTS idx_slack_users_team ON slack_users(team_id);
