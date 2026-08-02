use std::future::Future;

use chrono::{DateTime, Utc};
use sqlx::SqlitePool;

use crate::domain::{LibraryItem, WatchActivity};

use super::{ActivityRepository, DatabaseError, LibraryRepository};

#[derive(Debug, Clone)]
pub struct DashboardRepository {
    pool: SqlitePool,
}

impl DashboardRepository {
    pub(super) fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn load(&self) -> Result<(Vec<LibraryItem>, Vec<WatchActivity>), DatabaseError> {
        self.load_at_with(Utc::now(), || async {}).await
    }

    async fn load_at_with<F, Fut>(
        &self,
        now: DateTime<Utc>,
        after_library_read: F,
    ) -> Result<(Vec<LibraryItem>, Vec<WatchActivity>), DatabaseError>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = ()>,
    {
        let mut transaction = self.pool.begin().await?;
        let library_items = LibraryRepository::list_items_on(&mut transaction, now).await?;
        after_library_read().await;
        let activity = ActivityRepository::list_on(&mut transaction).await?;
        transaction.commit().await?;
        Ok((library_items, activity))
    }
}

#[cfg(test)]
mod tests {
    use tempfile::tempdir;

    use crate::database::{Database, DatabasePaths};
    use crate::domain::{LibraryStatus, MediaKind, UpsertLibraryEntryRequest};

    fn request(progress: i64) -> UpsertLibraryEntryRequest {
        UpsertLibraryEntryRequest {
            anilist_id: 101,
            media_kind: MediaKind::Anime,
            status: LibraryStatus::Watching,
            progress,
            volume_progress: None,
            score: None,
            notes: None,
        }
    }

    #[tokio::test]
    async fn combined_read_remains_on_one_snapshot_during_a_progress_commit() {
        let root = tempdir().expect("temporary database root");
        let paths = DatabasePaths::from_root(root.path()).expect("valid paths");
        let database = Database::connect(&paths).await.expect("database");
        database
            .library()
            .upsert(request(0))
            .await
            .expect("baseline entry");
        let repository = database.dashboard();
        let journal_mode = sqlx::query_scalar::<_, String>("PRAGMA journal_mode = WAL")
            .fetch_one(&repository.pool)
            .await
            .expect("WAL test mode");
        assert_eq!(journal_mode.to_ascii_lowercase(), "wal");

        let update_database = database.clone();
        let (library_items, activity) = repository
            .load_at_with(
                "2026-07-17T08:00:00Z".parse().expect("read time"),
                move || async move {
                    update_database
                        .library()
                        .upsert(request(1))
                        .await
                        .expect("concurrent progress commit");
                },
            )
            .await
            .expect("snapshot read");

        assert_eq!(library_items[0].entry.progress, 0);
        assert!(activity.is_empty());
        assert_eq!(
            database.library().list().await.expect("current library")[0].progress,
            1
        );
        assert_eq!(
            database
                .activity()
                .list()
                .await
                .expect("current activity")
                .len(),
            1
        );
    }
}
