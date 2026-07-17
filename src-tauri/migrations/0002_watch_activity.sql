ALTER TABLE media_snapshots
    ADD COLUMN duration_minutes INTEGER
        CHECK (duration_minutes IS NULL OR duration_minutes > 0);

CREATE TABLE watch_activity (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    anilist_id INTEGER NOT NULL CHECK (anilist_id BETWEEN 1 AND 2147483647),
    media_kind TEXT NOT NULL CHECK (media_kind IN ('ANIME', 'MANGA')),
    progress_delta INTEGER NOT NULL CHECK (progress_delta > 0),
    episode_duration_minutes INTEGER
        CHECK (episode_duration_minutes IS NULL OR episode_duration_minutes > 0),
    occurred_at TEXT NOT NULL
);

CREATE INDEX watch_activity_occurred_at_idx
    ON watch_activity (occurred_at DESC, id DESC);

CREATE INDEX watch_activity_media_idx
    ON watch_activity (anilist_id, media_kind, occurred_at DESC);
