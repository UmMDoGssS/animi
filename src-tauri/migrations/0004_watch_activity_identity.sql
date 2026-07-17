CREATE TABLE watch_activity_with_identity (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    event_id TEXT NOT NULL UNIQUE CHECK (
        length(event_id) = 36
        AND substr(event_id, 9, 1) = '-'
        AND substr(event_id, 14, 1) = '-'
        AND substr(event_id, 19, 1) = '-'
        AND substr(event_id, 24, 1) = '-'
    ),
    anilist_id INTEGER NOT NULL CHECK (anilist_id BETWEEN 1 AND 2147483647),
    media_kind TEXT NOT NULL CHECK (media_kind IN ('ANIME', 'MANGA')),
    progress_delta INTEGER NOT NULL CHECK (progress_delta > 0),
    episode_duration_minutes INTEGER
        CHECK (episode_duration_minutes IS NULL OR episode_duration_minutes > 0),
    occurred_at TEXT NOT NULL
);

INSERT INTO watch_activity_with_identity (
    id,
    event_id,
    anilist_id,
    media_kind,
    progress_delta,
    episode_duration_minutes,
    occurred_at
)
SELECT
    id,
    lower(
        substr(hex(randomblob(16)), 1, 8) || '-' ||
        substr(hex(randomblob(16)), 1, 4) || '-' ||
        '4' || substr(hex(randomblob(16)), 1, 3) || '-' ||
        'a' || substr(hex(randomblob(16)), 1, 3) || '-' ||
        substr(hex(randomblob(16)), 1, 12)
    ),
    anilist_id,
    media_kind,
    progress_delta,
    episode_duration_minutes,
    occurred_at
FROM watch_activity;

DROP TABLE watch_activity;
ALTER TABLE watch_activity_with_identity RENAME TO watch_activity;

CREATE INDEX watch_activity_occurred_at_idx
    ON watch_activity (occurred_at DESC, id DESC);

CREATE INDEX watch_activity_media_idx
    ON watch_activity (anilist_id, media_kind, occurred_at DESC);
