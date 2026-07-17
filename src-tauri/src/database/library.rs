use chrono::{DateTime, Duration, Utc};
use sqlx::{FromRow, SqliteConnection, SqlitePool};
use uuid::Uuid;

use crate::domain::{
    CommandError, LibraryEntry, LibraryItem, LibraryMediaSnapshot, LibraryMediaTag, LibraryStatus,
    MEDIA_DETAILS_CACHE_TTL_HOURS, MediaKind, UpsertLibraryEntryRequest, format_utc,
};

use super::{DatabaseError, MediaTag as StoredMediaTag, parse_timestamp};

const LIST_ITEMS_QUERY: &str = "SELECT \
    library.anilist_id, library.media_kind, library.status, library.progress, \
    library.score, library.notes, library.created_at, library.updated_at, \
    media.title, media.cover_url, media.media_format, media.genres_json, \
    media.tags_json, media.media_status, media.total_units, media.duration_minutes, \
    media.cached_at \
 FROM library_entries AS library \
 LEFT JOIN media_snapshots AS media \
    ON media.anilist_id = library.anilist_id \
    AND media.media_kind = library.media_kind \
 ORDER BY library.updated_at DESC, library.anilist_id ASC";

#[derive(Debug, Clone)]
pub struct LibraryRepository {
    pool: SqlitePool,
}

impl LibraryRepository {
    pub(super) fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn list(&self) -> Result<Vec<LibraryEntry>, DatabaseError> {
        let rows = sqlx::query_as::<_, LibraryEntryRow>(
            "SELECT anilist_id, media_kind, status, progress, score, notes, created_at, updated_at \
             FROM library_entries ORDER BY updated_at DESC, anilist_id ASC",
        )
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(LibraryEntry::try_from).collect()
    }

    pub async fn list_items(&self) -> Result<Vec<LibraryItem>, DatabaseError> {
        self.list_items_at(Utc::now()).await
    }

    pub(crate) async fn list_items_at(
        &self,
        now: DateTime<Utc>,
    ) -> Result<Vec<LibraryItem>, DatabaseError> {
        let rows = sqlx::query_as::<_, LibraryItemRow>(LIST_ITEMS_QUERY)
            .fetch_all(&self.pool)
            .await?;

        rows.into_iter().map(|row| row.into_item(now)).collect()
    }

    pub(super) async fn list_items_on(
        connection: &mut SqliteConnection,
        now: DateTime<Utc>,
    ) -> Result<Vec<LibraryItem>, DatabaseError> {
        let rows = sqlx::query_as::<_, LibraryItemRow>(LIST_ITEMS_QUERY)
            .fetch_all(connection)
            .await?;

        rows.into_iter().map(|row| row.into_item(now)).collect()
    }

    pub async fn upsert(
        &self,
        request: UpsertLibraryEntryRequest,
    ) -> Result<LibraryEntry, CommandError> {
        self.upsert_at(request, Utc::now()).await
    }

