use tauri::State;

use crate::database::Database;
use crate::domain::{
    AdvanceLibraryProgressRequest, CommandError, LibraryEntry, LibraryItem,
    RemoveLibraryEntryRequest, RemoveLibraryEntryResponse, UpsertLibraryEntryRequest,
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

#[tauri::command(rename_all = "camelCase")]
pub async fn advance_library_progress(
    database: State<'_, Database>,
    request: AdvanceLibraryProgressRequest,
) -> Result<LibraryEntry, CommandError> {
    advance_library_progress_with_database(&database, request).await
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

pub(crate) async fn advance_library_progress_with_database(
    database: &Database,
    request: AdvanceLibraryProgressRequest,
) -> Result<LibraryEntry, CommandError> {
    database
        .library()
        .advance_home_progress(request.anilist_id)
        .await
}

#[cfg(test)]
mod tests {
    use tempfile::tempdir;

    use chrono::Utc;

    use crate::database::{Database, DatabasePaths, MediaSnapshotRecord, MediaTag};
    use crate::domain::{
        AdvanceLibraryProgressRequest, LibraryStatus, MediaKind, RemoveLibraryEntryRequest,
        UpsertLibraryEntryRequest,
    };

    use super::{
        advance_library_progress_with_database, list_library_with_database,
        remove_library_entry_with_database, upsert_library_entry_with_database,
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

    #[tokio::test]
    async fn home_progress_advances_to_the_known_total_without_a_schedule() {
        let (_root, database) = test_database().await;
        let now = Utc::now();
        database
            .media()
            .upsert_snapshot(&MediaSnapshotRecord {
                anilist_id: 21,
                media_kind: MediaKind::Anime,
                title: "Airing Test".to_owned(),
                cover_url: None,
                media_format: Some("TV".to_owned()),
                genres: vec!["Drama".to_owned()],
                tags: vec![MediaTag {
                    name: "Time".to_owned(),
                    rank: 80,
                }],
                media_status: Some("RELEASING".to_owned()),
                total_units: Some(3),
                duration_minutes: Some(24),
                cached_at: now,
            })
            .await
            .expect("snapshot");
        let mut tracked = request();
        tracked.status = LibraryStatus::Watching;
        tracked.progress = 1;
        database
            .library()
            .upsert(tracked)
            .await
            .expect("tracked entry");
        let request = AdvanceLibraryProgressRequest { anilist_id: 21 };
        let first = advance_library_progress_with_database(&database, request)
            .await
            .expect("episode 2");
        let second = advance_library_progress_with_database(&database, request)
            .await
            .expect("episode 3");
        assert_eq!((first.progress, second.progress), (2, 3));
        let error = advance_library_progress_with_database(&database, request)
            .await
            .expect_err("episode total has been reached");
        assert_eq!(error.code, "PROGRESS_NOT_ADVANCEABLE");
        assert_eq!(database.activity().list().await.expect("activity").len(), 2);
    }

    #[tokio::test]
    async fn home_progress_requires_a_known_episode_total() {
        let (_root, database) = test_database().await;
        database
            .media()
            .upsert_snapshot(&MediaSnapshotRecord {
                anilist_id: 21,
                media_kind: MediaKind::Anime,
                title: "Unknown Total".to_owned(),
                cover_url: None,
                media_format: Some("TV".to_owned()),
                genres: vec![],
                tags: vec![],
                media_status: Some("RELEASING".to_owned()),
                total_units: None,
                duration_minutes: Some(24),
                cached_at: Utc::now(),
            })
            .await
            .expect("snapshot");
        let mut tracked = request();
        tracked.status = LibraryStatus::Watching;
        database
            .library()
            .upsert(tracked)
            .await
            .expect("tracked entry");

        let error = advance_library_progress_with_database(
            &database,
            AdvanceLibraryProgressRequest { anilist_id: 21 },
        )
        .await
        .expect_err("unknown episode totals are not guessed");
        assert_eq!(error.code, "EPISODE_TOTAL_UNKNOWN");
    }
}
