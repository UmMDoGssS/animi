INSERT INTO watch_activity (
    event_id,
    anilist_id,
    media_kind,
    progress_delta,
    episode_duration_minutes,
    occurred_at
)
SELECT
    lower(
        substr(hex(randomblob(16)), 1, 8) || '-' ||
        substr(hex(randomblob(16)), 1, 4) || '-' ||
        '4' || substr(hex(randomblob(16)), 1, 3) || '-' ||
        'a' || substr(hex(randomblob(16)), 1, 3) || '-' ||
        substr(hex(randomblob(16)), 1, 12)
    ),
    library.anilist_id,
    library.media_kind,
    library.activity_progress - COALESCE((
        SELECT SUM(activity.progress_delta)
        FROM watch_activity AS activity
        WHERE activity.anilist_id = library.anilist_id
          AND activity.media_kind = library.media_kind
          AND activity.occurred_at >= library.created_at
    ), 0),
    media.duration_minutes,
    library.created_at
FROM library_entries AS library
LEFT JOIN media_snapshots AS media
  ON media.anilist_id = library.anilist_id
 AND media.media_kind = library.media_kind
WHERE library.activity_progress > COALESCE((
    SELECT SUM(activity.progress_delta)
    FROM watch_activity AS activity
    WHERE activity.anilist_id = library.anilist_id
      AND activity.media_kind = library.media_kind
      AND activity.occurred_at >= library.created_at
), 0);
