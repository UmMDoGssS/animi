use std::io::Write;
use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tempfile::NamedTempFile;

use crate::database::{
    BackupLibraryEntryRecord, BackupSettingRecord, BackupSnapshotRecord, BackupWatchActivityRecord,
    Database, DatabaseError,
};
use crate::domain::{CommandError, LibraryStatus, MediaKind, format_utc};

mod import;

#[doc(hidden)]
pub use import::ImportSessionStore;
pub use import::{
    ClearLocalDataRequest, ClearLocalDataResponse, ImportBackupRequest, ImportBackupResponse,
    ImportChangeSummary, ImportPreviewResponse, ImportValidationIssue, SettingsOverview,
};
pub(crate) use import::{
    clear_local_data, get_settings_overview, import_backup, preview_import_from_path,
};

/// Every export currently uses backup schema v1.
pub const CURRENT_SCHEMA_VERSION: u32 = 1;

/// The P6-M3 validator will accept exactly these versions. No import path is
/// exposed until that validator enforces the semantic rules documented below.
pub const SUPPORTED_IMPORT_SCHEMA_VERSIONS: &[u32] = &[1];

/// Stable v1 backup envelope.
///
/// The format contains user-owned settings, authoritative library state, and
/// watch activity. It deliberately excludes AniList metadata snapshots,
/// airing rows, response caches, downloaded images, and database internals.
/// `activityProgress` preserves the per-entry activity high-water mark so a
/// corrected and restored progress value cannot be counted twice later.
/// `eventId` is the portable identity used to make future imports idempotent:
/// an identical existing event is a no-op, while a reused ID with a different
/// payload is invalid.
/// All timestamps are canonical UTC RFC 3339 strings with millisecond precision.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BackupEnvelopeV1 {
    pub schema_version: u32,
    pub exported_at: String,
    pub settings: Vec<BackupSettingV1>,
    pub library_entries: Vec<BackupLibraryEntryV1>,
    pub watch_activity: Vec<BackupWatchActivityV1>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BackupSettingV1 {
    pub key: String,
    pub value: Value,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BackupLibraryEntryV1 {
    pub anilist_id: i64,
    pub media_kind: MediaKind,
    pub status: LibraryStatus,
    pub progress: i64,
    pub activity_progress: i64,
    pub score: Option<f64>,
    pub notes: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BackupWatchActivityV1 {
    pub event_id: String,
    pub anilist_id: i64,
    pub media_kind: MediaKind,
    pub progress_delta: i64,
    pub episode_duration_minutes: Option<i64>,
    pub occurred_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportBackupResponse {
    pub schema_version: u32,
    pub exported_at: String,
    pub settings_count: usize,
    pub library_entry_count: usize,
    pub watch_activity_count: usize,
}

#[derive(Debug, thiserror::Error)]
enum BackupError {
    #[error("backup database snapshot failed: {0}")]
    Database(#[from] DatabaseError),
    #[error("backup serialization failed: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("the backup destination is not an absolute file path")]
    InvalidDestination,
    #[error("the backup destination overlaps Animi's live storage")]
    ProtectedDestination,
    #[error("backup file write failed: {0}")]
    Write(#[from] std::io::Error),
    #[error("backup file task failed: {0}")]
    FileTask(#[from] tokio::task::JoinError),
}

impl BackupError {
    fn into_command_error(self) -> CommandError {
        match self {
            Self::Database(error) => error.into_command_error(),
            Self::Serialization(_) => CommandError::new(
                "BACKUP_SERIALIZATION_FAILED",
                "Animi could not prepare the backup file.",
                false,
            ),
            Self::InvalidDestination => CommandError::new(
                "BACKUP_PATH_INVALID",
                "Choose a valid backup file location.",
                false,
            ),
            Self::ProtectedDestination => CommandError::new(
                "BACKUP_PATH_PROTECTED",
                "Choose a location outside Animi's live application storage.",
                false,
            ),
            Self::Write(_) | Self::FileTask(_) => CommandError::new(
                "BACKUP_WRITE_FAILED",
                "Animi could not write the backup file. Choose another location and try again.",
                true,
            ),
        }
    }
}

pub(crate) async fn export_backup_to_path(
    database: &Database,
    destination_path: PathBuf,
) -> Result<ExportBackupResponse, CommandError> {
    export_backup_to_path_at(database, destination_path, Utc::now()).await
}

pub(crate) async fn export_backup_to_path_at(
    database: &Database,
    destination_path: PathBuf,
    exported_at: DateTime<Utc>,
) -> Result<ExportBackupResponse, CommandError> {
    validate_destination(&destination_path, database.storage_root())
        .map_err(BackupError::into_command_error)?;

    let snapshot = database
        .backups()
        .snapshot()
        .await
        .map_err(BackupError::from)
        .map_err(BackupError::into_command_error)?;
    let envelope = BackupEnvelopeV1::from_snapshot(snapshot, exported_at);
    let response = ExportBackupResponse {
        schema_version: envelope.schema_version,
        exported_at: envelope.exported_at.clone(),
        settings_count: envelope.settings.len(),
        library_entry_count: envelope.library_entries.len(),
        watch_activity_count: envelope.watch_activity.len(),
    };

    let destination = destination_path;
    let write_result =
        tokio::task::spawn_blocking(move || write_atomically(&destination, &envelope))
            .await
            .map_err(BackupError::from)
            .map_err(BackupError::into_command_error)?;
    write_result.map_err(BackupError::into_command_error)?;

    Ok(response)
}

impl BackupEnvelopeV1 {
    fn from_snapshot(snapshot: BackupSnapshotRecord, exported_at: DateTime<Utc>) -> Self {
        Self {
            schema_version: CURRENT_SCHEMA_VERSION,
            exported_at: format_utc(exported_at),
            settings: snapshot
                .settings
                .into_iter()
                .map(BackupSettingV1::from)
                .collect(),
            library_entries: snapshot
                .library_entries
                .into_iter()
                .map(BackupLibraryEntryV1::from)
                .collect(),
            watch_activity: snapshot
                .watch_activity
                .into_iter()
                .map(BackupWatchActivityV1::from)
                .collect(),
        }
    }
}

impl From<BackupSettingRecord> for BackupSettingV1 {
    fn from(record: BackupSettingRecord) -> Self {
        Self {
            key: record.key,
            value: record.value,
            updated_at: format_utc(record.updated_at),
        }
    }
}

impl From<BackupLibraryEntryRecord> for BackupLibraryEntryV1 {
    fn from(record: BackupLibraryEntryRecord) -> Self {
        Self {
            anilist_id: record.anilist_id,
            media_kind: record.media_kind,
            status: record.status,
            progress: record.progress,
            activity_progress: record.activity_progress,
            score: record.score,
            notes: record.notes,
            created_at: format_utc(record.created_at),
            updated_at: format_utc(record.updated_at),
        }
    }
}

impl From<BackupWatchActivityRecord> for BackupWatchActivityV1 {
    fn from(record: BackupWatchActivityRecord) -> Self {
        Self {
            event_id: record.event_id.to_string(),
            anilist_id: record.anilist_id,
            media_kind: record.media_kind,
            progress_delta: record.progress_delta,
            episode_duration_minutes: record.episode_duration_minutes,
            occurred_at: format_utc(record.occurred_at),
        }
    }
}

fn validate_destination(destination: &Path, protected_root: &Path) -> Result<(), BackupError> {
    let has_file_name = destination.file_name().is_some_and(|name| !name.is_empty());
    let has_parent = destination
        .parent()
        .is_some_and(|parent| !parent.as_os_str().is_empty());
    if !destination.is_absolute() || !has_file_name || !has_parent {
        return Err(BackupError::InvalidDestination);
    }
    if is_within_root(destination, protected_root) {
        return Err(BackupError::ProtectedDestination);
    }
    Ok(())
}

#[cfg(windows)]
fn is_within_root(destination: &Path, root: &Path) -> bool {
    let destination = destination
        .as_os_str()
        .to_string_lossy()
        .replace('/', "\\")
        .to_lowercase();
    let root = root
        .as_os_str()
        .to_string_lossy()
        .replace('/', "\\")
        .trim_end_matches('\\')
        .to_lowercase();
    destination == root || destination.starts_with(&format!("{root}\\"))
}

#[cfg(not(windows))]
fn is_within_root(destination: &Path, root: &Path) -> bool {
    destination.starts_with(root)
}

fn write_atomically(destination: &Path, envelope: &BackupEnvelopeV1) -> Result<(), BackupError> {
    write_atomically_with(
        destination,
        |file| {
            serde_json::to_writer_pretty(&mut *file, envelope)
                .map_err(classify_serialization_error)?;
            file.write_all(b"\n")?;
            Ok(())
        },
        |temporary, destination| {
            temporary
                .persist(destination)
                .map_err(|error| error.error)?;
            Ok(())
        },
    )
}

fn write_atomically_with(
    destination: &Path,
    write: impl FnOnce(&mut std::fs::File) -> Result<(), BackupError>,
    persist: impl FnOnce(NamedTempFile, &Path) -> Result<(), BackupError>,
) -> Result<(), BackupError> {
    let parent = destination
        .parent()
        .ok_or_else(|| std::io::Error::other("backup destination has no parent directory"))?;
    let mut temporary = NamedTempFile::new_in(parent)?;
    write(temporary.as_file_mut())?;
    temporary.as_file_mut().sync_all()?;
    persist(temporary, destination)
}

fn classify_serialization_error(error: serde_json::Error) -> BackupError {
    match error.io_error_kind() {
        Some(kind) => BackupError::Write(std::io::Error::new(kind, error)),
        None => BackupError::Serialization(error),
    }
}

#[cfg(test)]
mod tests {
    use std::io::Write as _;

    use chrono::{DateTime, Utc};
    use serde_json::{Value, json};
    use tempfile::tempdir;
    use uuid::Uuid;

    use crate::database::{
        AiringDataRecord, CacheRecord, Database, DatabasePaths, MediaSnapshotRecord, MediaTag,
        SettingRecord,
    };
    use crate::domain::{LibraryStatus, MediaKind, UpsertLibraryEntryRequest};

    use super::{
        CURRENT_SCHEMA_VERSION, SUPPORTED_IMPORT_SCHEMA_VERSIONS, export_backup_to_path_at,
        write_atomically_with,
    };

    fn timestamp(value: &str) -> DateTime<Utc> {
        value.parse().expect("valid test timestamp")
    }

    async fn test_database() -> (tempfile::TempDir, Database) {
        let root = tempdir().expect("temporary database root");
        let paths = DatabasePaths::from_root(root.path()).expect("valid database paths");
        let database = Database::connect(&paths).await.expect("test database");
        (root, database)
    }

    fn request(progress: i64, notes: &str) -> UpsertLibraryEntryRequest {
        UpsertLibraryEntryRequest {
            anilist_id: 101,
            media_kind: MediaKind::Anime,
            status: LibraryStatus::Watching,
            progress,
            score: Some(8.5),
            notes: Some(notes.to_owned()),
        }
    }

    #[test]
    fn v1_is_the_only_documented_export_and_import_schema() {
        assert_eq!(CURRENT_SCHEMA_VERSION, 1);
        assert_eq!(SUPPORTED_IMPORT_SCHEMA_VERSIONS, &[1]);
    }

    #[test]
    fn a_failure_after_the_temporary_write_preserves_the_existing_backup() {
        let root = tempdir().expect("temporary export root");
        let destination = root.path().join("existing-backup.json");
        std::fs::write(&destination, "OLD_BACKUP").expect("existing backup fixture");

        let error = write_atomically_with(
            &destination,
            |file| {
                file.write_all(b"NEW_BACKUP")?;
                Ok(())
            },
            |temporary, _| {
                assert_eq!(
                    std::fs::read_to_string(temporary.path()).expect("synced temporary contents"),
                    "NEW_BACKUP"
                );
                Err(super::BackupError::Write(std::io::Error::other(
                    "injected persist failure",
                )))
            },
        )
        .expect_err("injected persist failure");

        assert!(matches!(error, super::BackupError::Write(_)));
        assert_eq!(
            std::fs::read_to_string(destination).expect("preserved existing backup"),
            "OLD_BACKUP"
        );
    }

    #[tokio::test]
    async fn empty_export_uses_the_exact_camel_case_v1_envelope() {
        let (_root, database) = test_database().await;
        let destination_root = tempdir().expect("temporary export root");
        let destination = destination_root.path().join("empty-backup.json");
        std::fs::write(&destination, "REPLACE_ME").expect("existing backup fixture");

        let response = export_backup_to_path_at(
            &database,
            destination.clone(),
            timestamp("2026-07-17T08:30:45.123Z"),
        )
        .await
        .expect("empty export");

        assert_eq!(response.schema_version, 1);
        assert_eq!(response.exported_at, "2026-07-17T08:30:45.123Z");
        assert_eq!(response.settings_count, 0);
        assert_eq!(response.library_entry_count, 0);
        assert_eq!(response.watch_activity_count, 0);

        let contents = std::fs::read_to_string(destination).expect("exported JSON");
        assert!(contents.ends_with('\n'));
        let value: Value = serde_json::from_str(&contents).expect("valid backup JSON");
        assert_eq!(
            value,
            json!({
                "schemaVersion": 1,
                "exportedAt": "2026-07-17T08:30:45.123Z",
                "settings": [],
                "libraryEntries": [],
                "watchActivity": []
            })
        );
        assert!(value.get("schema_version").is_none());
    }

    #[tokio::test]
    async fn populated_export_preserves_user_state_and_excludes_cached_data() {
        let (_root, database) = test_database().await;
        database
            .settings()
            .upsert_many(&[
                SettingRecord {
                    key: "legacyPreference".to_owned(),
                    value: json!({"kept": true}),
                    updated_at: timestamp("2026-07-17T07:00:00Z"),
                },
                SettingRecord {
                    key: "themePreference".to_owned(),
                    value: json!("MACCHIATO"),
                    updated_at: timestamp("2026-07-17T07:30:00Z"),
                },
            ])
            .await
            .expect("settings");
        let mut cached_media = MediaSnapshotRecord {
            anilist_id: 101,
            media_kind: MediaKind::Anime,
            title: "CACHE_SENTINEL_TITLE".to_owned(),
            cover_url: Some("https://cache.invalid/sentinel.jpg".to_owned()),
            media_format: Some("TV".to_owned()),
            genres: vec!["CACHE_SENTINEL_GENRE".to_owned()],
            tags: vec![MediaTag {
                name: "CACHE_SENTINEL_TAG".to_owned(),
                rank: 99,
            }],
            media_status: Some("RELEASING".to_owned()),
            total_units: Some(12),
            duration_minutes: None,
            release_date: None,
            cached_at: timestamp("2026-07-17T06:00:00Z"),
        };
        database
            .media()
            .upsert_snapshot(&cached_media)
            .await
            .expect("cached media");
        database
            .media()
            .upsert_airing(&AiringDataRecord {
                anilist_id: 101,
                media_kind: MediaKind::Anime,
                episode_number: 11,
                airing_at: timestamp("2026-07-24T08:00:00Z"),
                fetched_at: timestamp("2026-07-17T08:00:00Z"),
            })
            .await
            .expect("cached airing");
        database
            .cache()
            .upsert(&CacheRecord {
                cache_key: "CACHE_SENTINEL_RESPONSE".to_owned(),
                payload: json!({"secretSentinel": true}),
                fetched_at: timestamp("2026-07-17T08:00:00Z"),
                expires_at: timestamp("2026-07-17T14:00:00Z"),
            })
            .await
            .expect("response cache");

        database
            .library()
            .upsert_at(
                request(0, "  猫と星 // comfort notes  "),
                timestamp("2026-07-17T08:00:00Z"),
            )
            .await
            .expect("baseline entry");
        database
            .library()
            .upsert_at(
                request(10, "猫と星 // comfort notes"),
                timestamp("2026-07-17T08:15:00Z"),
            )
            .await
            .expect("activity increase");
        cached_media.duration_minutes = Some(24);
        cached_media.cached_at = timestamp("2026-07-17T06:30:00Z");
        database
            .media()
            .upsert_snapshot(&cached_media)
            .await
            .expect("updated duration cache");
        database
            .library()
            .upsert_at(
                request(11, "猫と星 // comfort notes"),
                timestamp("2026-07-17T08:18:00Z"),
            )
            .await
            .expect("known-duration activity increase");
        database
            .library()
            .upsert_at(
                request(5, "猫と星 // corrected"),
                timestamp("2026-07-17T08:20:00Z"),
            )
            .await
            .expect("progress correction");

        let destination_root = tempdir().expect("temporary export root");
        let destination = destination_root.path().join("猫-backup.json");
        let response = export_backup_to_path_at(
            &database,
            destination.clone(),
            timestamp("2026-07-17T09:00:00Z"),
        )
        .await
        .expect("populated export");

        assert_eq!(response.settings_count, 2);
        assert_eq!(response.library_entry_count, 1);
        assert_eq!(response.watch_activity_count, 2);
        let contents = std::fs::read_to_string(destination).expect("exported JSON");
        let value: Value = serde_json::from_str(&contents).expect("valid backup JSON");

        assert_eq!(
            value["settings"][0],
            json!({
                "key": "legacyPreference",
                "value": {"kept": true},
                "updatedAt": "2026-07-17T07:00:00.000Z"
            })
        );
        assert_eq!(
            value["settings"][1],
            json!({
                "key": "themePreference",
                "value": "MACCHIATO",
                "updatedAt": "2026-07-17T07:30:00.000Z"
            })
        );
        assert_eq!(value["libraryEntries"][0]["anilistId"], 101);
        assert_eq!(value["libraryEntries"][0]["mediaKind"], "ANIME");
        assert_eq!(value["libraryEntries"][0]["status"], "WATCHING");
        assert_eq!(value["libraryEntries"][0]["progress"], 5);
        assert_eq!(value["libraryEntries"][0]["activityProgress"], 11);
        assert_eq!(value["libraryEntries"][0]["score"], 8.5);
        assert_eq!(value["libraryEntries"][0]["notes"], "猫と星 // corrected");
        assert_eq!(
            value["libraryEntries"][0]["createdAt"],
            "2026-07-17T08:00:00.000Z"
        );
        assert_eq!(
            value["libraryEntries"][0]["updatedAt"],
            "2026-07-17T08:20:00.000Z"
        );
        assert_eq!(value["watchActivity"][0]["anilistId"], 101);
        assert_eq!(value["watchActivity"][0]["mediaKind"], "ANIME");
        assert_eq!(value["watchActivity"][0]["progressDelta"], 10);
        assert!(value["watchActivity"][0]["episodeDurationMinutes"].is_null());
        assert_eq!(
            value["watchActivity"][0]["occurredAt"],
            "2026-07-17T08:15:00.000Z"
        );
        assert_eq!(value["watchActivity"][1]["progressDelta"], 1);
        assert_eq!(value["watchActivity"][1]["episodeDurationMinutes"], 24);
        assert_eq!(
            value["watchActivity"][1]["occurredAt"],
            "2026-07-17T08:18:00.000Z"
        );
        assert!(
            Uuid::parse_str(
                value["watchActivity"][0]["eventId"]
                    .as_str()
                    .expect("event id string")
            )
            .is_ok()
        );
        let first_event_id = value["watchActivity"][0]["eventId"]
            .as_str()
            .expect("first event id");
        let second_event_id = value["watchActivity"][1]["eventId"]
            .as_str()
            .expect("second event id");
        assert_ne!(first_event_id, second_event_id);
        Uuid::parse_str(second_event_id).expect("second event UUID");

        for excluded in [
            "CACHE_SENTINEL_TITLE",
            "CACHE_SENTINEL_GENRE",
            "CACHE_SENTINEL_TAG",
            "CACHE_SENTINEL_RESPONSE",
            "secretSentinel",
        ] {
            assert!(
                !contents.contains(excluded),
                "excluded cache value {excluded}"
            );
        }
    }

    #[tokio::test]
    async fn invalid_paths_and_snapshot_failures_do_not_touch_existing_files() {
        let (_root, database) = test_database().await;
        let relative_error = export_backup_to_path_at(
            &database,
            "relative-backup.json".into(),
            timestamp("2026-07-17T09:00:00Z"),
        )
        .await
        .expect_err("relative path must fail");
        assert_eq!(relative_error.code, "BACKUP_PATH_INVALID");
        assert!(!relative_error.retryable);

        let protected_error = export_backup_to_path_at(
            &database,
            _root.path().join("must-not-replace-live-storage.json"),
            timestamp("2026-07-17T09:00:00Z"),
        )
        .await
        .expect_err("application storage must be protected");
        assert_eq!(protected_error.code, "BACKUP_PATH_PROTECTED");
        assert!(!protected_error.retryable);

        let missing_parent = tempdir()
            .expect("temporary missing-parent root")
            .path()
            .join("missing")
            .join("backup.json");
        let write_error =
            export_backup_to_path_at(&database, missing_parent, timestamp("2026-07-17T09:00:00Z"))
                .await
                .expect_err("missing parent must fail");
        assert_eq!(write_error.code, "BACKUP_WRITE_FAILED");
        assert!(write_error.retryable);

        let destination_root = tempdir().expect("temporary export root");
        let destination = destination_root.path().join("existing-backup.json");
        std::fs::write(&destination, "KEEP_EXISTING").expect("existing file fixture");
        database.clone().close().await;

        let database_error = export_backup_to_path_at(
            &database,
            destination.clone(),
            timestamp("2026-07-17T09:00:00Z"),
        )
        .await
        .expect_err("closed database must fail first");
        assert_eq!(database_error.code, "DATABASE_UNAVAILABLE");
        assert_eq!(
            std::fs::read_to_string(destination).expect("untouched existing file"),
            "KEEP_EXISTING"
        );
    }
}
