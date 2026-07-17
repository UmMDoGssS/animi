use tauri::State;

use crate::database::Database;
use crate::domain::{CommandError, DashboardInsights};

#[tauri::command(rename_all = "camelCase")]
pub async fn get_dashboard_insights(
    database: State<'_, Database>,
) -> Result<DashboardInsights, CommandError> {
    get_dashboard_insights_with_database(&database).await
}

pub(crate) async fn get_dashboard_insights_with_database(
    database: &Database,
) -> Result<DashboardInsights, CommandError> {
    let (library_items, activity) = database
        .dashboard()
        .load()
        .await
        .map_err(|error| error.into_command_error())?;
    DashboardInsights::build(library_items, activity)
}

#[cfg(test)]
mod tests {
    use tempfile::tempdir;

    use crate::database::{Database, DatabasePaths, MediaSnapshotRecord, MediaTag};
    use crate::domain::{LibraryStatus, MediaKind, UpsertLibraryEntryRequest};

    use super::get_dashboard_insights_with_database;

    #[tokio::test]
    async fn command_returns_authoritative_library_preferences_and_activity() {
        let root = tempdir().expect("temporary database root");
        let paths = DatabasePaths::from_root(root.path()).expect("valid paths");
        let database = Database::connect(&paths).await.expect("database");
        database
            .media()
            .upsert_snapshot(&MediaSnapshotRecord {
                anilist_id: 101,
                media_kind: MediaKind::Anime,
                title: "A Test Anime".to_owned(),
                cover_url: None,
                media_format: Some("TV".to_owned()),
                genres: vec!["Drama".to_owned()],
                tags: vec![MediaTag {
                    name: "Found Family".to_owned(),
                    rank: 90,
                }],
                media_status: Some("RELEASING".to_owned()),
                total_units: Some(12),
                duration_minutes: Some(24),
                release_date: None,
                cached_at: "2026-07-16T08:00:00Z".parse().expect("snapshot time"),
            })
            .await
            .expect("snapshot");
        for progress in [0, 2] {
            database
                .library()
                .upsert(UpsertLibraryEntryRequest {
                    anilist_id: 101,
                    media_kind: MediaKind::Anime,
                    status: LibraryStatus::Watching,
                    progress,
                    score: None,
                    notes: None,
                })
                .await
                .expect("library update");
        }

        let insights = get_dashboard_insights_with_database(&database)
            .await
            .expect("dashboard insights");

        assert_eq!(insights.library_totals.tracked, 1);
        assert_eq!(insights.library_totals.watching, 1);
        assert_eq!(insights.preference_signals.genres[0].label, "Drama");
        assert_eq!(insights.preference_signals.tags[0].label, "Found Family");
        assert_eq!(insights.activity.event_count, 1);
        assert_eq!(insights.activity.recorded_episodes, 2);
        assert_eq!(insights.activity.recorded_minutes, 48);
    }

    #[tokio::test]
    async fn database_failures_keep_the_structured_error_contract() {
        let root = tempdir().expect("temporary database root");
        let paths = DatabasePaths::from_root(root.path()).expect("valid paths");
        let database = Database::connect(&paths).await.expect("database");
        database.clone().close().await;

        let error = get_dashboard_insights_with_database(&database)
            .await
            .expect_err("closed database should fail");

        assert_eq!(error.code, "DATABASE_UNAVAILABLE");
        assert!(error.retryable);
    }
}
