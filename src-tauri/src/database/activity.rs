use sqlx::{FromRow, SqliteConnection, SqlitePool};

use crate::domain::{MediaKind, WatchActivity, format_utc};

use super::{DatabaseError, parse_timestamp};

const LIST_ACTIVITY_QUERY: &str = "SELECT anilist_id, media_kind, progress_delta, episode_duration_minutes, occurred_at \
     FROM watch_activity ORDER BY occurred_at DESC, id DESC";

#[derive(Debug, Clone)]
pub struct ActivityRepository {
    pool: SqlitePool,
}

impl ActivityRepository {
    pub(super) fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn list(&self) -> Result<Vec<WatchActivity>, DatabaseError> {
        let rows = sqlx::query_as::<_, WatchActivityRow>(LIST_ACTIVITY_QUERY)
            .fetch_all(&self.pool)
            .await?;

        rows.into_iter().map(WatchActivity::try_from).collect()
    }

    pub(super) async fn list_on(
        connection: &mut SqliteConnection,
    ) -> Result<Vec<WatchActivity>, DatabaseError> {
        let rows = sqlx::query_as::<_, WatchActivityRow>(LIST_ACTIVITY_QUERY)
            .fetch_all(connection)
            .await?;

        rows.into_iter().map(WatchActivity::try_from).collect()
    }
}

#[derive(Debug, FromRow)]
struct WatchActivityRow {
    anilist_id: i64,
    media_kind: String,
    progress_delta: i64,
    episode_duration_minutes: Option<i64>,
    occurred_at: String,
}

impl TryFrom<WatchActivityRow> for WatchActivity {
    type Error = DatabaseError;

    fn try_from(row: WatchActivityRow) -> Result<Self, Self::Error> {
        let media_kind = MediaKind::parse(&row.media_kind)
            .map_err(|error| DatabaseError::CorruptData(error.message))?;
        Ok(Self {
            anilist_id: row.anilist_id,
            media_kind,
            progress_delta: row.progress_delta,
            episode_duration_minutes: row.episode_duration_minutes,
            occurred_at: format_utc(parse_timestamp(&row.occurred_at)?),
        })
    }
}
