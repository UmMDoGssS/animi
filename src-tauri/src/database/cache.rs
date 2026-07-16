use sqlx::{FromRow, SqlitePool};

use crate::domain::format_utc;

use super::{CacheRecord, DatabaseError, parse_timestamp};

#[derive(Debug, Clone)]
pub struct CacheRepository {
    pool: SqlitePool,
}

impl CacheRepository {
    pub(super) fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn upsert(&self, cache: &CacheRecord) -> Result<(), DatabaseError> {
        let payload = serde_json::to_string(&cache.payload)?;
        sqlx::query(
            "INSERT INTO response_caches (cache_key, payload_json, fetched_at, expires_at) \
             VALUES (?, ?, ?, ?) ON CONFLICT(cache_key) DO UPDATE SET \
                payload_json = excluded.payload_json, fetched_at = excluded.fetched_at, \
                expires_at = excluded.expires_at",
        )
        .bind(&cache.cache_key)
        .bind(payload)
        .bind(format_utc(cache.fetched_at))
        .bind(format_utc(cache.expires_at))
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn get(&self, cache_key: &str) -> Result<Option<CacheRecord>, DatabaseError> {
        let row = sqlx::query_as::<_, CacheRow>(
            "SELECT cache_key, payload_json, fetched_at, expires_at \
             FROM response_caches WHERE cache_key = ?",
        )
        .bind(cache_key)
        .fetch_optional(&self.pool)
        .await?;
        row.map(CacheRecord::try_from).transpose()
    }
}

#[derive(Debug, FromRow)]
struct CacheRow {
    cache_key: String,
    payload_json: String,
    fetched_at: String,
    expires_at: String,
}

impl TryFrom<CacheRow> for CacheRecord {
    type Error = DatabaseError;

    fn try_from(row: CacheRow) -> Result<Self, Self::Error> {
        Ok(Self {
            cache_key: row.cache_key,
            payload: serde_json::from_str(&row.payload_json)?,
            fetched_at: parse_timestamp(&row.fetched_at)?,
            expires_at: parse_timestamp(&row.expires_at)?,
        })
    }
}
