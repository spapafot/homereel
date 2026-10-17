-- The day each video was recorded, read from the file itself because Drive only knows upload and
-- copy dates. A NULL date means the file was read and holds none, so it is not read again.
CREATE TABLE video_recorded_dates (
  drive_id TEXT PRIMARY KEY,
  recorded_date TEXT,
  md5_checksum TEXT,
  checked_at TEXT NOT NULL
);
