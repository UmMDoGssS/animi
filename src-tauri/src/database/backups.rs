use chrono::{DateTime, Utc};
use serde_json::Value;
use sqlx::{FromRow, SqlitePool};
use uuid::Uuid;

use crate::domain::{LibraryStatus, MediaKind};

use super::{DatabaseError, parse_timestamp};

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct BackupSnapshotRecord {
    pub settings: Vec<BackupSettingRecord>,
    pub library_entries: Vec<BackupLibraryEntryRecord>,
    pub watch_activity: Vec<BackupWatchActivityRecord>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct BackupSettingRecord {
    pub key: String,
    pub value: Value,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct BackupLibraryEntryRecord {
    pub anilist_id: i64,
    pub media_kind: MediaKind,
    pub status: LibraryStatus,
    pub progress: i64,
    pub activity_progress: i64,
    pub score: Option<f64>,
    pub notes: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct BackupWatchActivityRecord {
    pub event_id: Uuid,
    pub anilist_id: i64,
    pub media_kind: MediaKind,
    pub progress_delta: i64,
    pub episode_duration_minutes: Option<i64>,
    pub occurred_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct BackupRepository {
    pool: SqlitePool,
}

impl BackupRepository {
    pub(super) fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Reads every user-owned backup table through one SQLite transaction so
    /// the exported settings, library high-water marks, and activity agree.
    pub(crate) async fn snapshot(&self) -> Result<BackupSnapshotRecord, DatabaseError> {
        let mut transaction = self.pool.begin().await?;

        let setting_rows = sqlx::query_as::<_, BackupSettingRow>(
            "SELECT key, value_json, updated_at FROM settings ORDER BY key ASC",
        )
        .fetch_all(&mut *transaction)
        .await?;
        let library_rows = sqlx::query_as::<_, BackupLibraryEntryRow>(
            "SELECT anilist_id, media_kind, status, progress, activity_progress, score, notes, \
                    created_at, updated_at \
             FROM library_entries ORDER BY anilist_id ASC, media_kind ASC",
        )
        .fetch_all(&mut *transaction)
        .await?;
        let activity_rows = sqlx::query_as::<_, BackupWatchActivityRow>(
            "SELECT event_id, anilist_id, media_kind, progress_delta, \
                    episode_duration_minutes, occurred_at \
             FROM watch_activity ORDER BY occurred_at ASC, id ASC",
        )
        .fetch_all(&mut *transaction)
        .await?;

        transaction.commit().await?;

        Ok(BackupSnapshotRecord {
            settings: setting_rows
                .into_iter()
                .map(BackupSettingRecord::try_from)
                .collect::<Result<_, _>>()?,
            library_entries: library_rows
                .into_iter()
                .map(BackupLibraryEntryRecord::try_from)
                .collect::<Result<_, _>>()?,
            watch_activity: activity_rows
                .into_iter()
                .map(BackupWatchActivityRecord::try_from)
                .collect::<Result<_, _>>()?,
        })
    }
}

#[derive(Debug, FromRow)]
struct BackupSettingRow {
    key: String,
    value_json: String,
    updated_at: String,
}

impl TryFrom<BackupSettingRow> for BackupSettingRecord {
    type Error = DatabaseError;

    fn try_from(row: BackupSettingRow) -> Result<Self, Self::Error> {
        Ok(Self {
            key: row.key,
            value: serde_json::from_str(&row.value_json)?,
            updated_at: parse_timestamp(&row.updated_at)?,
        })
    }
}

#[derive(Debug, FromRow)]
struct BackupLibraryEntryRow {
    anilist_id: i64,
    media_kind: String,
    status: String,
    progress: i64,
    activity_progress: i64,
    score: Option<f64>,
    notes: Option<String>,
    created_at: String,
    updated_at: String,
}

impl TryFrom<BackupLibraryEntryRow> for BackupLibraryEntryRecord {
    type Error = DatabaseError;

    fn try_from(row: BackupLibraryEntryRow) -> Result<Self, Self::Error> {
        if row.activity_progress < row.progress {
            return Err(DatabaseError::CorruptData(
                "A library activity high-water mark is below current progress.".to_owned(),
            ));
        }
        if row
            .score
            .is_some_and(|score| !score.is_finite() || !(0.0..=10.0).contains(&score))
        {
            return Err(DatabaseError::CorruptData(
                "A library score is outside the supported range.".to_owned(),
            ));
        }

        Ok(Self {
            anilist_id: row.anilist_id,
            media_kind: MediaKind::parse(&row.media_kind)
                .map_err(|error| DatabaseError::CorruptData(error.message))?,
            status: LibraryStatus::parse(&row.status)
                .map_err(|error| DatabaseError::CorruptData(error.message))?,
            progress: row.progress,
            activity_progress: row.activity_progress,
            score: row.score,
            notes: row.notes,
            created_at: parse_timestamp(&row.created_at)?,
            updated_at: parse_timestamp(&row.updated_at)?,
        })
    }
}

#[derive(Debug, FromRow)]
struct BackupWatchActivityRow {
    event_id: String,
    anilist_id: i64,
    media_kind: String,
    progress_delta: i64,
    episode_duration_minutes: Option<i64>,
    occurred_at: String,
}

impl TryFrom<BackupWatchActivityRow> for BackupWatchActivityRecord {
    type Error = DatabaseError;

    fn try_from(row: BackupWatchActivityRow) -> Result<Self, Self::Error> {
        Ok(Self {
            event_id: Uuid::parse_str(&row.event_id).map_err(|_| {
                DatabaseError::CorruptData(
                    "A watch activity event identifier is invalid.".to_owned(),
                )
            })?,
            anilist_id: row.anilist_id,
            media_kind: MediaKind::parse(&row.media_kind)
                .map_err(|error| DatabaseError::CorruptData(error.message))?,
            progress_delta: row.progress_delta,
            episode_duration_minutes: row.episode_duration_minutes,
            occurred_at: parse_timestamp(&row.occurred_at)?,
        })
    }
}
