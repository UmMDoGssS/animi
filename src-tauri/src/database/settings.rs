use sqlx::{FromRow, SqlitePool};

use crate::domain::format_utc;

use super::{DatabaseError, SettingRecord, parse_timestamp};

#[derive(Debug, Clone)]
pub struct SettingsRepository {
    pool: SqlitePool,
}

impl SettingsRepository {
    pub(super) fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn upsert_many(&self, settings: &[SettingRecord]) -> Result<(), DatabaseError> {
        let mut transaction = self.pool.begin().await?;
        for setting in settings {
            let value = serde_json::to_string(&setting.value)?;
            sqlx::query(
                "INSERT INTO settings (key, value_json, updated_at) VALUES (?, ?, ?) \
                 ON CONFLICT(key) DO UPDATE SET \
                    value_json = excluded.value_json, updated_at = excluded.updated_at",
            )
            .bind(&setting.key)
            .bind(value)
            .bind(format_utc(setting.updated_at))
            .execute(&mut *transaction)
            .await?;
        }
        transaction.commit().await?;
        Ok(())
    }

    pub async fn get(&self, key: &str) -> Result<Option<SettingRecord>, DatabaseError> {
        let row = sqlx::query_as::<_, SettingRow>(
            "SELECT key, value_json, updated_at FROM settings WHERE key = ?",
        )
        .bind(key)
        .fetch_optional(&self.pool)
        .await?;
        row.map(SettingRecord::try_from).transpose()
    }
}

#[derive(Debug, FromRow)]
struct SettingRow {
    key: String,
    value_json: String,
    updated_at: String,
}

impl TryFrom<SettingRow> for SettingRecord {
    type Error = DatabaseError;

    fn try_from(row: SettingRow) -> Result<Self, Self::Error> {
        Ok(Self {
            key: row.key,
            value: serde_json::from_str(&row.value_json)?,
            updated_at: parse_timestamp(&row.updated_at)?,
        })
    }
}
