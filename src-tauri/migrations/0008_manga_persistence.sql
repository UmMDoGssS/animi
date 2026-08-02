ALTER TABLE media_snapshots
    ADD COLUMN volume_total INTEGER
        CHECK (volume_total IS NULL OR volume_total BETWEEN 0 AND 2147483647);

ALTER TABLE library_entries RENAME TO library_entries_before_manga;

CREATE TABLE library_entries (
    anilist_id INTEGER NOT NULL CHECK (anilist_id BETWEEN 1 AND 2147483647),
    media_kind TEXT NOT NULL CHECK (media_kind IN ('ANIME', 'MANGA')),
    status TEXT NOT NULL CHECK (status IN ('WATCHING', 'COMPLETED', 'PLANNING')),
    progress INTEGER NOT NULL CHECK (progress BETWEEN 0 AND 2147483647),
    volume_progress INTEGER CHECK (
        (media_kind = 'ANIME' AND volume_progress IS NULL)
        OR (media_kind = 'MANGA' AND volume_progress IS NOT NULL
            AND volume_progress BETWEEN 0 AND 2147483647)
    ),
    activity_progress INTEGER NOT NULL DEFAULT 0
        CHECK (activity_progress BETWEEN 0 AND 2147483647),
    activity_volume_progress INTEGER CHECK (
        (media_kind = 'ANIME' AND activity_volume_progress IS NULL)
        OR (media_kind = 'MANGA' AND activity_volume_progress IS NOT NULL
            AND activity_volume_progress BETWEEN 0 AND 2147483647)
    ),
    score REAL CHECK (score IS NULL OR (score >= 0 AND score <= 10)),
    notes TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    CHECK (activity_progress >= progress),
    CHECK (
        activity_volume_progress IS NULL
        OR activity_volume_progress >= volume_progress
    ),
    PRIMARY KEY (anilist_id, media_kind)
);

INSERT INTO library_entries (
    anilist_id,
    media_kind,
    status,
    progress,
    volume_progress,
    activity_progress,
    activity_volume_progress,
    score,
    notes,
    created_at,
    updated_at
)
SELECT
    anilist_id,
    media_kind,
    status,
    progress,
    CASE media_kind WHEN 'MANGA' THEN 0 ELSE NULL END,
    MAX(activity_progress, progress),
    CASE media_kind WHEN 'MANGA' THEN 0 ELSE NULL END,
    score,
    notes,
    created_at,
    updated_at
FROM library_entries_before_manga;

DROP TABLE library_entries_before_manga;

CREATE INDEX library_entries_status_updated_idx
    ON library_entries (status, updated_at DESC);

ALTER TABLE watch_activity RENAME TO watch_activity_before_manga;

CREATE TABLE watch_activity (
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
    progress_delta INTEGER NOT NULL
        CHECK (progress_delta BETWEEN 0 AND 2147483647),
    volume_delta INTEGER NOT NULL DEFAULT 0
        CHECK (volume_delta BETWEEN 0 AND 2147483647),
    episode_duration_minutes INTEGER
        CHECK (episode_duration_minutes IS NULL OR episode_duration_minutes > 0),
    occurred_at TEXT NOT NULL,
    CHECK (progress_delta > 0 OR volume_delta > 0),
    CHECK (media_kind = 'MANGA' OR volume_delta = 0),
    CHECK (media_kind = 'ANIME' OR episode_duration_minutes IS NULL)
);

INSERT INTO watch_activity (
    id,
    event_id,
    anilist_id,
    media_kind,
    progress_delta,
    volume_delta,
    episode_duration_minutes,
    occurred_at
)
SELECT
    id,
    event_id,
    anilist_id,
    media_kind,
    progress_delta,
    0,
    CASE media_kind WHEN 'ANIME' THEN episode_duration_minutes ELSE NULL END,
    occurred_at
FROM watch_activity_before_manga;

DROP TABLE watch_activity_before_manga;

CREATE INDEX watch_activity_occurred_at_idx
    ON watch_activity (occurred_at DESC, id DESC);

CREATE INDEX watch_activity_media_idx
    ON watch_activity (anilist_id, media_kind, occurred_at DESC);