    pub(crate) async fn upsert_at(
        &self,
        request: UpsertLibraryEntryRequest,
        now: DateTime<Utc>,
    ) -> Result<LibraryEntry, CommandError> {
        let mut transaction = self.pool.begin().await.map_err(map_database_error)?;
        let snapshot = sqlx::query_as::<_, SnapshotProgressRow>(
            "SELECT total_units, duration_minutes FROM media_snapshots \
             WHERE anilist_id = ? AND media_kind = ?",
        )
        .bind(request.anilist_id)
        .bind(request.media_kind.as_str())
        .fetch_optional(&mut *transaction)
        .await
        .map_err(map_database_error)?;
        let previous_activity_progress = sqlx::query_scalar::<_, i64>(
            "SELECT activity_progress FROM library_entries \
             WHERE anilist_id = ? AND media_kind = ?",
        )
        .bind(request.anilist_id)
        .bind(request.media_kind.as_str())
        .fetch_optional(&mut *transaction)
        .await
        .map_err(map_database_error)?;

        let entry = request.validate(snapshot.as_ref().and_then(|row| row.total_units))?;
        let timestamp = format_utc(now);

        sqlx::query(
            "INSERT INTO library_entries (\
                anilist_id, media_kind, status, progress, activity_progress, score, notes, \
                created_at, updated_at\
             ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?) \
             ON CONFLICT(anilist_id, media_kind) DO UPDATE SET \
                status = excluded.status, progress = excluded.progress, score = excluded.score, \
                notes = excluded.notes, updated_at = excluded.updated_at, \
                activity_progress = MAX(library_entries.activity_progress, excluded.progress)",
        )
        .bind(entry.anilist_id)
        .bind(entry.media_kind.as_str())
        .bind(entry.status.as_str())
        .bind(entry.progress)
        .bind(entry.progress)
        .bind(entry.score)
        .bind(&entry.notes)
        .bind(&timestamp)
        .bind(&timestamp)
        .execute(&mut *transaction)
        .await
        .map_err(map_database_error)?;

        // Saving nonzero progress is an explicit record that those episodes were watched, even
        // for a new entry. The high-water mark still prevents corrections and later restores
        // from double-counting episodes or rewriting the append-only history.
        if let Some(progress_delta) = entry
            .progress
            .checked_sub(previous_activity_progress.unwrap_or(0))
            .filter(|delta| *delta > 0)
        {
            sqlx::query(
                "INSERT INTO watch_activity (\
                    event_id, anilist_id, media_kind, progress_delta, \
                    episode_duration_minutes, occurred_at\
                 ) VALUES (?, ?, ?, ?, ?, ?)",
            )
            .bind(Uuid::new_v4().to_string())
            .bind(entry.anilist_id)
            .bind(entry.media_kind.as_str())
            .bind(progress_delta)
            .bind(snapshot.and_then(|row| row.duration_minutes))
            .bind(&timestamp)
            .execute(&mut *transaction)
            .await
            .map_err(map_database_error)?;
        }

        let row = sqlx::query_as::<_, LibraryEntryRow>(
            "SELECT anilist_id, media_kind, status, progress, score, notes, created_at, updated_at \
             FROM library_entries WHERE anilist_id = ? AND media_kind = ?",
        )
        .bind(entry.anilist_id)
        .bind(entry.media_kind.as_str())
        .fetch_one(&mut *transaction)
        .await
        .map_err(map_database_error)?;

        transaction.commit().await.map_err(map_database_error)?;
        LibraryEntry::try_from(row).map_err(DatabaseError::into_command_error)
    }

    pub async fn advance_home_progress(
        &self,
        anilist_id: i64,
    ) -> Result<LibraryEntry, CommandError> {
        self.advance_home_progress_at(anilist_id, Utc::now()).await
    }

    pub(crate) async fn advance_home_progress_at(
        &self,
        anilist_id: i64,
        now: DateTime<Utc>,
    ) -> Result<LibraryEntry, CommandError> {
        if !(1..=i64::from(i32::MAX)).contains(&anilist_id) {
            return Err(CommandError::new(
                "INVALID_ANILIST_ID",
                "The AniList identifier is outside the supported range.",
                false,
            ));
        }
        let timestamp = format_utc(now);
        let mut transaction = self.pool.begin().await.map_err(map_database_error)?;
        let row = sqlx::query_as::<_, HomeProgressRow>(
            "SELECT library.status, library.progress, library.activity_progress, library.score, \
                library.notes, library.created_at, media.total_units, media.duration_minutes \
             FROM library_entries AS library \
             LEFT JOIN media_snapshots AS media \
               ON media.anilist_id = library.anilist_id \
              AND media.media_kind = library.media_kind \
             WHERE library.anilist_id = ? AND library.media_kind = 'ANIME'",
        )
        .bind(anilist_id)
        .fetch_optional(&mut *transaction)
        .await
        .map_err(map_database_error)?
        .ok_or_else(|| {
            CommandError::new(
                "LIBRARY_ENTRY_NOT_FOUND",
                "This anime is not in the local library.",
                false,
            )
        })?;
        let status = LibraryStatus::parse(&row.status)?;
        if status != LibraryStatus::Watching {
            return Err(CommandError::new(
                "PROGRESS_NOT_ADVANCEABLE",
                "Home quick progress is available only for Watching anime.",
                false,
            ));
        }
        let maximum_progress = row.total_units.ok_or_else(|| {
            CommandError::new(
                "EPISODE_TOTAL_UNKNOWN",
                "The episode total is unknown, so Home did not change progress.",
                false,
            )
        })?;
        if row.progress >= maximum_progress {
            return Err(CommandError::new(
                "PROGRESS_NOT_ADVANCEABLE",
                "Every known episode is already marked watched.",
                false,
            ));
        }
        let progress = row.progress + 1;
        sqlx::query(
            "UPDATE library_entries SET progress = ?, activity_progress = MAX(activity_progress, ?), \
             updated_at = ? WHERE anilist_id = ? AND media_kind = 'ANIME'",
        )
        .bind(progress)
        .bind(progress)
        .bind(&timestamp)
        .bind(anilist_id)
        .execute(&mut *transaction)
        .await
        .map_err(map_database_error)?;
        if progress > row.activity_progress {
            sqlx::query(
                "INSERT INTO watch_activity (\
                    event_id, anilist_id, media_kind, progress_delta, \
                    episode_duration_minutes, occurred_at\
                 ) VALUES (?, ?, 'ANIME', ?, ?, ?)",
            )
            .bind(Uuid::new_v4().to_string())
            .bind(anilist_id)
            .bind(progress - row.activity_progress)
            .bind(row.duration_minutes)
            .bind(&timestamp)
            .execute(&mut *transaction)
            .await
            .map_err(map_database_error)?;
        }
        transaction.commit().await.map_err(map_database_error)?;
        Ok(LibraryEntry {
            anilist_id,
            media_kind: MediaKind::Anime,
            status,
            progress,
            score: row.score,
            notes: row.notes,
            created_at: row.created_at,
            updated_at: timestamp,
        })
    }

