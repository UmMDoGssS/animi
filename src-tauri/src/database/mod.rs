mod activity;
mod backups;
mod cache;
mod dashboard;
mod error;
mod library;
mod metadata;
mod records;
mod settings;

use std::path::{Path, PathBuf};

use sqlx::migrate::MigrateError;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::{ConnectOptions, SqlitePool};

pub use activity::ActivityRepository;
pub use backups::BackupRepository;
pub(crate) use backups::{
    BackupLibraryEntryRecord, BackupSettingRecord, BackupSnapshotRecord, BackupWatchActivityRecord,
};
pub use cache::CacheRepository;
pub use dashboard::DashboardRepository;
pub use error::DatabaseError;
pub use library::LibraryRepository;
pub use metadata::MediaRepository;
pub use records::{AiringDataRecord, CacheRecord, MediaSnapshotRecord, MediaTag, SettingRecord};
pub use settings::SettingsRepository;

use crate::domain::CommandError;

pub const DATABASE_FILE_NAME: &str = "animi.sqlite3";
static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("./migrations");

// Migration 2 briefly contained the activity-progress change that now belongs to migration 3.
// A database created by that build already has the final schema but records the combined checksum.
// Keep this narrow bridge so both known migration histories converge without replaying the ALTER.
const COMBINED_ACTIVITY_MIGRATION_CHECKSUM_HEX: &str = "6302b9c1c2fa2ccfd27724521b8b16f6d8d8fb7714f2ea3c6f69bd4cadf461adb71b2b2807cb69faa755806f68b32b77";

fn parse_timestamp(value: &str) -> Result<chrono::DateTime<chrono::Utc>, DatabaseError> {
    chrono::DateTime::parse_from_rfc3339(value)
        .map(|timestamp| timestamp.with_timezone(&chrono::Utc))
        .map_err(|error| DatabaseError::CorruptData(error.to_string()))
}

/// Resolves database storage from an injected root. Production injects Tauri's
/// app-data directory; every automated test injects a temporary directory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DatabasePaths {
    root: PathBuf,
    database: PathBuf,
}

impl DatabasePaths {
    pub fn from_root(root: impl Into<PathBuf>) -> Result<Self, CommandError> {
        let root = root.into();
        if root.as_os_str().is_empty() {
            return Err(CommandError::new(
                "INVALID_STORAGE_PATH",
                "The application storage path is empty.",
                false,
            ));
        }

        let database = root.join(DATABASE_FILE_NAME);
        Ok(Self { root, database })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn database(&self) -> &Path {
        &self.database
    }
}

#[derive(Debug, Clone)]
pub struct Database {
    pool: SqlitePool,
    storage_root: PathBuf,
}

impl Database {
    pub async fn connect(paths: &DatabasePaths) -> Result<Self, DatabaseError> {
        tokio::fs::create_dir_all(paths.root()).await?;
        let options = SqliteConnectOptions::new()
            .filename(paths.database())
            .create_if_missing(true)
            .foreign_keys(true)
            .disable_statement_logging();
        let pool = SqlitePoolOptions::new()
            .max_connections(4)
            .after_connect(|connection, _metadata| {
                Box::pin(async move {
                    sqlx::query("PRAGMA foreign_keys = ON")
                        .execute(connection)
                        .await?;
                    Ok(())
                })
            })
            .connect_with(options)
            .await?;

        match MIGRATOR.run(&pool).await {
            Ok(()) => {}
            Err(error @ MigrateError::VersionMismatch(2)) => {
                if !reconcile_combined_activity_migration(&pool).await? {
                    return Err(error.into());
                }
                MIGRATOR.run(&pool).await?;
            }
            Err(error) => return Err(error.into()),
        }
        Ok(Self {
            pool,
            storage_root: paths.root().to_owned(),
        })
    }

    pub fn library(&self) -> LibraryRepository {
        LibraryRepository::new(self.pool.clone())
    }

    pub fn activity(&self) -> ActivityRepository {
        ActivityRepository::new(self.pool.clone())
    }

    pub fn backups(&self) -> BackupRepository {
        BackupRepository::new(self.pool.clone())
    }

    pub fn dashboard(&self) -> DashboardRepository {
        DashboardRepository::new(self.pool.clone())
    }

    pub fn media(&self) -> MediaRepository {
        MediaRepository::new(self.pool.clone())
    }

    pub fn settings(&self) -> SettingsRepository {
        SettingsRepository::new(self.pool.clone())
    }

    pub fn cache(&self) -> CacheRepository {
        CacheRepository::new(self.pool.clone())
    }

    pub(crate) fn storage_root(&self) -> &Path {
        &self.storage_root
    }

    pub async fn foreign_keys_enabled(&self) -> Result<bool, DatabaseError> {
        let enabled = sqlx::query_scalar::<_, i64>("PRAGMA foreign_keys")
            .fetch_one(&self.pool)
            .await?;
        Ok(enabled == 1)
    }

