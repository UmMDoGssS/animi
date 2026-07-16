use animi_lib::database::{
    AiringDataRecord, CacheRecord, Database, DatabasePaths, MediaSnapshotRecord, MediaTag,
    SettingRecord,
};
use animi_lib::domain::{LibraryStatus, MediaKind, UpsertLibraryEntryRequest};
use chrono::{DateTime, Utc};
use serde_json::json;
use tempfile::tempdir;

fn library_request(progress: i64, status: LibraryStatus) -> UpsertLibraryEntryRequest {
    UpsertLibraryEntryRequest {
        anilist_id: 101,
        media_kind: MediaKind::Anime,
        status,
        progress,
        score: Some(8.5),
        notes: Some("  comfort show  ".to_owned()),
    }
}

fn snapshot(total_units: Option<i64>) -> MediaSnapshotRecord {
    MediaSnapshotRecord {
        anilist_id: 101,
        media_kind: MediaKind::Anime,
        title: "A Test Anime".to_owned(),
        cover_url: Some("https://example.invalid/cover.jpg".to_owned()),
        media_format: Some("TV".to_owned()),
        genres: vec!["Slice of Life".to_owned()],
        tags: vec![MediaTag {
            name: "Cats".to_owned(),
            rank: 99,
        }],
        media_status: Some("FINISHED".to_owned()),
        total_units,
        cached_at: timestamp("2026-07-16T08:00:00.000Z"),
    }
}

fn timestamp(value: &str) -> DateTime<Utc> {
    value.parse().expect("valid UTC test timestamp")
}

#[tokio::test]
async fn repository_crud_status_transition_and_restart_persistence() {
    let root = tempdir().expect("temporary database root");
    let paths = DatabasePaths::from_root(root.path()).expect("valid paths");
    let database = Database::connect(&paths).await.expect("database");
    database
        .media()
        .upsert_snapshot(&snapshot(Some(12)))
        .await
        .expect("snapshot");

    let created = database
        .library()
        .upsert(library_request(2, LibraryStatus::Watching))
        .await
        .expect("created entry");
    assert_eq!(created.notes.as_deref(), Some("comfort show"));

    let updated = database
        .library()
        .upsert(library_request(12, LibraryStatus::Completed))
        .await
        .expect("updated entry");
    assert_eq!(updated.status, LibraryStatus::Completed);
    assert_eq!(updated.created_at, created.created_at);
    assert!(updated.updated_at >= created.updated_at);

    database.close().await;
    let reopened = Database::connect(&paths).await.expect("reopened database");
    let entries = reopened.library().list().await.expect("persisted entries");
    assert_eq!(entries, vec![updated]);
    assert!(
        reopened
            .library()
            .remove(101, MediaKind::Anime)
            .await
            .expect("remove")
    );
    assert!(
        reopened
            .library()
            .list()
            .await
            .expect("empty list")
            .is_empty()
    );
}

#[tokio::test]
async fn progress_bounds_use_known_totals_and_allow_unknown_totals() {
    let root = tempdir().expect("temporary database root");
    let paths = DatabasePaths::from_root(root.path()).expect("valid paths");
    let database = Database::connect(&paths).await.expect("database");
    database
        .media()
        .upsert_snapshot(&snapshot(Some(12)))
        .await
        .expect("snapshot");

    let error = database
        .library()
        .upsert(library_request(13, LibraryStatus::Watching))
        .await
        .expect_err("known total should bound progress");
    assert_eq!(error.code, "PROGRESS_EXCEEDS_TOTAL");
    assert!(database.library().list().await.expect("library").is_empty());

    database
        .media()
        .upsert_snapshot(&snapshot(None))
        .await
        .expect("unknown total snapshot");
    database
        .library()
        .upsert(library_request(13, LibraryStatus::Watching))
        .await
        .expect("unknown total should not be guessed");
}

#[tokio::test]
async fn settings_batch_rolls_back_and_foreign_keys_are_enforced() {
    let root = tempdir().expect("temporary database root");
    let paths = DatabasePaths::from_root(root.path()).expect("valid paths");
    let database = Database::connect(&paths).await.expect("database");
    let timestamp = timestamp("2026-07-16T08:00:00.000Z");
    let settings = vec![
        SettingRecord {
            key: "theme".to_owned(),
            value: json!("dark"),
            updated_at: timestamp,
        },
        SettingRecord {
            key: String::new(),
            value: json!(true),
            updated_at: timestamp,
        },
    ];
    assert!(database.settings().upsert_many(&settings).await.is_err());
    assert!(
        database
            .settings()
            .get("theme")
            .await
            .expect("setting lookup")
            .is_none()
    );

    let orphan = AiringDataRecord {
        anilist_id: 999,
        media_kind: MediaKind::Anime,
        episode_number: 1,
        airing_at: timestamp,
        fetched_at: timestamp,
    };
    assert!(database.media().upsert_airing(&orphan).await.is_err());
}

#[tokio::test]
async fn metadata_settings_and_cache_records_round_trip() {
    let root = tempdir().expect("temporary database root");
    let paths = DatabasePaths::from_root(root.path()).expect("valid paths");
    let database = Database::connect(&paths).await.expect("database");
    let snapshot = snapshot(Some(12));
    database
        .media()
        .upsert_snapshot(&snapshot)
        .await
        .expect("snapshot upsert");
    assert_eq!(
        database
            .media()
            .get_snapshot(101, MediaKind::Anime)
            .await
            .expect("snapshot lookup"),
        Some(snapshot)
    );

    let airing = AiringDataRecord {
        anilist_id: 101,
        media_kind: MediaKind::Anime,
        episode_number: 13,
        airing_at: timestamp("2026-07-17T08:00:00.000Z"),
        fetched_at: timestamp("2026-07-16T08:00:00.000Z"),
    };
    database
        .media()
        .upsert_airing(&airing)
        .await
        .expect("airing upsert");
    assert_eq!(
        database
            .media()
            .list_airing(101, MediaKind::Anime)
            .await
            .expect("airing lookup"),
        vec![airing]
    );

    let setting = SettingRecord {
        key: "theme".to_owned(),
        value: json!("system"),
        updated_at: timestamp("2026-07-16T08:00:00.000Z"),
    };
    database
        .settings()
        .upsert_many(std::slice::from_ref(&setting))
        .await
        .expect("setting upsert");
    assert_eq!(
        database.settings().get("theme").await.expect("setting"),
        Some(setting)
    );

    let cache = CacheRecord {
        cache_key: "trending:anime:1".to_owned(),
        payload: json!({"ids": [101]}),
        fetched_at: timestamp("2026-07-16T08:00:00.000Z"),
        expires_at: timestamp("2026-07-16T14:00:00.000Z"),
    };
    database.cache().upsert(&cache).await.expect("cache upsert");
    assert_eq!(
        database
            .cache()
            .get("trending:anime:1")
            .await
            .expect("cache lookup"),
        Some(cache)
    );
}

#[tokio::test]
async fn metadata_snapshot_batches_roll_back_on_any_invalid_record() {
    let root = tempdir().expect("temporary database root");
    let paths = DatabasePaths::from_root(root.path()).expect("valid paths");
    let database = Database::connect(&paths).await.expect("database");
    let valid = snapshot(Some(12));
    let mut invalid = snapshot(Some(12));
    invalid.anilist_id = 0;

    assert!(
        database
            .media()
            .upsert_snapshots(&[valid.clone(), invalid])
            .await
            .is_err()
    );
    assert!(
        database
            .media()
            .get_snapshot(valid.anilist_id, valid.media_kind)
            .await
            .expect("snapshot lookup")
            .is_none()
    );
}
