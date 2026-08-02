use std::future::Future;
use std::path::PathBuf;
use std::pin::Pin;
use std::sync::Arc;

use tauri::{AppHandle, Runtime, State};
use tauri_plugin_dialog::DialogExt;

use crate::backups::{
    ClearLocalDataRequest, ClearLocalDataResponse, ExportBackupResponse, ImportBackupRequest,
    ImportBackupResponse, ImportPreviewResponse, ImportSessionStore, SettingsOverview,
};
use crate::database::Database;
use crate::domain::CommandError;

type PickerFuture =
    Pin<Box<dyn Future<Output = Result<Option<PathBuf>, CommandError>> + Send + 'static>>;
type Picker = dyn Fn() -> PickerFuture + Send + Sync + 'static;

/// Rust owns the native save dialog so the webview never receives or submits
/// an arbitrary filesystem path. Tests replace the picker with a temporary
/// path while exercising the same command.
#[derive(Clone)]
pub struct BackupDestinationPicker {
    pick: Arc<Picker>,
}

/// Rust-owned native source picker for recovery. As with export, the webview
/// receives neither a path nor arbitrary filesystem authority.
#[derive(Clone)]
pub struct BackupSourcePicker {
    pick: Arc<Picker>,
}

impl BackupSourcePicker {
    pub fn for_app<R: Runtime>(app: AppHandle<R>) -> Self {
        Self {
            pick: Arc::new(move || {
                let app = app.clone();
                Box::pin(async move {
                    let (sender, receiver) = tokio::sync::oneshot::channel();
                    app.dialog()
                        .file()
                        .add_filter("Animi backup", &["json"])
                        .set_title("Preview Animi backup")
                        .pick_file(move |selection| {
                            let _ = sender.send(selection);
                        });
                    let selection = receiver.await.map_err(|_| {
                        CommandError::new(
                            "BACKUP_DIALOG_FAILED",
                            "Animi could not open the backup file picker.",
                            true,
                        )
                    })?;
                    selection
                        .map(|path| {
                            path.into_path().map_err(|_| {
                                CommandError::new(
                                    "BACKUP_PATH_INVALID",
                                    "Choose a local backup file.",
                                    false,
                                )
                            })
                        })
                        .transpose()
                })
            }),
        }
    }

    #[doc(hidden)]
    pub fn fixed_path(path: Option<PathBuf>) -> Self {
        Self {
            pick: Arc::new(move || {
                let path = path.clone();
                Box::pin(async move { Ok(path) })
            }),
        }
    }

    async fn choose(&self) -> Result<Option<PathBuf>, CommandError> {
        (self.pick)().await
    }
}

impl BackupDestinationPicker {
    pub fn for_app<R: Runtime>(app: AppHandle<R>) -> Self {
        Self {
            pick: Arc::new(move || {
                let app = app.clone();
                Box::pin(async move {
                    let (sender, receiver) = tokio::sync::oneshot::channel();
                    app.dialog()
                        .file()
                        .add_filter("Animi backup", &["json"])
                        .set_file_name("animi-backup.json")
                        .set_title("Export Animi backup")
                        .save_file(move |selection| {
                            let _ = sender.send(selection);
                        });

                    let selection = receiver.await.map_err(|_| {
                        CommandError::new(
                            "BACKUP_DIALOG_FAILED",
                            "Animi could not open the backup file picker.",
                            true,
                        )
                    })?;
                    selection
                        .map(|path| {
                            path.into_path().map_err(|_| {
                                CommandError::new(
                                    "BACKUP_PATH_INVALID",
                                    "Choose a local filesystem location for the backup.",
                                    false,
                                )
                            })
                        })
                        .transpose()
                })
            }),
        }
    }

    /// Test seam for Tauri's mock runtime. Production constructs the picker
    /// only with `for_app`, which opens a user-mediated native dialog.
    #[doc(hidden)]
    pub fn fixed_path(path: Option<PathBuf>) -> Self {
        Self {
            pick: Arc::new(move || {
                let path = path.clone();
                Box::pin(async move { Ok(path) })
            }),
        }
    }

    async fn choose(&self) -> Result<Option<PathBuf>, CommandError> {
        (self.pick)().await
    }
}

#[tauri::command(rename_all = "camelCase")]
pub async fn export_backup(
    database: State<'_, Database>,
    destination_picker: State<'_, BackupDestinationPicker>,
) -> Result<Option<ExportBackupResponse>, CommandError> {
    export_backup_with_database(&database, &destination_picker).await
}

pub(crate) async fn export_backup_with_database(
    database: &Database,
    destination_picker: &BackupDestinationPicker,
) -> Result<Option<ExportBackupResponse>, CommandError> {
    let Some(destination_path) = destination_picker.choose().await? else {
        return Ok(None);
    };

    crate::backups::export_backup_to_path(database, destination_path)
        .await
        .map(Some)
}

#[tauri::command(rename_all = "camelCase")]
pub async fn preview_import(
    database: State<'_, Database>,
    source_picker: State<'_, BackupSourcePicker>,
    sessions: State<'_, ImportSessionStore>,
) -> Result<Option<ImportPreviewResponse>, CommandError> {
    let Some(source_path) = source_picker.choose().await? else {
        return Ok(None);
    };
    crate::backups::preview_import_from_path(&database, &sessions, source_path)
        .await
        .map(Some)
}

#[tauri::command(rename_all = "camelCase")]
pub async fn import_backup(
    database: State<'_, Database>,
    sessions: State<'_, ImportSessionStore>,
    request: ImportBackupRequest,
) -> Result<ImportBackupResponse, CommandError> {
    crate::backups::import_backup(&database, &sessions, request).await
}

#[tauri::command(rename_all = "camelCase")]
pub async fn get_settings_overview(
    database: State<'_, Database>,
) -> Result<SettingsOverview, CommandError> {
    crate::backups::get_settings_overview(&database).await
}

#[tauri::command(rename_all = "camelCase")]
pub async fn clear_local_data(
    database: State<'_, Database>,
    request: ClearLocalDataRequest,
) -> Result<ClearLocalDataResponse, CommandError> {
    crate::backups::clear_local_data(&database, request).await
}

#[cfg(test)]
mod tests {
    use tempfile::tempdir;

    use crate::database::{Database, DatabasePaths};

    use super::{BackupDestinationPicker, export_backup_with_database};

    async fn test_database() -> (tempfile::TempDir, Database) {
        let database_root = tempdir().expect("temporary database root");
        let paths = DatabasePaths::from_root(database_root.path()).expect("valid database path");
        let database = Database::connect(&paths).await.expect("test database");
        (database_root, database)
    }

    #[tokio::test]
    async fn command_exports_to_an_injected_temporary_path() {
        let (_database_root, database) = test_database().await;
        let export_root = tempdir().expect("temporary export root");
        let destination = export_root.path().join("animi-backup.json");
        let picker = BackupDestinationPicker::fixed_path(Some(destination.clone()));

        let response = export_backup_with_database(&database, &picker)
            .await
            .expect("export command")
            .expect("selected destination");

        assert_eq!(response.schema_version, 2);
        assert!(destination.is_file());
    }

    #[tokio::test]
    async fn cancelling_the_native_picker_makes_no_file_or_database_change() {
        let (_database_root, database) = test_database().await;
        let picker = BackupDestinationPicker::fixed_path(None);

        assert_eq!(
            export_backup_with_database(&database, &picker)
                .await
                .expect("cancelled command"),
            None
        );
        assert!(database.library().list().await.expect("library").is_empty());
    }
}
