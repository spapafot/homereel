-- Duration and picture size of videos on this computer, read with ffmpeg once. Drive reports both
-- for its own files. NULLs mean ffmpeg read the file but found no such value.
CREATE TABLE video_media_info (
  drive_id TEXT PRIMARY KEY,
  duration_ms INTEGER,
  width INTEGER,
  height INTEGER,
  checked_at TEXT NOT NULL
);
