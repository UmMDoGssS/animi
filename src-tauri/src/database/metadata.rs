use sqlx::{FromRow, SqlitePool};

use crate::domain::{MediaKind, format_utc};

use super::{AiringDataRecord, DatabaseError, MediaSnapshotRecord, MediaTag, parse_timestamp};

#[derive(Debug, Clone)]
pub struct MediaRepository {
    pool: SqlitePool,
}

impl MediaRepository {
    pub(super) fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn upsert_snapshot(
        &self,
        snapshot: &MediaSnapshotRecord,
    ) -> Result<(), DatabaseError> {
        self.upsert_snapshots(std::slice::from_ref(snapshot))
            .await?;
        Ok(())
    }

    pub async fn upsert_snapshots(
        &self,
        snapshots: &[MediaSnapshotRecord],
    ) -> Result<(), DatabaseError> {
        let prepared = snapshots
            .iter()
            .map(|snapshot| {
                validate_snapshot(snapshot)?;
                Ok((
                    snapshot,
                    serde_json::to_string(&snapshot.genres)?,
                    serde_json::to_string(&snapshot.tags)?,
                ))
            })
            .collect::<Result<Vec<_>, DatabaseError>>()?;
        let mut transaction = self.pool.begin().await?;
        for (snapshot, genres, tags) in prepared {
            sqlx::query(
                "INSERT INTO media_snapshots (\
                    anilist_id, media_kind, title, cover_url, media_format, genres_json, tags_json, \
                    media_status, total_units, volume_total, duration_minutes, release_date, cached_at\
                 ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?) \
                 ON CONFLICT(anilist_id, media_kind) DO UPDATE SET \
                    title = excluded.title, cover_url = excluded.cover_url, \
                    media_format = excluded.media_format, genres_json = excluded.genres_json, \
                    tags_json = excluded.tags_json, media_status = excluded.media_status, \
                    total_units = excluded.total_units, \
                    volume_total = excluded.volume_total, \
                    duration_minutes = excluded.duration_minutes, \
                    release_date = excluded.release_date, cached_at = excluded.cached_at",
            )
            .bind(snapshot.anilist_id)
            .bind(snapshot.media_kind.as_str())
            .bind(&snapshot.title)
            .bind(&snapshot.cover_url)
            .bind(&snapshot.media_format)
            .bind(genres)
            .bind(tags)
            .bind(&snapshot.media_status)
            .bind(snapshot.total_units)
            .bind(snapshot.volume_total)
            .bind(snapshot.duration_minutes)
            .bind(&snapshot.release_date)
            .bind(format_utc(snapshot.cached_at))
            .execute(&mut *transaction)
            .await?;
        }
        transaction.commit().await?;
        Ok(())
    }

    pub async fn get_snapshot(
        &self,
        anilist_id: i64,
        media_kind: MediaKind,
    ) -> Result<Option<MediaSnapshotRecord>, DatabaseError> {
        let row = sqlx::query_as::<_, SnapshotRow>(
            "SELECT anilist_id, media_kind, title, cover_url, media_format, genres_json, tags_json, \
             media_status, total_units, volume_total, duration_minutes, release_date, cached_at FROM media_snapshots \
             WHERE anilist_id = ? AND media_kind = ?",
        )
        .bind(anilist_id)
        .bind(media_kind.as_str())
        .fetch_optional(&self.pool)
        .await?;
        row.map(MediaSnapshotRecord::try_from).transpose()
    }