    pub async fn remove(
        &self,
        anilist_id: i64,
        media_kind: MediaKind,
    ) -> Result<bool, DatabaseError> {
        let result =
            sqlx::query("DELETE FROM library_entries WHERE anilist_id = ? AND media_kind = ?")
                .bind(anilist_id)
                .bind(media_kind.as_str())
                .execute(&self.pool)
                .await?;

        Ok(result.rows_affected() > 0)
    }
}

fn map_database_error(error: sqlx::Error) -> CommandError {
    DatabaseError::from(error).into_command_error()
}

#[derive(Debug, FromRow)]
struct LibraryEntryRow {
    anilist_id: i64,
    media_kind: String,
    status: String,
    progress: i64,
    score: Option<f64>,
    notes: Option<String>,
    created_at: String,
    updated_at: String,
}

#[derive(Debug, FromRow)]
struct SnapshotProgressRow {
    total_units: Option<i64>,
    duration_minutes: Option<i64>,
}

#[derive(Debug, FromRow)]
struct HomeProgressRow {
    status: String,
    progress: i64,
    activity_progress: i64,
    score: Option<f64>,
    notes: Option<String>,
    created_at: String,
    total_units: Option<i64>,
    duration_minutes: Option<i64>,
}

#[derive(Debug, FromRow)]
struct LibraryItemRow {
    anilist_id: i64,
    media_kind: String,
    status: String,
    progress: i64,
    score: Option<f64>,
    notes: Option<String>,
    created_at: String,
    updated_at: String,
    title: Option<String>,
    cover_url: Option<String>,
    media_format: Option<String>,
    genres_json: Option<String>,
    tags_json: Option<String>,
    media_status: Option<String>,
    total_units: Option<i64>,
    duration_minutes: Option<i64>,
    cached_at: Option<String>,
}

impl LibraryItemRow {
    fn into_item(self, now: DateTime<Utc>) -> Result<LibraryItem, DatabaseError> {
        let media_kind = MediaKind::parse(&self.media_kind)
            .map_err(|error| DatabaseError::CorruptData(error.message))?;
        let status = LibraryStatus::parse(&self.status)
            .map_err(|error| DatabaseError::CorruptData(error.message))?;
        let entry = LibraryEntry {
            anilist_id: self.anilist_id,
            media_kind,
            status,
            progress: self.progress,
            score: self.score,
            notes: self.notes,
            created_at: self.created_at,
            updated_at: self.updated_at,
        };
        let media = match (self.title, self.genres_json, self.tags_json, self.cached_at) {
            (Some(title), Some(genres_json), Some(tags_json), Some(cached_at)) => {
                let cached_timestamp = parse_timestamp(&cached_at)?;
                let expires_timestamp = cached_timestamp
                    .checked_add_signed(Duration::hours(MEDIA_DETAILS_CACHE_TTL_HOURS))
                    .ok_or_else(|| {
                        DatabaseError::CorruptData(
                            "A library metadata freshness timestamp is outside the supported range."
                                .to_owned(),
                        )
                    })?;
                let tags = serde_json::from_str::<Vec<StoredMediaTag>>(&tags_json)?
                    .into_iter()
                    .map(|tag| LibraryMediaTag {
                        name: tag.name,
                        rank: tag.rank,
                    })
                    .collect();
                Some(LibraryMediaSnapshot {
                    title,
                    cover_url: self.cover_url,
                    media_format: self.media_format,
                    genres: serde_json::from_str(&genres_json)?,
                    tags,
                    status: self.media_status,
                    episodes: self.total_units,
                    duration_minutes: self.duration_minutes,
                    cached_at: format_utc(cached_timestamp),
                    expires_at: format_utc(expires_timestamp),
                    is_stale: expires_timestamp <= now,
                })
            }
            (None, None, None, None) => None,
            _ => {
                return Err(DatabaseError::CorruptData(
                    "A library metadata snapshot is incomplete.".to_owned(),
                ));
            }
        };

        Ok(LibraryItem { entry, media })
    }
}

