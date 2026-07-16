use tauri::State;

use crate::database::Database;
use crate::domain::{
    CommandError, LibraryEntry, LibraryItem, RemoveLibraryEntryRequest, RemoveLibraryEntryResponse,
    UpsertLibraryEntryRequest,
};

#[tauri::command(rename_all = "camelCase")]
pub async fn list_library(database: State<'_, Database>) -> Result<Vec<LibraryItem>, CommandError> {
    list_library_with_database(&database).await
}

#[tauri::command(rename_all = "camelCase")]
pub async fn upsert_library_entry(
    database: State<'_, Database>,
    request: UpsertLibraryEntryRequest,
) -> Result<LibraryEntry, CommandError> {
    upsert_library_entry_with_database(&database, request).await
}

#[tauri::command(rename_all = "camelCase")]
pub async fn remove_library_entry(
    database: State<'_, Database>,
    request: RemoveLibraryEntryRequest,
) -> Result<RemoveLibraryEntryResponse, CommandError> {
    remove_library_entry_with_database(&database, request).await
}

pub(crate) async fn list_library_with_database(
    database: &Database,
) -> Result<Vec<LibraryItem>, CommandError> {
    database
        .library()
        .list_items()
        .await
        .map_err(|error| error.into_command_error())
}

pub(crate) async fn upsert_library_entry_with_database(
    database: &Database,
    request: UpsertLibraryEntryRequest,
) -> Result<LibraryEntry, CommandError> {
    database.library().upsert(request).await
}

pub(crate) async fn remove_library_entry_with_database(
    database: &Database,
    request: RemoveLibraryEntryRequest,
) -> Result<RemoveLibraryEntryResponse, CommandError> {
    request.validate()?;
    let removed = database
        .library()
        .remove(request.anilist_id, request.media_kind)
        .await
        .map_err(|error| error.into_command_error())?;
    Ok(RemoveLibraryEntryResponse { removed })
}

#[cfg(test)]
mod tests {
    use tempfile::tempdir;

    use crate::database::{Database, DatabasePaths};
    use crate::domain::{
        LibraryStatus, MediaKind, RemoveLibraryEntryRequest, UpsertLibraryEntryRequest,
    };

    use super::{
        list_library_with_database, remove_library_entry_with_database,
        upsert_library_entry_with_database,
    };

    async fn test_database() -> (tempfile::TempDir, Database) {
        let root = tempdir().expect("temporary database root");
        let paths = DatabasePaths::from_root(root.path()).expect("valid database path");
        let database = Database::connect(&paths).await.expect("test database");
        (root, database)
    }

    fn request() -> UpsertLibraryEntryRequest {
        UpsertLibraryEntryRequest {
            anilist_id: 21,
            media_kind: MediaKind::Anime,
            status: LibraryStatus::Planning,
            progress: 0,
            score: None,
            notes: None,
        }
    }

    #[tokio::test]
    async fn command_boundary_creates_lists_and_explicitly_removes() {
        let (_root, database) = test_database().await;
        let created = upsert_library_entry_with_database(&database, request())
            .await
            .expect("create command");
        assert_eq!(created.anilist_id, 21);
        let listed = list_library_with_database(&database)
            .await
            .expect("list command");
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].entry.anilist_id, 21);
        assert!(listed[0].media.is_none());

        let response = remove_library_entry_with_database(
            &database,
            RemoveLibraryEntryRequest {
                anilist_id: 21,
                media_kind: MediaKind::Anime,
                confirmed: true,
            },
        )
        .await
        .expect("remove command");
        assert!(response.removed);
    }

    #[tokio::test]
    async fn removal_without_confirmation_is_rejected_without_writing() {
        let (_root, database) = test_database().await;
        upsert_library_entry_with_database(&database, request())
            .await
            .expect("create command");

        let error = remove_library_entry_with_database(
            &database,
            RemoveLibraryEntryRequest {
                anilist_id: 21,
                media_kind: MediaKind::Anime,
                confirmed: false,
            },
        )
        .await
        .expect_err("confirmation is required");
        assert_eq!(error.code, "CONFIRMATION_REQUIRED");
        assert_eq!(
            list_library_with_database(&database)
                .await
                .expect("list command")
                .len(),
            1
        );
    }

    #[tokio::test]
    async fn database_failures_use_the_stable_command_error_envelope() {
        let (_root, database) = test_database().await;
        database.clone().close().await;

        let error = list_library_with_database(&database)
            .await
            .expect_err("a closed database should fail");
        assert_eq!(error.code, "DATABASE_UNAVAILABLE");
        assert!(error.retryable);
    }
}
