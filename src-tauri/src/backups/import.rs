use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, Sqlite, Transaction};
use uuid::Uuid;

use crate::database::{Database, DatabaseError};
use crate::domain::{CommandError, MAX_NOTES_CHARACTERS, MediaKind, format_utc};

use super::{
    BackupEnvelopeV1, BackupEnvelopeV2, BackupLibraryEntryV2, BackupProgressActivityV2,
    BackupSettingV1, SUPPORTED_IMPORT_SCHEMA_VERSIONS,
};

const MAX_BACKUP_BYTES: u64 = 64 * 1024 * 1024;
const CLEAR_CONFIRMATION: &str = "CLEAR MY LOCAL DATA";

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportValidationIssue {
    pub code: &'static str,
    pub message: String,
    pub path: Option<String>,
}

impl ImportValidationIssue {
    fn new(code: &'static str, message: impl Into<String>, path: Option<String>) -> Self {
        Self {
            code,
            message: message.into(),
            path,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportChangeSummary {
    pub settings_added: usize,
    pub settings_updated: usize,
    pub settings_kept_local: usize,
    pub library_added: usize,
    pub library_merged: usize,
    pub library_kept_local: usize,
    pub library_conflicts: usize,
    pub activity_added: usize,
    pub activity_unchanged: usize,
    pub unrelated_local_library_entries_retained: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportPreviewResponse {
    pub file_name: String,
    pub schema_version: Option<u32>,
    pub exported_at: Option<String>,
    pub settings_count: usize,
    pub library_entry_count: usize,
    pub watch_activity_count: usize,
    pub changes: ImportChangeSummary,
    pub validation_errors: Vec<ImportValidationIssue>,
    pub can_apply: bool,
    pub import_token: Option<String>,
    pub preserves_unrelated_local_entries: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ImportBackupRequest {
    pub import_token: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportBackupResponse {
    pub schema_version: u32,
    pub changes: ImportChangeSummary,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ClearLocalDataRequest {
    pub confirmation: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClearLocalDataResponse {
    pub settings_removed: u64,
    pub library_entries_removed: u64,
    pub watch_activity_removed: u64,
    pub cached_records_removed: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SettingsOverview {
    pub cached_media_count: i64,
    pub cached_anime_count: i64,
    pub cached_manga_count: i64,
    pub stale_media_count: i64,
    pub library_anime_count: i64,
    pub library_manga_count: i64,
    pub anime_activity_count: i64,
    pub manga_activity_count: i64,
    pub airing_record_count: i64,
    pub stale_airing_count: i64,
    pub response_cache_count: i64,
    pub stale_response_cache_count: i64,
    pub latest_cache_update_at: Option<String>,
}

#[derive(Debug, Clone)]
struct ValidatedBackup {
    schema_version: u32,
    exported_at: String,
    settings: Vec<ValidatedSetting>,
    library_entries: Vec<ValidatedLibraryEntry>,
    watch_activity: Vec<ValidatedActivity>,
}

#[derive(Debug, Clone)]
struct ValidatedSetting {
    source: BackupSettingV1,
    updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
struct ValidatedLibraryEntry {
    source: BackupLibraryEntryV2,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ValidatedActivity {
    source: BackupProgressActivityV2,
    event_id: Uuid,
    occurred_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
struct ImportSession {
    backup: ValidatedBackup,
    preview: ImportChangeSummary,
}

#[derive(Debug, Clone, Default)]
pub struct ImportSessionStore {
    sessions: Arc<Mutex<HashMap<Uuid, ImportSession>>>,
}

impl ImportSessionStore {
    fn insert(&self, session: ImportSession) -> Result<Uuid, CommandError> {
        let token = Uuid::new_v4();
        self.sessions
            .lock()
            .map_err(|_| import_session_error())?
            .insert(token, session);
        Ok(token)
    }

    fn get(&self, token: Uuid) -> Result<ImportSession, CommandError> {
        self.sessions
            .lock()
            .map_err(|_| import_session_error())?
            .get(&token)
            .cloned()
            .ok_or_else(|| {
                CommandError::new(
                    "IMPORT_SESSION_EXPIRED",
                    "Preview the backup again before restoring it.",
                    false,
                )
            })
    }

    fn remove(&self, token: Uuid) -> Result<(), CommandError> {
        self.sessions
            .lock()
            .map_err(|_| import_session_error())?
            .remove(&token);
        Ok(())
    }
}

fn import_session_error() -> CommandError {
    CommandError::new(
        "IMPORT_SESSION_FAILED",
        "Animi could not access the prepared recovery session.",
        true,
    )
}

#[derive(Debug, thiserror::Error)]
enum ImportError {
    #[error("backup file task failed: {0}")]
    FileTask(#[from] tokio::task::JoinError),
    #[error("backup file read failed: {0}")]
    Read(#[from] std::io::Error),
    #[error("backup database operation failed: {0}")]
    Database(#[from] DatabaseError),
    #[error("the import preview is stale")]
    PreviewStale,
    #[error("an activity event identity conflicts with local data")]
    ActivityConflict,
    #[error("backup apply failed: {0}")]
    Apply(String),
}

impl ImportError {
    fn into_command_error(self) -> CommandError {
        match self {
            Self::FileTask(_) | Self::Read(_) => CommandError::new(
                "BACKUP_READ_FAILED",
                "Animi could not read that backup file.",
                true,
            ),
            Self::Database(error) => error.into_command_error(),
            Self::PreviewStale => CommandError::new(
                "IMPORT_PREVIEW_STALE",
                "Local data changed after this preview. Preview the backup again before restoring it.",
                false,
            ),
            Self::ActivityConflict => CommandError::new(
                "BACKUP_ACTIVITY_ID_CONFLICT",
                "A watch activity event ID now conflicts with local data. Preview the backup again.",
                false,
            ),
            Self::Apply(_) => CommandError::new(
                "BACKUP_IMPORT_FAILED",
                "Animi could not restore the backup. No changes were kept.",
                true,
            ),
        }
    }
}

pub(crate) async fn preview_import_from_path(
    database: &Database,
    sessions: &ImportSessionStore,
    source_path: PathBuf,
) -> Result<ImportPreviewResponse, CommandError> {
    if !source_path.is_absolute() {
        return Err(CommandError::new(
            "BACKUP_PATH_INVALID",
            "Choose a valid backup file.",
            false,
        ));
    }
    let file_name = source_path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("Animi backup")
        .to_owned();
    let contents = tokio::task::spawn_blocking(move || read_backup_file(&source_path))
        .await
        .map_err(ImportError::from)
        .and_then(|result| result.map_err(ImportError::from))
        .map_err(ImportError::into_command_error)?;

    let parsed = parse_and_validate(&contents);
    let (schema_version, exported_at, counts) = preview_metadata(&contents, parsed.as_ref().ok());
    let backup = match parsed {
        Ok(backup) => backup,
        Err(validation_errors) => {
            return Ok(ImportPreviewResponse {
                file_name,
                schema_version,
                exported_at,
                settings_count: counts.0,
                library_entry_count: counts.1,
                watch_activity_count: counts.2,
                changes: ImportChangeSummary::default(),
                validation_errors,
                can_apply: false,
                import_token: None,
                preserves_unrelated_local_entries: true,
            });
        }
    };

    let mut transaction = database
        .pool()
        .begin()
        .await
        .map_err(DatabaseError::from)
        .map_err(ImportError::from)
        .map_err(ImportError::into_command_error)?;
    let plan = build_plan(&mut transaction, &backup).await;
    transaction
        .rollback()
        .await
        .map_err(DatabaseError::from)
        .map_err(ImportError::from)
        .map_err(ImportError::into_command_error)?;
    let plan = match plan {
        Ok(plan) => plan,
        Err(PlanError::ActivityConflict(event_id)) => {
            return Ok(ImportPreviewResponse {
                file_name,
                schema_version: Some(backup.schema_version),
                exported_at: Some(backup.exported_at.clone()),
                settings_count: backup.settings.len(),
                library_entry_count: backup.library_entries.len(),
                watch_activity_count: backup.watch_activity.len(),
                changes: ImportChangeSummary::default(),
                validation_errors: vec![ImportValidationIssue::new(
                    "BACKUP_ACTIVITY_ID_CONFLICT",
                    "A watch activity event ID already exists with different details.",
                    Some(format!("watchActivity[{event_id}]")),
                )],
                can_apply: false,
                import_token: None,
                preserves_unrelated_local_entries: true,
            });
        }
        Err(PlanError::Database(error)) => return Err(error.into_command_error()),
    };

    let token = sessions.insert(ImportSession {
        backup: backup.clone(),
        preview: plan.summary,
    })?;
    Ok(ImportPreviewResponse {
        file_name,
        schema_version: Some(backup.schema_version),
        exported_at: Some(backup.exported_at.clone()),
        settings_count: backup.settings.len(),
        library_entry_count: backup.library_entries.len(),
        watch_activity_count: backup.watch_activity.len(),
        changes: plan.summary,
        validation_errors: Vec::new(),
        can_apply: true,
        import_token: Some(token.to_string()),
        preserves_unrelated_local_entries: true,
    })
}

fn read_backup_file(path: &Path) -> Result<String, std::io::Error> {
    let metadata = std::fs::metadata(path)?;
    if !metadata.is_file() {
        return Err(std::io::Error::other("backup source is not a file"));
    }
    if metadata.len() > MAX_BACKUP_BYTES {
        return Err(std::io::Error::other("backup file exceeds the size limit"));
    }
    std::fs::read_to_string(path)
}

fn preview_metadata(
    contents: &str,
    valid: Option<&ValidatedBackup>,
) -> (Option<u32>, Option<String>, (usize, usize, usize)) {
    if let Some(valid) = valid {
        return (
            Some(valid.schema_version),
            Some(valid.exported_at.clone()),
            (
                valid.settings.len(),
                valid.library_entries.len(),
                valid.watch_activity.len(),
            ),
        );
    }
    let Ok(value) = serde_json::from_str::<serde_json::Value>(contents) else {
        return (None, None, (0, 0, 0));
    };
    let schema_version = value
        .get("schemaVersion")
        .and_then(serde_json::Value::as_u64)
        .and_then(|version| u32::try_from(version).ok());
    let exported_at = value
        .get("exportedAt")
        .and_then(serde_json::Value::as_str)
        .map(str::to_owned);
    let count = |key: &str| {
        value
            .get(key)
            .and_then(serde_json::Value::as_array)
            .map_or(0, Vec::len)
    };
    (
        schema_version,
        exported_at,
        (
            count("settings"),
            count("libraryEntries"),
            count("watchActivity"),
        ),
    )
}

fn parse_and_validate(contents: &str) -> Result<ValidatedBackup, Vec<ImportValidationIssue>> {
    let value: serde_json::Value = serde_json::from_str(contents).map_err(|error| {
        vec![ImportValidationIssue::new(
            "BACKUP_JSON_INVALID",
            format!("The backup is not valid JSON: {error}"),
            None,
        )]
    })?;
    let Some(version) = value
        .get("schemaVersion")
        .and_then(serde_json::Value::as_u64)
    else {
        return Err(vec![ImportValidationIssue::new(
            "BACKUP_SCHEMA_VERSION_MISSING",
            "The backup does not contain a numeric schemaVersion.",
            Some("schemaVersion".to_owned()),
        )]);
    };
    let Ok(schema_version) = u32::try_from(version) else {
        return Err(vec![ImportValidationIssue::new(
            "BACKUP_SCHEMA_VERSION_UNSUPPORTED",
            format!("Backup schema version {version} is not supported by this Animi release."),
            Some("schemaVersion".to_owned()),
        )]);
    };
    if !SUPPORTED_IMPORT_SCHEMA_VERSIONS.contains(&schema_version) {
        return Err(vec![ImportValidationIssue::new(
            "BACKUP_SCHEMA_VERSION_UNSUPPORTED",
            format!("Backup schema version {version} is not supported by this Animi release."),
            Some("schemaVersion".to_owned()),
        )]);
    }
    let (schema_version, exported_at, settings_source, library_source, activity_source) =
        match schema_version {
            1 => {
                let envelope: BackupEnvelopeV1 =
                    serde_json::from_value(value).map_err(|error| {
                        vec![ImportValidationIssue::new(
                            "BACKUP_SHAPE_INVALID",
                            format!("The backup structure is invalid: {error}"),
                            None,
                        )]
                    })?;
                (
                    1,
                    envelope.exported_at,
                    envelope.settings,
                    envelope
                        .library_entries
                        .into_iter()
                        .map(|entry| BackupLibraryEntryV2 {
                            anilist_id: entry.anilist_id,
                            media_kind: entry.media_kind,
                            status: entry.status,
                            progress: entry.progress,
                            volume_progress: (entry.media_kind == MediaKind::Manga).then_some(0),
                            activity_progress: entry.activity_progress,
                            activity_volume_progress: (entry.media_kind == MediaKind::Manga)
                                .then_some(0),
                            score: entry.score,
                            notes: entry.notes,
                            created_at: entry.created_at,
                            updated_at: entry.updated_at,
                        })
                        .collect(),
                    envelope
                        .watch_activity
                        .into_iter()
                        .map(|activity| BackupProgressActivityV2 {
                            event_id: activity.event_id,
                            anilist_id: activity.anilist_id,
                            media_kind: activity.media_kind,
                            progress_delta: activity.progress_delta,
                            volume_delta: 0,
                            episode_duration_minutes: activity.episode_duration_minutes,
                            occurred_at: activity.occurred_at,
                        })
                        .collect(),
                )
            }
            2 => {
                let envelope: BackupEnvelopeV2 =
                    serde_json::from_value(value).map_err(|error| {
                        vec![ImportValidationIssue::new(
                            "BACKUP_SHAPE_INVALID",
                            format!("The backup structure is invalid: {error}"),
                            None,
                        )]
                    })?;
                (
                    2,
                    envelope.exported_at,
                    envelope.settings,
                    envelope.library_entries,
                    envelope.watch_activity,
                )
            }
            _ => {
                return Err(vec![ImportValidationIssue::new(
                    "BACKUP_SCHEMA_VERSION_UNSUPPORTED",
                    format!(
                        "Backup schema version {version} is not supported by this Animi release."
                    ),
                    Some("schemaVersion".to_owned()),
                )]);
            }
        };

    let mut issues = Vec::new();
    parse_timestamp(&exported_at, "exportedAt", &mut issues);

    let mut setting_keys = HashSet::new();
    let mut settings = Vec::with_capacity(settings_source.len());
    for (index, setting) in settings_source.into_iter().enumerate() {
        let base = format!("settings[{index}]");
        if setting.key.trim().is_empty() || setting.key.len() > 256 {
            issues.push(ImportValidationIssue::new(
                "BACKUP_SETTING_KEY_INVALID",
                "A setting key must contain between 1 and 256 characters.",
                Some(format!("{base}.key")),
            ));
        }
        if setting.key == "themePreference"
            && !matches!(
                setting.value.as_str(),
                Some("SYSTEM" | "LATTE" | "MACCHIATO")
            )
        {
            issues.push(ImportValidationIssue::new(
                "BACKUP_THEME_PREFERENCE_INVALID",
                "The themePreference setting must be SYSTEM, LATTE, or MACCHIATO.",
                Some(format!("{base}.value")),
            ));
        }
        if !setting_keys.insert(setting.key.clone()) {
            issues.push(ImportValidationIssue::new(
                "BACKUP_SETTING_DUPLICATE",
                format!("The setting key '{}' appears more than once.", setting.key),
                Some(format!("{base}.key")),
            ));
        }
        if let Some(updated_at) = parse_timestamp(
            &setting.updated_at,
            &format!("{base}.updatedAt"),
            &mut issues,
        ) {
            settings.push(ValidatedSetting {
                source: setting,
                updated_at,
            });
        }
    }

    let mut library_keys = HashSet::new();
    let mut library_entries = Vec::with_capacity(library_source.len());
    for (index, entry) in library_source.into_iter().enumerate() {
        let base = format!("libraryEntries[{index}]");
        validate_anilist_id(entry.anilist_id, &format!("{base}.anilistId"), &mut issues);
        if !(0..=i64::from(i32::MAX)).contains(&entry.progress) {
            issues.push(ImportValidationIssue::new(
                "BACKUP_PROGRESS_INVALID",
                "Library progress is outside the supported range.",
                Some(format!("{base}.progress")),
            ));
        }
        if !(entry.progress..=i64::from(i32::MAX)).contains(&entry.activity_progress) {
            issues.push(ImportValidationIssue::new(
                "BACKUP_ACTIVITY_PROGRESS_INVALID",
                "Activity progress must be at least current progress and within the supported range.",
                Some(format!("{base}.activityProgress")),
            ));
        }
        let volume_values_valid = match entry.media_kind {
            MediaKind::Anime => {
                entry.volume_progress.is_none() && entry.activity_volume_progress.is_none()
            }
            MediaKind::Manga => matches!(
                (entry.volume_progress, entry.activity_volume_progress),
                (Some(progress), Some(high_water))
                    if (0..=i64::from(i32::MAX)).contains(&progress)
                        && (progress..=i64::from(i32::MAX)).contains(&high_water)
            ),
        };
        if !volume_values_valid {
            issues.push(ImportValidationIssue::new(
                "BACKUP_VOLUME_PROGRESS_INVALID",
                "Volume progress and its activity high-water mark are incompatible with the media kind.",
                Some(format!("{base}.volumeProgress")),
            ));
        }
        if entry
            .score
            .is_some_and(|score| !score.is_finite() || !(0.0..=10.0).contains(&score))
        {
            issues.push(ImportValidationIssue::new(
                "BACKUP_SCORE_INVALID",
                "A library score must be between 0 and 10.",
                Some(format!("{base}.score")),
            ));
        }
        if entry
            .notes
            .as_ref()
            .is_some_and(|notes| notes.chars().count() > MAX_NOTES_CHARACTERS)
        {
            issues.push(ImportValidationIssue::new(
                "BACKUP_NOTES_TOO_LONG",
                format!("Library notes cannot exceed {MAX_NOTES_CHARACTERS} characters."),
                Some(format!("{base}.notes")),
            ));
        }
        if !library_keys.insert((entry.anilist_id, entry.media_kind)) {
            issues.push(ImportValidationIssue::new(
                "BACKUP_LIBRARY_ENTRY_DUPLICATE",
                "A library entry appears more than once.",
                Some(base.clone()),
            ));
        }
        let created_at =
            parse_timestamp(&entry.created_at, &format!("{base}.createdAt"), &mut issues);
        let updated_at =
            parse_timestamp(&entry.updated_at, &format!("{base}.updatedAt"), &mut issues);
        if let (Some(created_at), Some(updated_at)) = (created_at, updated_at) {
            if updated_at < created_at {
                issues.push(ImportValidationIssue::new(
                    "BACKUP_LIBRARY_TIMESTAMPS_INVALID",
                    "A library entry updatedAt cannot be earlier than createdAt.",
                    Some(base),
                ));
            } else {
                library_entries.push(ValidatedLibraryEntry {
                    source: entry,
                    created_at,
                    updated_at,
                });
            }
        }
    }

    let mut activity_ids: HashMap<Uuid, BackupProgressActivityV2> = HashMap::new();
    let mut watch_activity = Vec::with_capacity(activity_source.len());
    for (index, activity) in activity_source.into_iter().enumerate() {
        let base = format!("watchActivity[{index}]");
        validate_anilist_id(
            activity.anilist_id,
            &format!("{base}.anilistId"),
            &mut issues,
        );
        if !(0..=i64::from(i32::MAX)).contains(&activity.progress_delta)
            || !(0..=i64::from(i32::MAX)).contains(&activity.volume_delta)
            || activity.progress_delta == 0 && activity.volume_delta == 0
        {
            issues.push(ImportValidationIssue::new(
                "BACKUP_ACTIVITY_DELTA_INVALID",
                "Progress activity deltas must be nonnegative, in range, and not both zero.",
                Some(format!("{base}.progressDelta")),
            ));
        }
        if activity.media_kind == MediaKind::Anime && activity.volume_delta != 0 {
            issues.push(ImportValidationIssue::new(
                "BACKUP_ACTIVITY_KIND_INVALID",
                "Anime activity cannot contain a volume delta.",
                Some(format!("{base}.volumeDelta")),
            ));
        }
        if activity.media_kind == MediaKind::Manga && activity.episode_duration_minutes.is_some() {
            issues.push(ImportValidationIssue::new(
                "BACKUP_ACTIVITY_KIND_INVALID",
                "Manga activity cannot contain an episode-duration snapshot.",
                Some(format!("{base}.episodeDurationMinutes")),
            ));
        }
        if activity
            .episode_duration_minutes
            .is_some_and(|duration| !(1..=1440).contains(&duration))
        {
            issues.push(ImportValidationIssue::new(
                "BACKUP_ACTIVITY_DURATION_INVALID",
                "Episode duration must be between 1 and 1440 minutes when present.",
                Some(format!("{base}.episodeDurationMinutes")),
            ));
        }
        let event_id = match Uuid::parse_str(&activity.event_id) {
            Ok(event_id) if event_id.get_version_num() == 4 => Some(event_id),
            Err(_) => {
                issues.push(ImportValidationIssue::new(
                    "BACKUP_ACTIVITY_ID_INVALID",
                    "A watch activity eventId is not a valid UUID.",
                    Some(format!("{base}.eventId")),
                ));
                None
            }
            Ok(_) => {
                issues.push(ImportValidationIssue::new(
                    "BACKUP_ACTIVITY_ID_INVALID",
                    "A watch activity eventId must be a version 4 UUID.",
                    Some(format!("{base}.eventId")),
                ));
                None
            }
        };
        let occurred_at = parse_timestamp(
            &activity.occurred_at,
            &format!("{base}.occurredAt"),
            &mut issues,
        );
        if let (Some(event_id), Some(occurred_at)) = (event_id, occurred_at) {
            if let Some(existing) = activity_ids.get(&event_id) {
                let (code, message) = if existing == &activity {
                    (
                        "BACKUP_ACTIVITY_EVENT_DUPLICATE",
                        "A watch activity event appears more than once.",
                    )
                } else {
                    (
                        "BACKUP_ACTIVITY_ID_CONFLICT",
                        "A watch activity eventId is reused with different details.",
                    )
                };
                issues.push(ImportValidationIssue::new(
                    code,
                    message,
                    Some(format!("{base}.eventId")),
                ));
            } else {
                activity_ids.insert(event_id, activity.clone());
                watch_activity.push(ValidatedActivity {
                    source: activity,
                    event_id,
                    occurred_at,
                });
            }
        }
    }

    if issues.is_empty() {
        Ok(ValidatedBackup {
            schema_version,
            exported_at,
            settings,
            library_entries,
            watch_activity,
        })
    } else {
        Err(issues)
    }
}

fn parse_timestamp(
    value: &str,
    path: &str,
    issues: &mut Vec<ImportValidationIssue>,
) -> Option<DateTime<Utc>> {
    match DateTime::parse_from_rfc3339(value) {
        Ok(timestamp) if timestamp.offset().local_minus_utc() == 0 => {
            Some(timestamp.with_timezone(&Utc))
        }
        _ => {
            issues.push(ImportValidationIssue::new(
                "BACKUP_TIMESTAMP_INVALID",
                "Backup timestamps must be valid RFC 3339 UTC values.",
                Some(path.to_owned()),
            ));
            None
        }
    }
}

fn validate_anilist_id(id: i64, path: &str, issues: &mut Vec<ImportValidationIssue>) {
    if !(1..=i64::from(i32::MAX)).contains(&id) {
        issues.push(ImportValidationIssue::new(
            "BACKUP_ANILIST_ID_INVALID",
            "An AniList identifier is outside the supported range.",
            Some(path.to_owned()),
        ));
    }
}

#[derive(Debug)]
struct ImportPlan {
    summary: ImportChangeSummary,
    setting_actions: Vec<SettingAction>,
    library_actions: Vec<LibraryAction>,
    activities_to_insert: Vec<ValidatedActivity>,
}

#[derive(Debug)]
enum SettingAction {
    Upsert(ValidatedSetting),
}

#[derive(Debug)]
struct LibraryAction {
    entry: ValidatedLibraryEntry,
    activity_progress: i64,
    activity_volume_progress: Option<i64>,
    imported_fields_win: bool,
}

#[derive(Debug, thiserror::Error)]
enum PlanError {
    #[error("database operation failed: {0}")]
    Database(#[from] DatabaseError),
    #[error("activity event ID conflicts with local data: {0}")]
    ActivityConflict(Uuid),
}

#[derive(Debug, FromRow)]
struct LocalSettingRow {
    key: String,
    updated_at: String,
}

#[derive(Debug, FromRow)]
struct LocalLibraryRow {
    anilist_id: i64,
    media_kind: String,
    activity_progress: i64,
    activity_volume_progress: Option<i64>,
    created_at: String,
    updated_at: String,
}

#[derive(Debug, FromRow)]
struct LocalActivityRow {
    event_id: String,
    anilist_id: i64,
    media_kind: String,
    progress_delta: i64,
    volume_delta: i64,
    episode_duration_minutes: Option<i64>,
    occurred_at: String,
}

async fn build_plan(
    transaction: &mut Transaction<'_, Sqlite>,
    backup: &ValidatedBackup,
) -> Result<ImportPlan, PlanError> {
    let settings = sqlx::query_as::<_, LocalSettingRow>("SELECT key, updated_at FROM settings")
        .fetch_all(&mut **transaction)
        .await
        .map_err(DatabaseError::from)?;
    let local_settings = settings
        .into_iter()
        .map(|row| {
            DateTime::parse_from_rfc3339(&row.updated_at)
                .map(|timestamp| (row.key, timestamp.with_timezone(&Utc)))
                .map_err(|error| DatabaseError::CorruptData(error.to_string()))
        })
        .collect::<Result<HashMap<_, _>, _>>()?;

    let library = sqlx::query_as::<_, LocalLibraryRow>(
        "SELECT anilist_id, media_kind, activity_progress, activity_volume_progress, created_at, updated_at FROM library_entries",
    ).fetch_all(&mut **transaction).await.map_err(DatabaseError::from)?;
    let local_library_count = library.len();
    let local_library = library
        .into_iter()
        .map(|row| {
            let kind = MediaKind::parse(&row.media_kind)
                .map_err(|error| DatabaseError::CorruptData(error.message))?;
            let created_at = DateTime::parse_from_rfc3339(&row.created_at)
                .map_err(|error| DatabaseError::CorruptData(error.to_string()))?
                .with_timezone(&Utc);
            let updated_at = DateTime::parse_from_rfc3339(&row.updated_at)
                .map_err(|error| DatabaseError::CorruptData(error.to_string()))?
                .with_timezone(&Utc);
            Ok((
                (row.anilist_id, kind),
                (
                    row.activity_progress,
                    row.activity_volume_progress,
                    created_at,
                    updated_at,
                ),
            ))
        })
        .collect::<Result<HashMap<_, _>, DatabaseError>>()?;

    let activities = sqlx::query_as::<_, LocalActivityRow>(
        "SELECT event_id, anilist_id, media_kind, progress_delta, volume_delta, episode_duration_minutes, occurred_at FROM watch_activity",
    ).fetch_all(&mut **transaction).await.map_err(DatabaseError::from)?;
    let local_activities = activities
        .into_iter()
        .map(|row| {
            let id = Uuid::parse_str(&row.event_id).map_err(|_| {
                DatabaseError::CorruptData("invalid local activity event ID".to_owned())
            })?;
            let kind = MediaKind::parse(&row.media_kind)
                .map_err(|error| DatabaseError::CorruptData(error.message))?;
            let occurred_at = DateTime::parse_from_rfc3339(&row.occurred_at)
                .map_err(|error| DatabaseError::CorruptData(error.to_string()))?
                .with_timezone(&Utc);
            Ok((
                id,
                (
                    row.anilist_id,
                    kind,
                    row.progress_delta,
                    row.volume_delta,
                    row.episode_duration_minutes,
                    occurred_at,
                ),
            ))
        })
        .collect::<Result<HashMap<_, _>, DatabaseError>>()?;

    let mut summary = ImportChangeSummary::default();
    let mut setting_actions = Vec::new();
    for setting in &backup.settings {
        match local_settings.get(&setting.source.key) {
            None => {
                summary.settings_added += 1;
                setting_actions.push(SettingAction::Upsert(setting.clone()));
            }
            Some(local_updated) if setting.updated_at > *local_updated => {
                summary.settings_updated += 1;
                setting_actions.push(SettingAction::Upsert(setting.clone()));
            }
            Some(_) => summary.settings_kept_local += 1,
        }
    }

    let imported_keys: HashSet<_> = backup
        .library_entries
        .iter()
        .map(|entry| (entry.source.anilist_id, entry.source.media_kind))
        .collect();
    summary.unrelated_local_library_entries_retained = local_library
        .keys()
        .filter(|key| !imported_keys.contains(key))
        .count();
    debug_assert!(summary.unrelated_local_library_entries_retained <= local_library_count);

    let mut library_actions = Vec::new();
    for entry in &backup.library_entries {
        let key = (entry.source.anilist_id, entry.source.media_kind);
        match local_library.get(&key) {
            None => {
                summary.library_added += 1;
                library_actions.push(LibraryAction {
                    entry: entry.clone(),
                    activity_progress: entry.source.activity_progress,
                    activity_volume_progress: entry.source.activity_volume_progress,
                    imported_fields_win: true,
                });
            }
            Some((
                local_activity_progress,
                local_activity_volume_progress,
                local_created_at,
                local_updated_at,
            )) => {
                summary.library_conflicts += 1;
                let imported_fields_win = entry.updated_at > *local_updated_at;
                let activity_progress = if entry.created_at == *local_created_at {
                    entry.source.activity_progress.max(*local_activity_progress)
                } else if imported_fields_win {
                    entry.source.activity_progress
                } else {
                    *local_activity_progress
                };
                let activity_volume_progress = if entry.created_at == *local_created_at {
                    match (
                        entry.source.activity_volume_progress,
                        *local_activity_volume_progress,
                    ) {
                        (Some(imported), Some(local)) => Some(imported.max(local)),
                        (imported, local) => imported.or(local),
                    }
                } else if imported_fields_win {
                    entry.source.activity_volume_progress
                } else {
                    *local_activity_volume_progress
                };
                if imported_fields_win
                    || activity_progress != *local_activity_progress
                    || activity_volume_progress != *local_activity_volume_progress
                {
                    summary.library_merged += 1;
                    library_actions.push(LibraryAction {
                        entry: entry.clone(),
                        activity_progress,
                        activity_volume_progress,
                        imported_fields_win,
                    });
                } else {
                    summary.library_kept_local += 1;
                }
            }
        }
    }

    let mut activities_to_insert = Vec::new();
    for activity in &backup.watch_activity {
        let imported = (
            activity.source.anilist_id,
            activity.source.media_kind,
            activity.source.progress_delta,
            activity.source.volume_delta,
            activity.source.episode_duration_minutes,
            activity.occurred_at,
        );
        match local_activities.get(&activity.event_id) {
            None => {
                summary.activity_added += 1;
                activities_to_insert.push(activity.clone());
            }
            Some(local) if local == &imported => summary.activity_unchanged += 1,
            Some(_) => return Err(PlanError::ActivityConflict(activity.event_id)),
        }
    }

    Ok(ImportPlan {
        summary,
        setting_actions,
        library_actions,
        activities_to_insert,
    })
}

pub(crate) async fn import_backup(
    database: &Database,
    sessions: &ImportSessionStore,
    request: ImportBackupRequest,
) -> Result<ImportBackupResponse, CommandError> {
    let token = Uuid::parse_str(&request.import_token).map_err(|_| {
        CommandError::new(
            "IMPORT_TOKEN_INVALID",
            "Preview the backup again before restoring it.",
            false,
        )
    })?;
    let session = sessions.get(token)?;
    let changes = apply_import(database, &session.backup, session.preview, None)
        .await
        .map_err(ImportError::into_command_error)?;
    sessions.remove(token)?;
    Ok(ImportBackupResponse {
        schema_version: session.backup.schema_version,
        changes,
    })
}

async fn apply_import(
    database: &Database,
    backup: &ValidatedBackup,
    expected: ImportChangeSummary,
    fail_after_writes: Option<usize>,
) -> Result<ImportChangeSummary, ImportError> {
    let mut transaction = database.pool().begin().await.map_err(DatabaseError::from)?;
    let plan = match build_plan(&mut transaction, backup).await {
        Ok(plan) => plan,
        Err(PlanError::ActivityConflict(_)) => {
            transaction.rollback().await.map_err(DatabaseError::from)?;
            return Err(ImportError::ActivityConflict);
        }
        Err(PlanError::Database(error)) => {
            transaction.rollback().await.map_err(DatabaseError::from)?;
            return Err(error.into());
        }
    };
    if plan.summary != expected {
        transaction.rollback().await.map_err(DatabaseError::from)?;
        return Err(ImportError::PreviewStale);
    }

    let mut writes = 0_usize;
    for action in plan.setting_actions {
        let SettingAction::Upsert(setting) = action;
        sqlx::query(
            "INSERT INTO settings (key, value_json, updated_at) VALUES (?, ?, ?) \
             ON CONFLICT(key) DO UPDATE SET value_json = excluded.value_json, updated_at = excluded.updated_at",
        )
        .bind(&setting.source.key)
        .bind(serde_json::to_string(&setting.source.value).map_err(DatabaseError::from)?)
        .bind(format_utc(setting.updated_at))
        .execute(&mut *transaction).await.map_err(DatabaseError::from)?;
        writes += 1;
        if fail_after_writes.is_some_and(|limit| writes >= limit) {
            transaction.rollback().await.map_err(DatabaseError::from)?;
            return Err(ImportError::Apply("injected mid-import failure".to_owned()));
        }
    }
    for action in plan.library_actions {
        if action.imported_fields_win {
            sqlx::query(
                "INSERT INTO library_entries (anilist_id, media_kind, status, progress, volume_progress, activity_progress, activity_volume_progress, score, notes, created_at, updated_at) \
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?) ON CONFLICT(anilist_id, media_kind) DO UPDATE SET \
                 status = excluded.status, progress = excluded.progress, volume_progress = excluded.volume_progress, \
                 activity_progress = excluded.activity_progress, activity_volume_progress = excluded.activity_volume_progress, \
                 score = excluded.score, notes = excluded.notes, created_at = excluded.created_at, updated_at = excluded.updated_at",
            )
            .bind(action.entry.source.anilist_id)
            .bind(action.entry.source.media_kind.as_str())
            .bind(action.entry.source.status.as_str())
            .bind(action.entry.source.progress)
            .bind(action.entry.source.volume_progress)
            .bind(action.activity_progress)
            .bind(action.activity_volume_progress)
            .bind(action.entry.source.score)
            .bind(&action.entry.source.notes)
            .bind(format_utc(action.entry.created_at))
            .bind(format_utc(action.entry.updated_at))
            .execute(&mut *transaction).await.map_err(DatabaseError::from)?;
        } else {
            sqlx::query(
                "UPDATE library_entries SET activity_progress = ?, activity_volume_progress = ? WHERE anilist_id = ? AND media_kind = ?",
            )
            .bind(action.activity_progress)
            .bind(action.activity_volume_progress)
            .bind(action.entry.source.anilist_id)
            .bind(action.entry.source.media_kind.as_str())
            .execute(&mut *transaction).await.map_err(DatabaseError::from)?;
        }
        writes += 1;
        if fail_after_writes.is_some_and(|limit| writes >= limit) {
            transaction.rollback().await.map_err(DatabaseError::from)?;
            return Err(ImportError::Apply("injected mid-import failure".to_owned()));
        }
    }
    for activity in plan.activities_to_insert {
        sqlx::query(
            "INSERT INTO watch_activity (event_id, anilist_id, media_kind, progress_delta, volume_delta, episode_duration_minutes, occurred_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(activity.event_id.to_string())
        .bind(activity.source.anilist_id)
        .bind(activity.source.media_kind.as_str())
        .bind(activity.source.progress_delta)
        .bind(activity.source.volume_delta)
        .bind(activity.source.episode_duration_minutes)
        .bind(format_utc(activity.occurred_at))
        .execute(&mut *transaction).await.map_err(DatabaseError::from)?;
        writes += 1;
        if fail_after_writes.is_some_and(|limit| writes >= limit) {
            transaction.rollback().await.map_err(DatabaseError::from)?;
            return Err(ImportError::Apply("injected mid-import failure".to_owned()));
        }
    }
    transaction.commit().await.map_err(DatabaseError::from)?;
    Ok(plan.summary)
}

pub(crate) async fn get_settings_overview(
    database: &Database,
) -> Result<SettingsOverview, CommandError> {
    let now = Utc::now();
    let media_stale_before = format_utc(now - Duration::hours(24));
    let airing_stale_before = format_utc(now - Duration::minutes(30));
    let now_string = format_utc(now);
    let row = sqlx::query_as::<_, (
        i64, i64, i64, i64, i64, i64, i64, i64, i64, i64, i64, i64, Option<String>
    )>(
        "SELECT \
          (SELECT COUNT(*) FROM media_snapshots), \
          (SELECT COUNT(*) FROM media_snapshots WHERE media_kind = 'ANIME'), \
          (SELECT COUNT(*) FROM media_snapshots WHERE media_kind = 'MANGA'), \
          (SELECT COUNT(*) FROM media_snapshots WHERE cached_at < ?), \
          (SELECT COUNT(*) FROM library_entries WHERE media_kind = 'ANIME'), \
          (SELECT COUNT(*) FROM library_entries WHERE media_kind = 'MANGA'), \
          (SELECT COUNT(*) FROM watch_activity WHERE media_kind = 'ANIME'), \
          (SELECT COUNT(*) FROM watch_activity WHERE media_kind = 'MANGA'), \
          (SELECT COUNT(*) FROM airing_data), \
          (SELECT COUNT(*) FROM airing_data WHERE fetched_at < ?), \
          (SELECT COUNT(*) FROM response_caches), \
          (SELECT COUNT(*) FROM response_caches WHERE expires_at <= ?), \
          MAX(value) FROM (SELECT MAX(cached_at) AS value FROM media_snapshots UNION ALL SELECT MAX(fetched_at) FROM airing_data UNION ALL SELECT MAX(fetched_at) FROM response_caches)",
    )
    .bind(media_stale_before).bind(airing_stale_before).bind(now_string)
    .fetch_one(database.pool()).await.map_err(DatabaseError::from)
    .map_err(DatabaseError::into_command_error)?;
    Ok(SettingsOverview {
        cached_media_count: row.0,
        cached_anime_count: row.1,
        cached_manga_count: row.2,
        stale_media_count: row.3,
        library_anime_count: row.4,
        library_manga_count: row.5,
        anime_activity_count: row.6,
        manga_activity_count: row.7,
        airing_record_count: row.8,
        stale_airing_count: row.9,
        response_cache_count: row.10,
        stale_response_cache_count: row.11,
        latest_cache_update_at: row.12,
    })
}

pub(crate) async fn clear_local_data(
    database: &Database,
    request: ClearLocalDataRequest,
) -> Result<ClearLocalDataResponse, CommandError> {
    if request.confirmation != CLEAR_CONFIRMATION {
        return Err(CommandError::new(
            "CLEAR_CONFIRMATION_REQUIRED",
            format!("Type {CLEAR_CONFIRMATION} to clear local data."),
            false,
        ));
    }
    let mut transaction = database
        .pool()
        .begin()
        .await
        .map_err(DatabaseError::from)
        .map_err(DatabaseError::into_command_error)?;
    let watch_activity_removed = sqlx::query("DELETE FROM watch_activity")
        .execute(&mut *transaction)
        .await;
    let library_entries_removed = sqlx::query("DELETE FROM library_entries")
        .execute(&mut *transaction)
        .await;
    let settings_removed = sqlx::query("DELETE FROM settings")
        .execute(&mut *transaction)
        .await;
    let airing_removed = sqlx::query("DELETE FROM airing_data")
        .execute(&mut *transaction)
        .await;
    let media_removed = sqlx::query("DELETE FROM media_snapshots")
        .execute(&mut *transaction)
        .await;
    let response_removed = sqlx::query("DELETE FROM response_caches")
        .execute(&mut *transaction)
        .await;
    let results = match (
        watch_activity_removed,
        library_entries_removed,
        settings_removed,
        airing_removed,
        media_removed,
        response_removed,
    ) {
        (Ok(activity), Ok(library), Ok(settings), Ok(airing), Ok(media), Ok(response)) => (
            activity.rows_affected(),
            library.rows_affected(),
            settings.rows_affected(),
            airing.rows_affected() + media.rows_affected() + response.rows_affected(),
        ),
        _ => {
            transaction
                .rollback()
                .await
                .map_err(DatabaseError::from)
                .map_err(DatabaseError::into_command_error)?;
            return Err(CommandError::new(
                "CLEAR_LOCAL_DATA_FAILED",
                "Animi could not clear local data. No changes were kept.",
                true,
            ));
        }
    };
    transaction
        .commit()
        .await
        .map_err(DatabaseError::from)
        .map_err(DatabaseError::into_command_error)?;
    Ok(ClearLocalDataResponse {
        watch_activity_removed: results.0,
        library_entries_removed: results.1,
        settings_removed: results.2,
        cached_records_removed: results.3,
    })
}

#[cfg(test)]
mod tests {
    use chrono::{DateTime, Utc};
    use serde_json::json;
    use tempfile::tempdir;

    use crate::backups::{ImportBackupRequest, export_backup_to_path_at};
    use crate::database::{
        AiringDataRecord, CacheRecord, Database, DatabasePaths, MediaSnapshotRecord, SettingRecord,
    };
    use crate::domain::{LibraryStatus, MediaKind, UpsertLibraryEntryRequest};

    use super::{
        ClearLocalDataRequest, ImportSessionStore, apply_import, clear_local_data,
        get_settings_overview, import_backup, parse_and_validate, preview_import_from_path,
    };

    fn timestamp(value: &str) -> DateTime<Utc> {
        value.parse().expect("valid timestamp")
    }

    async fn database() -> (tempfile::TempDir, Database) {
        let root = tempdir().expect("temporary root");
        let paths = DatabasePaths::from_root(root.path()).expect("paths");
        let database = Database::connect(&paths).await.expect("database");
        (root, database)
    }

    fn valid_backup() -> serde_json::Value {
        json!({
            "schemaVersion": 1,
            "exportedAt": "2026-07-17T10:00:00.000Z",
            "settings": [{"key":"themePreference","value":"LATTE","updatedAt":"2026-07-17T09:00:00.000Z"}],
            "libraryEntries": [{
                "anilistId": 101, "mediaKind":"ANIME", "status":"WATCHING", "progress":3,
                "activityProgress":5, "score":8.5, "notes":"recovery", "createdAt":"2026-07-17T08:00:00.000Z",
                "updatedAt":"2026-07-17T09:00:00.000Z"
            }],
            "watchActivity": [{
                "eventId":"00000000-0000-4000-a000-000000000001", "anilistId":101, "mediaKind":"ANIME",
                "progressDelta":2, "episodeDurationMinutes":24, "occurredAt":"2026-07-17T08:30:00.000Z"
            }]
        })
    }

    async fn preview_value(
        database: &Database,
        sessions: &ImportSessionStore,
        value: &serde_json::Value,
    ) -> super::ImportPreviewResponse {
        let root = tempdir().expect("backup root");
        let file = root.path().join("backup.json");
        std::fs::write(&file, value.to_string()).expect("backup fixture");
        preview_import_from_path(database, sessions, file)
            .await
            .expect("preview")
    }

    fn library_request(id: i64, progress: i64, notes: &str) -> UpsertLibraryEntryRequest {
        UpsertLibraryEntryRequest {
            anilist_id: id,
            media_kind: MediaKind::Anime,
            status: LibraryStatus::Watching,
            progress,
            volume_progress: None,
            score: Some(8.0),
            notes: Some(notes.to_owned()),
        }
    }

    #[test]
    fn validation_reports_malformed_shape_and_record_errors() {
        let malformed = parse_and_validate("{not json}").expect_err("malformed JSON");
        assert_eq!(malformed[0].code, "BACKUP_JSON_INVALID");

        let mut invalid = valid_backup();
        invalid["libraryEntries"][0]["progress"] = json!(-1);
        invalid["watchActivity"][0]["eventId"] = json!("not-a-uuid");
        let issues = parse_and_validate(&invalid.to_string()).expect_err("invalid records");
        assert!(
            issues
                .iter()
                .any(|issue| issue.code == "BACKUP_PROGRESS_INVALID")
        );
        assert!(
            issues
                .iter()
                .any(|issue| issue.code == "BACKUP_ACTIVITY_ID_INVALID")
        );
    }

    #[tokio::test]
    async fn mid_import_failure_rolls_back_every_write() {
        let (_root, database) = database().await;
        let backup = parse_and_validate(&valid_backup().to_string()).expect("valid backup");
        let sessions = ImportSessionStore::default();
        let file_root = tempdir().expect("backup root");
        let file = file_root.path().join("backup.json");
        std::fs::write(&file, valid_backup().to_string()).expect("backup fixture");
        let preview = preview_import_from_path(&database, &sessions, file)
            .await
            .expect("preview");

        apply_import(&database, &backup, preview.changes, Some(2))
            .await
            .expect_err("forced failure");
        let snapshot = database.backups().snapshot().await.expect("snapshot");
        assert!(snapshot.settings.is_empty());
        assert!(snapshot.library_entries.is_empty());
        assert!(snapshot.watch_activity.is_empty());
    }

    #[test]
    fn duplicate_and_conflicting_activity_ids_are_actionable() {
        let mut duplicate = valid_backup();
        let item = duplicate["watchActivity"][0].clone();
        duplicate["watchActivity"]
            .as_array_mut()
            .expect("array")
            .push(item.clone());
        let issues = parse_and_validate(&duplicate.to_string()).expect_err("duplicate");
        assert!(
            issues
                .iter()
                .any(|issue| issue.code == "BACKUP_ACTIVITY_EVENT_DUPLICATE")
        );

        duplicate["watchActivity"][1]["progressDelta"] = json!(3);
        let issues = parse_and_validate(&duplicate.to_string()).expect_err("conflict");
        assert!(
            issues
                .iter()
                .any(|issue| issue.code == "BACKUP_ACTIVITY_ID_CONFLICT")
        );
        let _ = timestamp("2026-07-17T08:00:00Z");
    }

    #[tokio::test]
    async fn populated_export_clear_and_import_round_trip_preserves_user_state() {
        let (_source_root, source) = database().await;
        source
            .settings()
            .upsert_many(&[SettingRecord {
                key: "themePreference".to_owned(),
                value: json!("MACCHIATO"),
                updated_at: timestamp("2026-07-17T07:00:00Z"),
            }])
            .await
            .expect("source setting");
        source
            .library()
            .upsert_at(
                library_request(101, 2, "round trip"),
                timestamp("2026-07-17T08:00:00Z"),
            )
            .await
            .expect("source baseline");
        source
            .library()
            .upsert_at(
                library_request(101, 4, "round trip"),
                timestamp("2026-07-17T08:30:00Z"),
            )
            .await
            .expect("source progress");

        let export_root = tempdir().expect("export root");
        let file = export_root.path().join("round-trip.json");
        export_backup_to_path_at(&source, file.clone(), timestamp("2026-07-17T09:00:00Z"))
            .await
            .expect("export");

        let (_target_root, target) = database().await;
        let sessions = ImportSessionStore::default();
        let preview = preview_import_from_path(&target, &sessions, file)
            .await
            .expect("preview");
        assert!(preview.can_apply);
        assert_eq!(preview.changes.library_added, 1);
        assert_eq!(preview.changes.activity_added, 2);
        let result = import_backup(
            &target,
            &sessions,
            ImportBackupRequest {
                import_token: preview.import_token.expect("token"),
            },
        )
        .await
        .expect("import");
        assert_eq!(result.changes, preview.changes);

        let source_snapshot = source.backups().snapshot().await.expect("source snapshot");
        let target_snapshot = target.backups().snapshot().await.expect("target snapshot");
        assert_eq!(target_snapshot, source_snapshot);

        let cleared = clear_local_data(
            &target,
            ClearLocalDataRequest {
                confirmation: "CLEAR MY LOCAL DATA".to_owned(),
            },
        )
        .await
        .expect("clear");
        assert_eq!(cleared.library_entries_removed, 1);
        assert_eq!(cleared.watch_activity_removed, 2);
        assert!(
            target
                .backups()
                .snapshot()
                .await
                .expect("empty snapshot")
                .library_entries
                .is_empty()
        );
    }

    #[tokio::test]
    async fn repeated_identical_import_is_idempotent() {
        let (_root, database) = database().await;
        let sessions = ImportSessionStore::default();
        let first = preview_value(&database, &sessions, &valid_backup()).await;
        import_backup(
            &database,
            &sessions,
            ImportBackupRequest {
                import_token: first.import_token.expect("first token"),
            },
        )
        .await
        .expect("first import");

        let second = preview_value(&database, &sessions, &valid_backup()).await;
        assert_eq!(second.changes.activity_added, 0);
        assert_eq!(second.changes.activity_unchanged, 1);
        assert_eq!(second.changes.settings_kept_local, 1);
        assert_eq!(second.changes.library_kept_local, 1);
        import_backup(
            &database,
            &sessions,
            ImportBackupRequest {
                import_token: second.import_token.expect("second token"),
            },
        )
        .await
        .expect("idempotent import");
        assert_eq!(
            database
                .backups()
                .snapshot()
                .await
                .expect("snapshot")
                .watch_activity
                .len(),
            1
        );
    }

    #[tokio::test]
    async fn conflicts_follow_timestamp_and_incarnation_rules_without_deleting_unrelated_entries() {
        let (_root, database) = database().await;
        for (id, progress, at, notes) in [
            (101, 7, "2026-07-17T08:00:00Z", "local 101"),
            (102, 9, "2026-07-17T08:00:00Z", "local 102"),
            (103, 5, "2026-07-17T10:00:00Z", "local 103"),
            (104, 4, "2026-07-17T09:00:00Z", "local 104"),
            (999, 1, "2026-07-17T07:00:00Z", "unrelated"),
        ] {
            database
                .library()
                .upsert_at(library_request(id, progress, notes), timestamp(at))
                .await
                .expect("local entry");
        }
        database
            .settings()
            .upsert_many(&[
                SettingRecord {
                    key: "themePreference".to_owned(),
                    value: json!("LATTE"),
                    updated_at: timestamp("2026-07-17T09:00:00Z"),
                },
                SettingRecord {
                    key: "localOnly".to_owned(),
                    value: json!(true),
                    updated_at: timestamp("2026-07-17T09:00:00Z"),
                },
            ])
            .await
            .expect("local settings");

        let mut backup = valid_backup();
        backup["settings"] = json!([
            {"key":"themePreference","value":"MACCHIATO","updatedAt":"2026-07-17T09:00:00.000Z"},
            {"key":"importedOnly","value":42,"updatedAt":"2026-07-17T10:00:00.000Z"}
        ]);
        backup["libraryEntries"] = json!([
            {"anilistId":101,"mediaKind":"ANIME","status":"COMPLETED","progress":6,"activityProgress":6,"score":9.0,"notes":"import 101","createdAt":"2026-07-17T08:00:00.000Z","updatedAt":"2026-07-17T11:00:00.000Z"},
            {"anilistId":102,"mediaKind":"ANIME","status":"COMPLETED","progress":3,"activityProgress":3,"score":9.0,"notes":"import 102","createdAt":"2026-07-16T08:00:00.000Z","updatedAt":"2026-07-17T11:00:00.000Z"},
            {"anilistId":103,"mediaKind":"ANIME","status":"COMPLETED","progress":4,"activityProgress":8,"score":9.0,"notes":"import 103","createdAt":"2026-07-17T10:00:00.000Z","updatedAt":"2026-07-17T10:00:00.000Z"},
            {"anilistId":104,"mediaKind":"ANIME","status":"COMPLETED","progress":4,"activityProgress":4,"score":9.0,"notes":"import 104","createdAt":"2026-07-17T08:00:00.000Z","updatedAt":"2026-07-17T08:00:00.000Z"}
        ]);
        backup["watchActivity"] = json!([]);
        // Entry 103 intentionally ties the local update but raises the same-incarnation high-water.
        backup["libraryEntries"][2]["createdAt"] = json!("2026-07-17T10:00:00.000Z");

        let sessions = ImportSessionStore::default();
        let preview = preview_value(&database, &sessions, &backup).await;
        assert!(preview.can_apply, "{:?}", preview.validation_errors);
        assert_eq!(preview.changes.settings_added, 1);
        assert_eq!(preview.changes.settings_kept_local, 1);
        assert_eq!(preview.changes.library_conflicts, 4);
        assert_eq!(preview.changes.library_merged, 3);
        assert_eq!(preview.changes.library_kept_local, 1);
        assert_eq!(preview.changes.unrelated_local_library_entries_retained, 1);
        import_backup(
            &database,
            &sessions,
            ImportBackupRequest {
                import_token: preview.import_token.expect("token"),
            },
        )
        .await
        .expect("import");

        let snapshot = database.backups().snapshot().await.expect("snapshot");
        assert_eq!(snapshot.library_entries.len(), 5);
        let find = |id| {
            snapshot
                .library_entries
                .iter()
                .find(|entry| entry.anilist_id == id)
                .expect("entry")
        };
        assert_eq!(find(101).notes.as_deref(), Some("import 101"));
        assert_eq!(
            find(101).activity_progress,
            7,
            "same incarnation keeps greatest high-water"
        );
        assert_eq!(
            find(102).activity_progress,
            3,
            "winning different incarnation keeps its high-water"
        );
        assert_eq!(find(103).notes.as_deref(), Some("local 103"));
        assert_eq!(
            find(103).activity_progress,
            8,
            "older import can raise same-incarnation high-water"
        );
        assert_eq!(
            find(104).notes.as_deref(),
            Some("local 104"),
            "newer local incarnation stays local"
        );
        assert_eq!(find(999).notes.as_deref(), Some("unrelated"));
        assert_eq!(
            database
                .settings()
                .get("themePreference")
                .await
                .expect("theme")
                .expect("theme")
                .value,
            json!("LATTE")
        );
        assert_eq!(
            database
                .settings()
                .get("importedOnly")
                .await
                .expect("imported")
                .expect("imported")
                .value,
            json!(42)
        );
        assert!(
            database
                .settings()
                .get("localOnly")
                .await
                .expect("local")
                .is_some()
        );
    }

    #[tokio::test]
    async fn stale_preview_and_new_activity_identity_conflicts_make_no_changes() {
        let (_root, database) = database().await;
        let sessions = ImportSessionStore::default();
        let stale = preview_value(&database, &sessions, &valid_backup()).await;
        database
            .settings()
            .upsert_many(&[SettingRecord {
                key: "themePreference".to_owned(),
                value: json!("SYSTEM"),
                updated_at: timestamp("2026-07-17T12:00:00Z"),
            }])
            .await
            .expect("local change");
        let error = import_backup(
            &database,
            &sessions,
            ImportBackupRequest {
                import_token: stale.import_token.expect("stale token"),
            },
        )
        .await
        .expect_err("stale preview");
        assert_eq!(error.code, "IMPORT_PREVIEW_STALE");
        assert!(database.library().list().await.expect("library").is_empty());

        let fresh = preview_value(&database, &sessions, &valid_backup()).await;
        sqlx::query(
            "INSERT INTO watch_activity (event_id, anilist_id, media_kind, progress_delta, episode_duration_minutes, occurred_at) VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind("00000000-0000-4000-a000-000000000001")
        .bind(999_i64).bind("ANIME").bind(1_i64).bind(24_i64).bind("2026-07-17T08:30:00.000Z")
        .execute(database.pool()).await.expect("conflicting local event");
        let error = import_backup(
            &database,
            &sessions,
            ImportBackupRequest {
                import_token: fresh.import_token.expect("fresh token"),
            },
        )
        .await
        .expect_err("activity conflict");
        assert_eq!(error.code, "BACKUP_ACTIVITY_ID_CONFLICT");
        assert!(database.library().list().await.expect("library").is_empty());
    }

    #[tokio::test]
    async fn malformed_unsupported_and_invalid_backups_never_create_import_sessions() {
        let (_root, database) = database().await;
        let sessions = ImportSessionStore::default();
        for (contents, expected) in [
            ("{not json}".to_owned(), "BACKUP_JSON_INVALID"),
            (
                json!({"schemaVersion":9}).to_string(),
                "BACKUP_SCHEMA_VERSION_UNSUPPORTED",
            ),
        ] {
            let root = tempdir().expect("backup root");
            let file = root.path().join("invalid.json");
            std::fs::write(&file, contents).expect("fixture");
            let preview = preview_import_from_path(&database, &sessions, file)
                .await
                .expect("invalid preview");
            assert!(!preview.can_apply);
            assert!(preview.import_token.is_none());
            assert_eq!(preview.validation_errors[0].code, expected);
        }

        let mut invalid = valid_backup();
        invalid["settings"][0]["value"] = json!("NEON");
        invalid["libraryEntries"][0]["status"] = json!("PAUSED");
        let preview = preview_value(&database, &sessions, &invalid).await;
        assert!(!preview.can_apply);
        assert!(
            preview
                .validation_errors
                .iter()
                .any(|issue| issue.code == "BACKUP_SHAPE_INVALID")
        );
        assert!(
            database
                .backups()
                .snapshot()
                .await
                .expect("unchanged")
                .library_entries
                .is_empty()
        );

        let missing_root = tempdir().expect("missing root");
        let missing = missing_root.path().join("missing.json");
        let error = preview_import_from_path(&database, &sessions, missing)
            .await
            .expect_err("unreadable file");
        assert_eq!(error.code, "BACKUP_READ_FAILED");
    }

    #[tokio::test]
    async fn empty_backup_round_trip_is_a_valid_no_op() {
        let (_root, database) = database().await;
        let sessions = ImportSessionStore::default();
        let empty = json!({
            "schemaVersion": 1,
            "exportedAt": "2026-07-17T09:00:00.000Z",
            "settings": [],
            "libraryEntries": [],
            "watchActivity": []
        });
        let preview = preview_value(&database, &sessions, &empty).await;
        assert!(preview.can_apply);
        assert_eq!(preview.changes, super::ImportChangeSummary::default());
        let result = import_backup(
            &database,
            &sessions,
            ImportBackupRequest {
                import_token: preview.import_token.expect("token"),
            },
        )
        .await
        .expect("empty import");
        assert_eq!(result.changes, preview.changes);
        assert!(
            database
                .backups()
                .snapshot()
                .await
                .expect("snapshot")
                .library_entries
                .is_empty()
        );
    }

    #[tokio::test]
    async fn clear_requires_the_exact_confirmation() {
        let (_root, database) = database().await;
        database
            .library()
            .upsert_at(
                library_request(101, 1, "keep"),
                timestamp("2026-07-17T08:00:00Z"),
            )
            .await
            .expect("entry");
        let error = clear_local_data(
            &database,
            ClearLocalDataRequest {
                confirmation: "clear".to_owned(),
            },
        )
        .await
        .expect_err("confirmation");
        assert_eq!(error.code, "CLEAR_CONFIRMATION_REQUIRED");
        assert_eq!(database.library().list().await.expect("preserved").len(), 1);
    }

    #[tokio::test]
    async fn settings_overview_reports_live_cache_freshness() {
        let (_root, database) = database().await;
        let now = Utc::now();
        database
            .media()
            .upsert_snapshot(&MediaSnapshotRecord {
                anilist_id: 101,
                media_kind: MediaKind::Anime,
                title: "Cached title".to_owned(),
                cover_url: None,
                media_format: Some("TV".to_owned()),
                genres: Vec::new(),
                tags: Vec::new(),
                media_status: Some("RELEASING".to_owned()),
                total_units: Some(12),
                volume_total: None,
                duration_minutes: Some(24),
                release_date: None,
                cached_at: now - chrono::Duration::hours(25),
            })
            .await
            .expect("media cache");
        database
            .media()
            .upsert_airing(&AiringDataRecord {
                anilist_id: 101,
                media_kind: MediaKind::Anime,
                episode_number: 3,
                airing_at: now + chrono::Duration::days(1),
                fetched_at: now - chrono::Duration::minutes(31),
            })
            .await
            .expect("airing cache");
        database
            .cache()
            .upsert(&CacheRecord {
                cache_key: "settings-overview".to_owned(),
                payload: json!({"items": []}),
                fetched_at: now,
                expires_at: now + chrono::Duration::hours(6),
            })
            .await
            .expect("response cache");

        let overview = get_settings_overview(&database).await.expect("overview");
        assert_eq!(overview.cached_media_count, 1);
        assert_eq!(overview.stale_media_count, 1);
        assert_eq!(overview.airing_record_count, 1);
        assert_eq!(overview.stale_airing_count, 1);
        assert_eq!(overview.response_cache_count, 1);
        assert_eq!(overview.stale_response_cache_count, 0);
        assert!(overview.latest_cache_update_at.is_some());
    }

    #[test]
    fn schema_one_and_two_fixtures_normalize_to_lossless_mixed_media_records() {
        let schema_one = parse_and_validate(include_str!(
            "../../tests/fixtures/backups/v1-mixed-profile.json"
        ))
        .expect("valid schema 1 fixture");
        assert_eq!(schema_one.schema_version, 1);
        let schema_one_manga = schema_one
            .library_entries
            .iter()
            .find(|entry| entry.source.media_kind == MediaKind::Manga)
            .expect("schema 1 manga");
        assert_eq!(schema_one_manga.source.volume_progress, Some(0));
        assert_eq!(schema_one_manga.source.activity_volume_progress, Some(0));
        assert_eq!(schema_one.watch_activity[1].source.volume_delta, 0);

        let schema_two = parse_and_validate(include_str!(
            "../../tests/fixtures/backups/v2-mixed-profile.json"
        ))
        .expect("valid schema 2 fixture");
        assert_eq!(schema_two.schema_version, 2);
        let manga = schema_two
            .library_entries
            .iter()
            .find(|entry| entry.source.media_kind == MediaKind::Manga)
            .expect("schema 2 manga");
        assert_eq!(manga.source.volume_progress, Some(3));
        assert_eq!(manga.source.activity_volume_progress, Some(4));
        assert_eq!(schema_two.watch_activity[1].source.volume_delta, 3);
    }

    #[test]
    fn schema_one_rejects_an_impossible_manga_duration_snapshot() {
        let mut value: serde_json::Value = serde_json::from_str(include_str!(
            "../../tests/fixtures/backups/v1-mixed-profile.json"
        ))
        .expect("fixture JSON");
        value["watchActivity"][1]["episodeDurationMinutes"] = json!(24);
        let issues = parse_and_validate(&value.to_string()).expect_err("invalid manga duration");
        assert!(issues.iter().any(|issue| {
            issue.code == "BACKUP_ACTIVITY_KIND_INVALID"
                && issue.path.as_deref() == Some("watchActivity[1].episodeDurationMinutes")
        }));
    }

    #[tokio::test]
    async fn schema_two_mixed_fixture_imports_idempotently() {
        let (_root, database) = database().await;
        let sessions = ImportSessionStore::default();
        let value: serde_json::Value = serde_json::from_str(include_str!(
            "../../tests/fixtures/backups/v2-mixed-profile.json"
        ))
        .expect("fixture JSON");

        let first_preview = preview_value(&database, &sessions, &value).await;
        assert!(first_preview.can_apply);
        assert_eq!(first_preview.changes.library_added, 2);
        assert_eq!(first_preview.changes.activity_added, 2);
        import_backup(
            &database,
            &sessions,
            ImportBackupRequest {
                import_token: first_preview.import_token.expect("first token"),
            },
        )
        .await
        .expect("first import");

        let entries = database.library().list().await.expect("mixed library");
        assert_eq!(entries.len(), 2);
        let manga = entries
            .iter()
            .find(|entry| entry.media_kind == MediaKind::Manga)
            .expect("manga entry");
        assert_eq!((manga.progress, manga.volume_progress), (18, Some(3)));
        let activity = database.activity().list().await.expect("mixed activity");
        assert_eq!(activity.len(), 2);
        assert_eq!(activity[0].volume_delta, 3);

        let repeated = preview_value(&database, &sessions, &value).await;
        assert!(repeated.can_apply);
        assert_eq!(repeated.changes.activity_unchanged, 2);
        assert_eq!(repeated.changes.activity_added, 0);
        import_backup(
            &database,
            &sessions,
            ImportBackupRequest {
                import_token: repeated.import_token.expect("repeat token"),
            },
        )
        .await
        .expect("idempotent import");
        assert_eq!(database.activity().list().await.expect("activity").len(), 2);
    }
}
