-- Unified content items from all sources
CREATE TABLE IF NOT EXISTS content_items (
    id TEXT PRIMARY KEY,
    source TEXT NOT NULL,           -- 'slack', 'jira', 'confluence'
    source_id TEXT NOT NULL,        -- Original ID from source
    source_url TEXT,                -- Deep link back to source
    content_type TEXT NOT NULL,     -- 'message', 'ticket', 'page', 'comment'
    title TEXT,
    body TEXT,                       -- Raw content (encrypted)
    author TEXT,
    author_id TEXT,
    channel_or_project TEXT,        -- Slack channel / Jira project / Confluence space
    parent_id TEXT,                 -- For threading
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    synced_at INTEGER NOT NULL,
    metadata TEXT,                  -- JSON blob for source-specific data
    UNIQUE(source, source_id)
);

-- AI-generated summaries and analysis
CREATE TABLE IF NOT EXISTS ai_summaries (
    id TEXT PRIMARY KEY,
    content_item_id TEXT,
    summary_type TEXT NOT NULL,     -- 'item', 'daily', 'weekly'
    summary TEXT NOT NULL,
    highlights TEXT,                -- JSON array of key points
    category TEXT,                  -- 'sales', 'marketing', 'product', 'engineering', 'research'
    category_confidence REAL,
    importance_score REAL,
    entities TEXT,                  -- JSON: people, projects, topics
    generated_at INTEGER NOT NULL,
    user_override_category TEXT,    -- If user recategorized
    FOREIGN KEY (content_item_id) REFERENCES content_items(id) ON DELETE CASCADE
);

-- Sync state tracking
CREATE TABLE IF NOT EXISTS sync_state (
    id TEXT PRIMARY KEY,
    source TEXT NOT NULL,
    resource_type TEXT NOT NULL,    -- 'channel', 'project', 'space'
    resource_id TEXT NOT NULL,
    last_sync_at INTEGER,
    cursor TEXT,                    -- Pagination cursor for incremental sync
    status TEXT,                    -- 'pending', 'syncing', 'complete', 'error'
    error_message TEXT,
    UNIQUE(source, resource_type, resource_id)
);

-- User preferences
CREATE TABLE IF NOT EXISTS preferences (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL             -- JSON values
);

-- Analytics/audit log
CREATE TABLE IF NOT EXISTS analytics (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    event_type TEXT NOT NULL,       -- 'view', 'click', 'ai_request', 'sync'
    event_data TEXT,                -- JSON
    created_at INTEGER NOT NULL
);

-- Credentials (encrypted)
CREATE TABLE IF NOT EXISTS credentials (
    id TEXT PRIMARY KEY,
    service TEXT NOT NULL,          -- 'slack', 'atlassian', 'gemini'
    encrypted_data TEXT NOT NULL,   -- AES-GCM encrypted token data
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);

-- Create indexes for common queries
CREATE INDEX IF NOT EXISTS idx_content_items_source ON content_items(source);
CREATE INDEX IF NOT EXISTS idx_content_items_created_at ON content_items(created_at);
CREATE INDEX IF NOT EXISTS idx_content_items_channel ON content_items(channel_or_project);
CREATE INDEX IF NOT EXISTS idx_ai_summaries_type ON ai_summaries(summary_type);
CREATE INDEX IF NOT EXISTS idx_ai_summaries_category ON ai_summaries(category);
CREATE INDEX IF NOT EXISTS idx_ai_summaries_generated_at ON ai_summaries(generated_at);
CREATE INDEX IF NOT EXISTS idx_sync_state_source ON sync_state(source);
CREATE INDEX IF NOT EXISTS idx_analytics_event_type ON analytics(event_type);
CREATE INDEX IF NOT EXISTS idx_analytics_created_at ON analytics(created_at);
