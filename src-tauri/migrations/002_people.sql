-- People and pets the user labels with reference photos. Every title request sends the photos so
-- Gemini can name them when they appear.
CREATE TABLE people (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  name TEXT NOT NULL UNIQUE COLLATE NOCASE CHECK (length(trim(name)) > 0),
  kind TEXT NOT NULL CHECK (kind IN ('person', 'animal')),
  description TEXT,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

-- Cropped JPEGs, small enough to send with every request.
CREATE TABLE person_photos (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  person_id INTEGER NOT NULL REFERENCES people(id) ON DELETE CASCADE,
  data BLOB NOT NULL,
  created_at TEXT NOT NULL
);
CREATE INDEX idx_person_photos_person ON person_photos(person_id, id);

-- Who Gemini recognized the last time it titled each video.
CREATE TABLE video_ai_people (
  drive_id TEXT NOT NULL,
  person_id INTEGER NOT NULL REFERENCES people(id) ON DELETE CASCADE,
  PRIMARY KEY (drive_id, person_id)
);
CREATE INDEX idx_video_ai_people_person ON video_ai_people(person_id);