    pub async fn close(self) {
        self.pool.close().await;
    }
}

async fn reconcile_combined_activity_migration(pool: &SqlitePool) -> Result<bool, DatabaseError> {
    let applied = sqlx::query_as::<_, (String, bool)>(
        "SELECT lower(hex(checksum)), success \
         FROM _sqlx_migrations WHERE version = 2",
    )
    .fetch_optional(pool)
    .await?;
    let Some((checksum, true)) = applied else {
        return Ok(false);
    };
    if checksum != COMBINED_ACTIVITY_MIGRATION_CHECKSUM_HEX {
        return Ok(false);
    }

    let activity_progress_column = sqlx::query_as::<_, (String, bool, Option<String>)>(
        "SELECT type, \"notnull\", dflt_value \
         FROM pragma_table_info('library_entries') WHERE name = 'activity_progress'",
    )
    .fetch_optional(pool)
    .await?;
    if activity_progress_column != Some(("INTEGER".to_owned(), true, Some("0".to_owned()))) {
        return Ok(false);
    }

    let migration_three_exists = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM _sqlx_migrations WHERE version = 3)",
    )
    .fetch_one(pool)
    .await?;
    if migration_three_exists {
        return Ok(false);
    }

    let canonical_migration_two = MIGRATOR
        .iter()
        .find(|migration| migration.version == 2)
        .ok_or_else(|| DatabaseError::CorruptData("migration 2 is not embedded".to_owned()))?;
    let activity_progress_migration = MIGRATOR
        .iter()
        .find(|migration| migration.version == 3)
        .ok_or_else(|| DatabaseError::CorruptData("migration 3 is not embedded".to_owned()))?;

    let mut transaction = pool.begin().await?;
    let updated = sqlx::query(
        "UPDATE _sqlx_migrations SET checksum = ? \
         WHERE version = 2 AND lower(hex(checksum)) = ? AND success = TRUE",
    )
    .bind(canonical_migration_two.checksum.as_ref())
    .bind(COMBINED_ACTIVITY_MIGRATION_CHECKSUM_HEX)
    .execute(&mut *transaction)
    .await?;
    if updated.rows_affected() != 1 {
        transaction.rollback().await?;
        return Ok(false);
    }

    sqlx::query(
        "INSERT INTO _sqlx_migrations (\
            version, description, success, checksum, execution_time\
         ) VALUES (?, ?, TRUE, ?, 0)",
    )
    .bind(activity_progress_migration.version)
    .bind(activity_progress_migration.description.as_ref())
    .bind(activity_progress_migration.checksum.as_ref())
    .execute(&mut *transaction)
    .await?;
    transaction.commit().await?;

    Ok(true)
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use sqlx::ConnectOptions;
    use sqlx::migrate::MigrateError;
    use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
    use tempfile::tempdir;

    use super::{
        COMBINED_ACTIVITY_MIGRATION_CHECKSUM_HEX, DATABASE_FILE_NAME, Database, DatabaseError,
        DatabasePaths, MIGRATOR,
    };

    fn v0001_fixture() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/database/v0001.sqlite3")
    }

    fn decode_checksum(checksum: &str) -> Vec<u8> {
        (0..checksum.len())
            .step_by(2)
            .map(|index| {
                u8::from_str_radix(&checksum[index..index + 2], 16)
                    .expect("test checksum should be hexadecimal")
            })
            .collect()
    }

    fn embedded_checksum(version: i64) -> String {
        let migration = MIGRATOR
            .iter()
            .find(|migration| migration.version == version)
            .expect("embedded migration should exist");
        migration
            .checksum
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect()
    }

    async fn prepare_combined_activity_history(
        paths: &DatabasePaths,
        checksum: &[u8],
        include_activity_progress: bool,
    ) {
        std::fs::copy(v0001_fixture(), paths.database()).expect("copy v0001 fixture");
        let options = SqliteConnectOptions::new()
            .filename(paths.database())
            .foreign_keys(true)
            .disable_statement_logging();
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(options)
            .await
            .expect("fixture database");

        sqlx::raw_sql(include_str!("../../migrations/0002_watch_activity.sql"))
            .execute(&pool)
            .await
            .expect("migration 2 schema");
        if include_activity_progress {
            sqlx::raw_sql(include_str!("../../migrations/0003_activity_progress.sql"))
                .execute(&pool)
                .await
                .expect("combined activity-progress schema");
        }
        sqlx::query(
            "INSERT INTO _sqlx_migrations (\
                version, description, success, checksum, execution_time\
             ) VALUES (2, 'watch activity', TRUE, ?, 0)",
        )
        .bind(checksum)
        .execute(&pool)
        .await
        .expect("combined migration ledger row");
        pool.close().await;
    }

    #[test]
    fn test_database_is_rooted_only_in_a_temporary_directory() {
        let temporary_root = tempdir().expect("a temporary test directory should be available");
        let paths = DatabasePaths::from_root(temporary_root.path())
            .expect("the temporary root should be accepted");

        assert!(paths.database().starts_with(temporary_root.path()));
        assert_eq!(
            paths.database().file_name(),
            Some(DATABASE_FILE_NAME.as_ref())
        );
    }

    #[test]
    fn empty_storage_root_is_rejected() {
        let error = DatabasePaths::from_root("")
            .expect_err("an empty root must never resolve to the current directory");

        assert_eq!(error.code, "INVALID_STORAGE_PATH");
    }

    #[tokio::test]
    async fn fresh_database_runs_migrations_and_enables_foreign_keys() {
        let root = tempdir().expect("temporary database root");
        let paths = DatabasePaths::from_root(root.path()).expect("valid database paths");
        let database = Database::connect(&paths).await.expect("migrated database");

        assert!(
            database
                .foreign_keys_enabled()
                .await
                .expect("pragma result")
        );
        assert!(paths.database().exists());
    }

    #[tokio::test]
    async fn combined_activity_migration_history_is_canonicalized_once() {
        let root = tempdir().expect("temporary database root");
        let paths = DatabasePaths::from_root(root.path()).expect("valid database paths");
        let combined_checksum = decode_checksum(COMBINED_ACTIVITY_MIGRATION_CHECKSUM_HEX);
        prepare_combined_activity_history(&paths, &combined_checksum, true).await;

        let database = Database::connect(&paths)
            .await
            .expect("known combined migration should be reconciled");
        let applied = sqlx::query_as::<_, (i64, String)>(
            "SELECT version, lower(hex(checksum)) \
             FROM _sqlx_migrations WHERE version IN (2, 3) ORDER BY version",
        )
        .fetch_all(&database.pool)
        .await
        .expect("canonical migration ledger");
        assert_eq!(
            applied,
            vec![(2, embedded_checksum(2)), (3, embedded_checksum(3))]
        );
        assert_eq!(
            sqlx::query_scalar::<_, i64>(
                "SELECT activity_progress FROM library_entries \
                 WHERE anilist_id = 101 AND media_kind = 'ANIME'",
            )
            .fetch_one(&database.pool)
            .await
            .expect("preserved activity baseline"),
            3
        );

        database.close().await;
        Database::connect(&paths)
            .await
            .expect("canonicalized history should reopen cleanly");
    }

    #[tokio::test]
    async fn unknown_migration_two_checksum_is_rejected_without_rewriting_history() {
        let root = tempdir().expect("temporary database root");
        let paths = DatabasePaths::from_root(root.path()).expect("valid database paths");
        let unknown_checksum = vec![0_u8; 48];
        prepare_combined_activity_history(&paths, &unknown_checksum, true).await;

        let error = Database::connect(&paths)
            .await
            .expect_err("unknown migration history must stay rejected");
        assert!(matches!(
            error,
            DatabaseError::Migration(MigrateError::VersionMismatch(2))
        ));

        let options = SqliteConnectOptions::new()
            .filename(paths.database())
            .disable_statement_logging();
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(options)
            .await
            .expect("reopen rejected history");
        assert_eq!(
            sqlx::query_scalar::<_, String>(
                "SELECT lower(hex(checksum)) FROM _sqlx_migrations WHERE version = 2",
            )
            .fetch_one(&pool)
            .await
            .expect("unchanged migration 2 checksum"),
            "00".repeat(48)
        );
        assert_eq!(
            sqlx::query_scalar::<_, i64>(
                "SELECT COUNT(*) FROM _sqlx_migrations WHERE version = 3",
            )
            .fetch_one(&pool)
            .await
            .expect("no migration 3 ledger row"),
            0
        );
    }

    #[tokio::test]
    async fn combined_checksum_without_its_schema_is_rejected_atomically() {
        let root = tempdir().expect("temporary database root");
        let paths = DatabasePaths::from_root(root.path()).expect("valid database paths");
        let combined_checksum = decode_checksum(COMBINED_ACTIVITY_MIGRATION_CHECKSUM_HEX);
        prepare_combined_activity_history(&paths, &combined_checksum, false).await;

        let error = Database::connect(&paths)
            .await
            .expect_err("known checksum must still match its expected schema");
        assert!(matches!(
            error,
            DatabaseError::Migration(MigrateError::VersionMismatch(2))
        ));

        let options = SqliteConnectOptions::new()
            .filename(paths.database())
            .disable_statement_logging();
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(options)
            .await
            .expect("reopen rejected history");
        assert_eq!(
            sqlx::query_scalar::<_, String>(
                "SELECT lower(hex(checksum)) FROM _sqlx_migrations WHERE version = 2",
            )
            .fetch_one(&pool)
            .await
            .expect("unchanged combined checksum"),
            COMBINED_ACTIVITY_MIGRATION_CHECKSUM_HEX
        );
        assert_eq!(
            sqlx::query_scalar::<_, i64>(
                "SELECT COUNT(*) FROM pragma_table_info('library_entries') \
                 WHERE name = 'activity_progress'",
            )
            .fetch_one(&pool)
            .await
            .expect("missing activity-progress column"),
            0
        );
    }
}
