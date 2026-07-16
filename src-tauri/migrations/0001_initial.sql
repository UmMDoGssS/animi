PRAGMA foreign_keys = ON;

CREATE TABLE library_entries (
    anilist_id INTEGER NOT NULL CHECK (anilist_id BETWEEN 1 AND 2147483647),
    media_kind TEXT NOT NULL CHECK (media_kind IN ('ANIME', 'MANGA')),
    status TEXT NOT NULL CHECK (status IN ('WATCHING', 'COMPLETED', 'PLANNING')),
    progress INTEGER NOT NULL CHECK (progress BETWEEN 0 AND 2147483647),
    score REAL CHECK (score IS NULL OR (score >= 0 AND score <= 10)),
    notes TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    PRIMARY KEY (anilist_id, media_kind)
);

CREATE TABLE settings (
    key TEXT PRIMARY KEY NOT NULL CHECK (length(key) > 0),
    value_json TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE media_snapshots (
    anilist_id INTEGER NOT NULL CHECK (anilist_id BETWEEN 1 AND 2147483647),
    media_kind TEXT NOT NULL CHECK (media_kind IN ('ANIME', 'MANGA')),
    title TEXT NOT NULL,
    cover_url TEXT,
    media_format TEXT,
    genres_json TEXT NOT NULL DEFAULT '[]',
    tags_json TEXT NOT NULL DEFAULT '[]',
    media_status TEXT,
    total_units INTEGER CHECK (total_units IS NULL OR total_units BETWEEN 0 AND 2147483647),
    cached_at TEXT NOT NULL,
    PRIMARY KEY (anilist_id, media_kind)
);

CREATE TABLE airing_data (
    anilist_id INTEGER NOT NULL,
    media_kind TEXT NOT NULL,
    episode_number INTEGER NOT NULL CHECK (episode_number > 0),
    airing_at TEXT NOT NULL,
    fetched_at TEXT NOT NULL,
    PRIMARY KEY (anilist_id, media_kind, episode_number),
    FOREIGN KEY (anilist_id, media_kind)
        REFERENCES media_snapshots (anilist_id, media_kind)
        ON DELETE CASCADE
);

CREATE TABLE response_caches (
    cache_key TEXT PRIMARY KEY NOT NULL CHECK (length(cache_key) > 0),
    payload_json TEXT NOT NULL,
    fetched_at TEXT NOT NULL,
    expires_at TEXT NOT NULL
);

CREATE INDEX library_entries_status_updated_idx
    ON library_entries (status, updated_at DESC);

CREATE INDEX airing_data_airing_at_idx ON airing_data (airing_at);
CREATE INDEX response_caches_expires_at_idx ON response_caches (expires_at);