    pub async fn upsert_airing(&self, airing: &AiringDataRecord) -> Result<(), DatabaseError> {
        sqlx::query(
            "INSERT INTO airing_data (\
                anilist_id, media_kind, episode_number, airing_at, fetched_at\
             ) VALUES (?, ?, ?, ?, ?) \
             ON CONFLICT(anilist_id, media_kind, episode_number) DO UPDATE SET \
                airing_at = excluded.airing_at, fetched_at = excluded.fetched_at",
        )
        .bind(airing.anilist_id)
        .bind(airing.media_kind.as_str())
        .bind(airing.episode_number)
        .bind(format_utc(airing.airing_at))
        .bind(format_utc(airing.fetched_at))
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn replace_airing(
        &self,
        anilist_id: i64,
        media_kind: MediaKind,
        schedules: &[AiringDataRecord],
    ) -> Result<(), DatabaseError> {
        if schedules
            .iter()
            .any(|schedule| schedule.anilist_id != anilist_id || schedule.media_kind != media_kind)
        {
            return Err(DatabaseError::CorruptData(
                "Airing replacement contains a mismatched media identifier.".to_owned(),
            ));
        }
        let mut transaction = self.pool.begin().await?;
        sqlx::query("DELETE FROM airing_data WHERE anilist_id = ? AND media_kind = ?")
            .bind(anilist_id)
            .bind(media_kind.as_str())
            .execute(&mut *transaction)
            .await?;
        for airing in schedules {
            sqlx::query(
                "INSERT INTO airing_data (\
                    anilist_id, media_kind, episode_number, airing_at, fetched_at\
                 ) VALUES (?, ?, ?, ?, ?)",
            )
            .bind(airing.anilist_id)
            .bind(airing.media_kind.as_str())
            .bind(airing.episode_number)
            .bind(format_utc(airing.airing_at))
            .bind(format_utc(airing.fetched_at))
            .execute(&mut *transaction)
            .await?;
        }
        transaction.commit().await?;
        Ok(())
    }

    pub async fn list_airing(
        &self,
        anilist_id: i64,
        media_kind: MediaKind,
    ) -> Result<Vec<AiringDataRecord>, DatabaseError> {
        let rows = sqlx::query_as::<_, AiringRow>(
            "SELECT anilist_id, media_kind, episode_number, airing_at, fetched_at \
             FROM airing_data WHERE anilist_id = ? AND media_kind = ? \
             ORDER BY episode_number ASC",
        )
        .bind(anilist_id)
        .bind(media_kind.as_str())
        .fetch_all(&self.pool)
        .await?;
        rows.into_iter().map(AiringDataRecord::try_from).collect()
    }
}

fn validate_snapshot(snapshot: &MediaSnapshotRecord) -> Result<(), DatabaseError> {
    if snapshot
        .total_units
        .is_some_and(|total| !(0..=i64::from(i32::MAX)).contains(&total))
        || snapshot
            .volume_total
            .is_some_and(|total| !(0..=i64::from(i32::MAX)).contains(&total))
        || snapshot
            .duration_minutes
            .is_some_and(|duration| duration <= 0)
    {
        return Err(DatabaseError::CorruptData(
            "A media snapshot contains an invalid total or duration.".to_owned(),
        ));
    }
    match snapshot.media_kind {
        MediaKind::Anime if snapshot.volume_total.is_some() => Err(DatabaseError::CorruptData(
            "An anime snapshot cannot contain a volume total.".to_owned(),
        )),
        MediaKind::Manga if snapshot.duration_minutes.is_some() => Err(DatabaseError::CorruptData(
            "A manga snapshot cannot contain an episode duration.".to_owned(),
        )),
        _ => Ok(()),
    }
}

#[derive(Debug, FromRow)]
struct SnapshotRow {
    anilist_id: i64,
    media_kind: String,
    title: String,
    cover_url: Option<String>,
    media_format: Option<String>,
    genres_json: String,
    tags_json: String,
    media_status: Option<String>,
    total_units: Option<i64>,
    volume_total: Option<i64>,
    duration_minutes: Option<i64>,
    release_date: Option<String>,
    cached_at: String,
}

impl TryFrom<SnapshotRow> for MediaSnapshotRecord {
    type Error = DatabaseError;

    fn try_from(row: SnapshotRow) -> Result<Self, Self::Error> {
        let media_kind = MediaKind::parse(&row.media_kind)
            .map_err(|error| DatabaseError::CorruptData(error.message))?;
        let genres = serde_json::from_str(&row.genres_json)?;
        let tags: Vec<MediaTag> = serde_json::from_str(&row.tags_json)?;
        Ok(Self {
            anilist_id: row.anilist_id,
            media_kind,
            title: row.title,
            cover_url: row.cover_url,
            media_format: row.media_format,
            genres,
            tags,
            media_status: row.media_status,
            total_units: row.total_units,
            volume_total: row.volume_total,
            duration_minutes: row.duration_minutes,
            release_date: row.release_date.map(validate_release_date).transpose()?,
            cached_at: parse_timestamp(&row.cached_at)?,
        })
    }
}

fn validate_release_date(value: String) -> Result<String, DatabaseError> {
    let valid = match value.len() {
        4 => value.parse::<i32>().is_ok_and(|year| year > 0),
        7 => chrono::NaiveDate::parse_from_str(&format!("{value}-01"), "%Y-%m-%d").is_ok(),
        10 => chrono::NaiveDate::parse_from_str(&value, "%Y-%m-%d").is_ok(),
        _ => false,
    };
    if valid {
        Ok(value)
    } else {
        Err(DatabaseError::CorruptData(
            "A cached media release date is invalid.".to_owned(),
        ))
    }
}

#[derive(Debug, FromRow)]
struct AiringRow {
    anilist_id: i64,
    media_kind: String,
    episode_number: i64,
    airing_at: String,
    fetched_at: String,
}

impl TryFrom<AiringRow> for AiringDataRecord {
    type Error = DatabaseError;

    fn try_from(row: AiringRow) -> Result<Self, Self::Error> {
        let media_kind = MediaKind::parse(&row.media_kind)
            .map_err(|error| DatabaseError::CorruptData(error.message))?;
        Ok(Self {
            anilist_id: row.anilist_id,
            media_kind,
            episode_number: row.episode_number,
            airing_at: parse_timestamp(&row.airing_at)?,
            fetched_at: parse_timestamp(&row.fetched_at)?,
        })
    }
}
