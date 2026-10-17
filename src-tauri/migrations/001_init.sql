-- DriveTube keeps only what it adds on top of Google Drive. Drive itself is read live and never
-- indexed. Timestamps are RFC 3339 UTC text.

-- Display metadata for videos that have local state (favorites, playlists, history, titles), so
-- those lists render without one Drive request per video. JSON of `VideoFile`.
CREATE TABLE video_snapshots (
  drive_id TEXT PRIMARY KEY,
  data TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE TABLE user_video_state (
  drive_id TEXT PRIMARY KEY,
  is_favorite INTEGER NOT NULL DEFAULT 0,
  favorited_at TEXT,
  is_watch_later INTEGER NOT NULL DEFAULT 0,
  watch_later_at TEXT,
  last_position_ms INTEGER,
  duration_ms INTEGER,
  last_watched_at TEXT,
  watched INTEGER NOT NULL DEFAULT 0,
  custom_title TEXT,
  updated_at TEXT NOT NULL
);
CREATE INDEX idx_state_favorites ON user_video_state(favorited_at DESC) WHERE is_favorite = 1;
CREATE INDEX idx_state_watch_later ON user_video_state(watch_later_at DESC) WHERE is_watch_later = 1;
CREATE INDEX idx_state_history ON user_video_state(last_watched_at DESC)
  WHERE last_watched_at IS NOT NULL;

CREATE TABLE playlists (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  name TEXT NOT NULL CHECK (length(trim(name)) > 0),
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE TABLE playlist_items (
  playlist_id INTEGER NOT NULL REFERENCES playlists(id) ON DELETE CASCADE,
  drive_id TEXT NOT NULL,
  position INTEGER NOT NULL,
  added_at TEXT NOT NULL,
  PRIMARY KEY (playlist_id, drive_id)
);
CREATE INDEX idx_playlist_items_order ON playlist_items(playlist_id, position);

-- Kept apart from snapshots so clearing cached metadata never discards titles that cost API calls.
CREATE TABLE video_ai_titles (
  drive_id TEXT PRIMARY KEY,
  title TEXT,
  confidence REAL,
  model TEXT,
  md5_checksum TEXT,
  status TEXT NOT NULL CHECK (status IN ('pending', 'done', 'failed')),
  error TEXT,
  attempts INTEGER NOT NULL DEFAULT 0,
  prompt_tokens INTEGER,
  output_tokens INTEGER,
  generated_at TEXT,
  updated_at TEXT NOT NULL
);
CREATE INDEX idx_ai_titles_status ON video_ai_titles(status);

CREATE TABLE app_settings (
  key TEXT PRIMARY KEY,
  value TEXT NOT NULL
);
