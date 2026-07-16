use chrono::{DateTime, Utc};
use sqlx::{FromRow, SqlitePool};

use crate::domain::{
    CommandError, LibraryEntry, LibraryStatus, MediaKind, UpsertLibraryEntryRequest, format_utc,
};

use super::DatabaseError;

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
        let known_total = sqlx::query_scalar::<_, Option<i64>>(
            "SELECT total_units FROM media_snapshots WHERE anilist_id = ? AND media_kind = ?",
        )
        .bind(request.anilist_id)
        .bind(request.media_kind.as_str())
        .fetch_optional(&mut *transaction)
        .await
        .map_err(map_database_error)?
        .flatten();

        let entry = request.validate(known_total)?;
        let timestamp = format_utc(now);

        sqlx::query(
            "INSERT INTO library_entries (\
                anilist_id, media_kind, status, progress, score, notes, created_at, updated_at\
             ) VALUES (?, ?, ?, ?, ?, ?, ?, ?) \
             ON CONFLICT(anilist_id, media_kind) DO UPDATE SET \
                status = excluded.status, progress = excluded.progress, score = excluded.score, \
                notes = excluded.notes, updated_at = excluded.updated_at",
        )
        .bind(entry.anilist_id)
        .bind(entry.media_kind.as_str())
        .bind(entry.status.as_str())
        .bind(entry.progress)
        .bind(entry.score)
        .bind(&entry.notes)
        .bind(&timestamp)
        .bind(&timestamp)
        .execute(&mut *transaction)
        .await
        .map_err(map_database_error)?;

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

    use crate::database::{Database, DatabasePaths};
    use crate::domain::{LibraryStatus, MediaKind, UpsertLibraryEntryRequest};

    #[tokio::test]
    async fn updates_preserve_creation_time_and_advance_update_time() {
        let root = tempdir().expect("temporary database root");
        let paths = DatabasePaths::from_root(root.path()).expect("valid paths");
        let database = Database::connect(&paths).await.expect("database");
        let repository = database.library();
        let first_time: DateTime<Utc> = "2026-07-16T08:00:00Z".parse().expect("first time");
        let second_time: DateTime<Utc> = "2026-07-16T08:30:00Z".parse().expect("second time");
        let request = UpsertLibraryEntryRequest {
            anilist_id: 1,
            media_kind: MediaKind::Anime,
            status: LibraryStatus::Planning,
            progress: 0,
            score: None,
            notes: None,
        };

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
}