impl TryFrom<LibraryEntryRow> for LibraryEntry {
    type Error = DatabaseError;

    fn try_from(row: LibraryEntryRow) -> Result<Self, Self::Error> {
        let media_kind = MediaKind::parse(&row.media_kind)
            .map_err(|error| DatabaseError::CorruptData(error.message))?;
        let status = LibraryStatus::parse(&row.status)
            .map_err(|error| DatabaseError::CorruptData(error.message))?;

        Ok(Self {
            anilist_id: row.anilist_id,
            media_kind,
            status,
            progress: row.progress,
            score: row.score,
            notes: row.notes,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }
}

#[cfg(test)]
mod tests {
    use chrono::{DateTime, Utc};
    use tempfile::tempdir;

    use crate::database::{Database, DatabasePaths, MediaSnapshotRecord, MediaTag};
    use crate::domain::{LibraryStatus, MediaKind, UpsertLibraryEntryRequest, format_utc};

    fn request(progress: i64, status: LibraryStatus) -> UpsertLibraryEntryRequest {
        UpsertLibraryEntryRequest {
            anilist_id: 1,
            media_kind: MediaKind::Anime,
            status,
            progress,
            score: None,
            notes: None,
        }
    }

    fn snapshot(duration_minutes: Option<i64>) -> MediaSnapshotRecord {
        MediaSnapshotRecord {
            anilist_id: 1,
            media_kind: MediaKind::Anime,
            title: "A Test Anime".to_owned(),
            cover_url: Some("https://example.invalid/cover.jpg".to_owned()),
            media_format: Some("TV".to_owned()),
            genres: vec!["Drama".to_owned()],
            tags: vec![MediaTag {
                name: "Found Family".to_owned(),
                rank: 91,
            }],
            media_status: Some("RELEASING".to_owned()),
            total_units: Some(12),
            duration_minutes,
            cached_at: "2026-07-16T08:00:00Z".parse().expect("snapshot timestamp"),
        }
    }

    #[tokio::test]
    async fn updates_preserve_creation_time_and_advance_update_time() {
        let root = tempdir().expect("temporary database root");
        let paths = DatabasePaths::from_root(root.path()).expect("valid paths");
        let database = Database::connect(&paths).await.expect("database");
        let repository = database.library();
        let first_time: DateTime<Utc> = "2026-07-16T08:00:00Z".parse().expect("first time");
        let second_time: DateTime<Utc> = "2026-07-16T08:30:00Z".parse().expect("second time");
        let request = request(0, LibraryStatus::Planning);

        let created = repository
            .upsert_at(request.clone(), first_time)
            .await
            .expect("created entry");
        let updated = repository
            .upsert_at(request, second_time)
            .await
            .expect("updated entry");

        assert_eq!(created.created_at, "2026-07-16T08:00:00.000Z");
        assert_eq!(updated.created_at, created.created_at);
        assert_eq!(updated.updated_at, "2026-07-16T08:30:00.000Z");
    }

    #[tokio::test]
    async fn enriched_items_keep_missing_metadata_explicit_and_report_freshness() {
        let root = tempdir().expect("temporary database root");
        let paths = DatabasePaths::from_root(root.path()).expect("valid paths");
        let database = Database::connect(&paths).await.expect("database");
        let repository = database.library();
        repository
            .upsert_at(
                request(2, LibraryStatus::Watching),
                "2026-07-16T08:30:00Z".parse().expect("entry time"),
            )
            .await
            .expect("entry");

        let missing = repository
            .list_items_at("2026-07-16T09:00:00Z".parse().expect("list time"))
            .await
            .expect("missing metadata item");
        assert_eq!(missing.len(), 1);
        assert!(missing[0].media.is_none());

        database
            .media()
            .upsert_snapshot(&snapshot(Some(24)))
            .await
            .expect("snapshot");
        let fresh = repository
            .list_items_at("2026-07-17T07:59:59Z".parse().expect("fresh time"))
            .await
            .expect("fresh item");
        let media = fresh[0].media.as_ref().expect("joined metadata");
        assert_eq!(media.title, "A Test Anime");
        assert_eq!(media.duration_minutes, Some(24));
        assert_eq!(media.expires_at, "2026-07-17T08:00:00.000Z");
        assert!(!media.is_stale);

        let stale = repository
            .list_items_at("2026-07-17T08:00:00Z".parse().expect("stale time"))
            .await
            .expect("stale item");
        assert!(stale[0].media.as_ref().expect("metadata").is_stale);

        sqlx::query(
            "UPDATE media_snapshots SET cached_at = ? WHERE anilist_id = ? AND media_kind = ?",
        )
        .bind(format_utc(DateTime::<Utc>::MAX_UTC))
        .bind(1_i64)
        .bind(MediaKind::Anime.as_str())
        .execute(&repository.pool)
        .await
        .expect("extreme timestamp fixture");
        let error = repository
            .list_items_at(Utc::now())
            .await
            .expect_err("freshness overflow must not panic");
        assert!(matches!(error, super::DatabaseError::CorruptData(_)));
    }

    #[tokio::test]
    async fn home_progress_uses_the_known_total_and_a_supplied_clock() {
        let root = tempdir().expect("temporary database root");
        let paths = DatabasePaths::from_root(root.path()).expect("valid paths");
        let database = Database::connect(&paths).await.expect("database");
        let repository = database.library();
        let now: DateTime<Utc> = "2026-07-17T08:00:00Z".parse().expect("clock");
        database
            .media()
            .upsert_snapshot(&snapshot(Some(24)))
            .await
            .expect("snapshot");
        repository
            .upsert_at(request(11, LibraryStatus::Watching), now)
            .await
            .expect("baseline");

        let advanced = repository
            .advance_home_progress_at(1, now)
            .await
            .expect("episode 12");
        assert_eq!(advanced.progress, 12);
        assert_eq!(advanced.updated_at, "2026-07-17T08:00:00.000Z");
        let activity = database.activity().list().await.expect("activity");
        assert_eq!(activity.len(), 2);
        assert_eq!(activity[0].progress_delta, 1);
        assert_eq!(activity[1].progress_delta, 11);
        let error = repository
            .advance_home_progress_at(1, now)
            .await
            .expect_err("known total is already watched");
        assert_eq!(error.code, "PROGRESS_NOT_ADVANCEABLE");
    }

    #[tokio::test]
    async fn activity_records_initial_progress_and_later_positive_increases() {
        let root = tempdir().expect("temporary database root");
        let paths = DatabasePaths::from_root(root.path()).expect("valid paths");
        let database = Database::connect(&paths).await.expect("database");
        let repository = database.library();
        database
            .media()
            .upsert_snapshot(&snapshot(Some(24)))
            .await
            .expect("snapshot");

        repository
            .upsert_at(
                request(3, LibraryStatus::Watching),
                "2026-07-16T08:00:00Z".parse().expect("baseline time"),
            )
            .await
            .expect("initial progress");
        let initial_activity = database.activity().list().await.expect("activity");
        assert_eq!(initial_activity.len(), 1);
        assert_eq!(initial_activity[0].progress_delta, 3);
        assert_eq!(initial_activity[0].episode_duration_minutes, Some(24));
        assert_eq!(initial_activity[0].occurred_at, "2026-07-16T08:00:00.000Z");

        repository
            .upsert_at(
                request(5, LibraryStatus::Watching),
                "2026-07-16T09:00:00Z".parse().expect("increase time"),
            )
            .await
            .expect("progress increase");
        repository
            .upsert_at(
                request(4, LibraryStatus::Watching),
                "2026-07-16T09:15:00Z".parse().expect("correction time"),
            )
            .await
            .expect("progress correction");
        database
            .media()
            .upsert_snapshot(&snapshot(Some(25)))
            .await
            .expect("updated duration snapshot");
        repository
            .upsert_at(
                request(5, LibraryStatus::Watching),
                "2026-07-16T09:30:00Z".parse().expect("restore time"),
            )
            .await
            .expect("restore corrected progress");
        repository
            .upsert_at(
                request(6, LibraryStatus::Watching),
                "2026-07-16T09:45:00Z"
                    .parse()
                    .expect("second increase time"),
            )
            .await
            .expect("second progress increase");
        repository
            .upsert_at(
                request(6, LibraryStatus::Completed),
                "2026-07-16T10:00:00Z".parse().expect("status time"),
            )
            .await
            .expect("status only update");
        repository
            .upsert_at(
                request(5, LibraryStatus::Watching),
                "2026-07-16T11:00:00Z".parse().expect("correction time"),
            )
            .await
            .expect("progress correction");

        let activity = database.activity().list().await.expect("activity");
        assert_eq!(activity.len(), 3);
        assert_eq!(activity[0].progress_delta, 1);
        assert_eq!(activity[0].episode_duration_minutes, Some(25));
        assert_eq!(activity[0].occurred_at, "2026-07-16T09:45:00.000Z");
        assert_eq!(activity[1].progress_delta, 2);
        assert_eq!(activity[1].episode_duration_minutes, Some(24));
        assert_eq!(activity[1].occurred_at, "2026-07-16T09:00:00.000Z");
        assert_eq!(activity[2], initial_activity[0]);

        let before_invalid = repository.list().await.expect("library before invalid");
        let invalid_error = repository
            .upsert_at(
                request(13, LibraryStatus::Watching),
                "2026-07-16T12:00:00Z".parse().expect("invalid time"),
            )
            .await
            .expect_err("known total should reject invalid progress");
        assert_eq!(invalid_error.code, "PROGRESS_EXCEEDS_TOTAL");
        assert_eq!(repository.list().await.expect("library"), before_invalid);
        assert_eq!(database.activity().list().await.expect("activity").len(), 3);

        assert!(
            repository
                .remove(1, MediaKind::Anime)
                .await
                .expect("remove entry")
        );
        assert_eq!(database.activity().list().await.expect("retained").len(), 3);
    }

    #[tokio::test]
    async fn activity_preserves_an_unknown_duration_snapshot() {
        let root = tempdir().expect("temporary database root");
        let paths = DatabasePaths::from_root(root.path()).expect("valid paths");
        let database = Database::connect(&paths).await.expect("database");
        let repository = database.library();
        database
            .media()
            .upsert_snapshot(&snapshot(None))
            .await
            .expect("snapshot");
        repository
            .upsert_at(
                request(2, LibraryStatus::Watching),
                "2026-07-16T08:00:00Z".parse().expect("baseline time"),
            )
            .await
            .expect("initial progress");

        let activity = database.activity().list().await.expect("activity");
        assert_eq!(activity[0].progress_delta, 2);
        assert_eq!(activity[0].episode_duration_minutes, None);
    }

    #[tokio::test]
    async fn activity_failure_rolls_back_the_progress_change() {
        let root = tempdir().expect("temporary database root");
        let paths = DatabasePaths::from_root(root.path()).expect("valid paths");
        let database = Database::connect(&paths).await.expect("database");
        let repository = database.library();
        repository
            .upsert_at(
                request(0, LibraryStatus::Watching),
                "2026-07-16T08:00:00Z".parse().expect("baseline time"),
            )
            .await
            .expect("baseline");
        let before = repository.list().await.expect("entry before failure")[0].clone();
        sqlx::query(
            "CREATE TRIGGER fail_watch_activity BEFORE INSERT ON watch_activity \
             BEGIN SELECT RAISE(ABORT, 'forced activity failure'); END",
        )
        .execute(&repository.pool)
        .await
        .expect("failure trigger");

        let mut changed = request(2, LibraryStatus::Completed);
        changed.score = Some(9.0);
        changed.notes = Some("must roll back".to_owned());
        let error = repository
            .upsert_at(
                changed,
                "2026-07-16T09:00:00Z".parse().expect("increase time"),
            )
            .await
            .expect_err("activity insert should fail");

        assert_eq!(error.code, "DATABASE_UNAVAILABLE");
        assert_eq!(repository.list().await.expect("library")[0], before);
        assert!(
            database
                .activity()
                .list()
                .await
                .expect("activity")
                .is_empty()
        );
    }
}
