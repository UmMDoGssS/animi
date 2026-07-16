mod cache;
mod error;
mod library;
mod metadata;
mod records;
mod settings;

use std::path::{Path, PathBuf};

use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::{ConnectOptions, SqlitePool};

pub use cache::CacheRepository;
pub use error::DatabaseError;
pub use library::LibraryRepository;
pub use metadata::MediaRepository;
pub use records::{AiringDataRecord, CacheRecord, MediaSnapshotRecord, MediaTag, SettingRecord};
pub use settings::SettingsRepository;

use crate::domain::CommandError;

pub const DATABASE_FILE_NAME: &str = "animi.sqlite3";
static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("./migrations");

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

        MIGRATOR.run(&pool).await?;
        Ok(Self { pool })
    }

    pub fn library(&self) -> LibraryRepository {
        LibraryRepository::new(self.pool.clone())
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

#[cfg(test)]
mod tests {
    use tempfile::tempdir;

    use super::{DATABASE_FILE_NAME, Database, DatabasePaths};

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
}
