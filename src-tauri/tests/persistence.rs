use animi_lib::database::{
    AiringDataRecord, CacheRecord, Database, DatabasePaths, MediaSnapshotRecord, MediaTag,
    SettingRecord,
};
use animi_lib::domain::{LibraryStatus, MediaKind, UpsertLibraryEntryRequest};
use chrono::{DateTime, Utc};
use serde_json::json;
use sqlx::sqlite::SqliteConnectOptions;
use sqlx::{Connection, SqliteConnection};
use tempfile::tempdir;

const MIGRATION_0001_CHECKSUM: &str = "4f33f13a114dc098bc6ab75229eb86c43c4f35eb98c8280f36215860ebfd679deb83643d3a731391ddd5e67aac85cc04";
const MIGRATION_0002_CHECKSUM: &str = "9361fde43170e805d0688fc5db6e64a11ac3e7f5a1a54d3b881f2a218f122e8e77c43993ea186a1968fb733f16b1fe27";
const MIGRATION_0003_CHECKSUM: &str = "614bc98e326578e7a1649dae266c728ee5ec5468ec7a0c72291b998c3c7b7297620ef2f567b803105036135f6126b318";
const MIGRATION_0004_CHECKSUM: &str = "2d33bf79c69c9975e0f2e0341bd55b7c8117d495db9e4818dcf0f678aec199f9aeabf55df24da15138c670442231563e";

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
        duration_minutes: Some(24),
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

#[tokio::test]
async fn v0001_fixture_upgrades_without_losing_authoritative_records() {
    let root = tempdir().expect("temporary database root");
    let paths = DatabasePaths::from_root(root.path()).expect("valid paths");
    let fixture = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/database/v0001.sqlite3");
    std::fs::copy(&fixture, paths.database()).expect("copy immutable fixture");

    let database = Database::connect(&paths).await.expect("upgraded database");
    let items = database
        .library()
        .list_items()
        .await
        .expect("upgraded library");

    assert_eq!(items.len(), 1);
    assert_eq!(items[0].entry.anilist_id, 101);
    assert_eq!(items[0].entry.progress, 3);
    assert_eq!(
        items[0].media.as_ref().expect("preserved snapshot").title,
        "Fixture Anime"
    );
    assert_eq!(
        items[0]
            .media
            .as_ref()
            .expect("preserved snapshot")
            .duration_minutes,
        None
    );
    assert_eq!(
        database
            .settings()
            .get("theme")
            .await
            .expect("setting lookup")
            .expect("preserved theme")
            .value,
        json!("SYSTEM")
    );
    assert_eq!(
        database
            .media()
            .list_airing(101, MediaKind::Anime)
            .await
            .expect("preserved airing")
            .len(),
        1
    );
    let legacy_cache = database
        .cache()
        .get("details:anime:101")
        .await
        .expect("cache lookup")
        .expect("preserved legacy cache");
    assert_eq!(legacy_cache.payload["title"], "Fixture Anime");
    assert!(legacy_cache.payload.get("durationMinutes").is_none());
    assert!(
        database
            .activity()
            .list()
            .await
            .expect("activity")
            .is_empty()
    );

    database
        .library()
        .upsert(library_request(4, LibraryStatus::Watching))
        .await
        .expect("post-upgrade progress");
    let activity = database.activity().list().await.expect("new activity");
    assert_eq!(activity.len(), 1);
    assert_eq!(activity[0].progress_delta, 1);
    assert_eq!(activity[0].episode_duration_minutes, None);

    database.close().await;
    let reopened = Database::connect(&paths).await.expect("reopened upgrade");
    assert_eq!(
        reopened
            .activity()
            .list()
            .await
            .expect("persisted activity"),
        activity
    );
    assert_eq!(
        reopened
            .library()
            .list_items()
            .await
            .expect("persisted library")[0]
            .entry
            .progress,
        4
    );
}

#[tokio::test]
async fn v0002_fixture_preserves_activity_and_applies_activity_progress_migration() {
    let root = tempdir().expect("temporary database root");
    let paths = DatabasePaths::from_root(root.path()).expect("valid paths");
    let fixture = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/database/v0002.sqlite3");
    std::fs::copy(&fixture, paths.database()).expect("copy immutable fixture");

    let database = Database::connect(&paths).await.expect("upgraded database");
    let items = database
        .library()
        .list_items()
        .await
        .expect("upgraded library");
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].entry.anilist_id, 101);
    assert_eq!(items[0].entry.progress, 3);
    assert_eq!(
        items[0]
            .media
            .as_ref()
            .expect("preserved snapshot")
            .duration_minutes,
        Some(24)
    );
    assert_eq!(
        database
            .settings()
            .get("theme")
            .await
            .expect("setting lookup")
            .expect("preserved theme")
            .value,
        json!("SYSTEM")
    );
    assert_eq!(
        database
            .media()
            .list_airing(101, MediaKind::Anime)
            .await
            .expect("preserved airing")
            .len(),
        1
    );
    assert!(
        database
            .cache()
            .get("details:anime:101")
            .await
            .expect("cache lookup")
            .is_some()
    );

    let original_activity = database
        .activity()
        .list()
        .await
        .expect("preserved activity");
    assert_eq!(original_activity.len(), 1);
    assert_eq!(original_activity[0].progress_delta, 2);
    assert_eq!(original_activity[0].episode_duration_minutes, Some(24));
    assert_eq!(original_activity[0].occurred_at, "2026-07-16T08:30:00.000Z");

    let options = SqliteConnectOptions::new()
        .filename(paths.database())
        .read_only(true);
    let mut inspection = SqliteConnection::connect_with(&options)
        .await
        .expect("read-only migration inspection");
    let activity_progress = sqlx::query_scalar::<_, i64>(
        "SELECT activity_progress FROM library_entries \
         WHERE anilist_id = 101 AND media_kind = 'ANIME'",
    )
    .fetch_one(&mut inspection)
    .await
    .expect("activity progress baseline");
    assert_eq!(activity_progress, 3);

    let migrations = sqlx::query_as::<_, (i64, String)>(
        "SELECT version, lower(hex(checksum)) FROM _sqlx_migrations ORDER BY version",
    )
    .fetch_all(&mut inspection)
    .await
    .expect("applied migration checksums");
    assert_eq!(
        migrations,
        vec![
            (1, MIGRATION_0001_CHECKSUM.to_owned()),
            (2, MIGRATION_0002_CHECKSUM.to_owned()),
            (3, MIGRATION_0003_CHECKSUM.to_owned()),
            (4, MIGRATION_0004_CHECKSUM.to_owned()),
        ]
    );
    inspection
        .close()
        .await
        .expect("close migration inspection");

    database
        .library()
        .upsert(library_request(4, LibraryStatus::Watching))
        .await
        .expect("post-upgrade progress");
    let activity = database.activity().list().await.expect("updated activity");
    assert_eq!(activity.len(), 2);
    assert_eq!(activity[0].progress_delta, 1);
    assert_eq!(activity[0].episode_duration_minutes, Some(24));
    assert_eq!(activity[1], original_activity[0]);

    database.close().await;
    let reopened = Database::connect(&paths).await.expect("reopened upgrade");
    assert_eq!(
        reopened
            .library()
            .list_items()
            .await
            .expect("persisted library")[0]
            .entry
            .progress,
        4
    );
    assert_eq!(
        reopened
            .activity()
            .list()
            .await
            .expect("persisted activity"),
        activity
    );
}

#[tokio::test]
async fn v0003_fixture_adds_stable_activity_identity_without_changing_the_event() {
    let root = tempdir().expect("temporary database root");
    let paths = DatabasePaths::from_root(root.path()).expect("valid paths");
    let fixture = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/database/v0003.sqlite3");
    std::fs::copy(&fixture, paths.database()).expect("copy immutable fixture");

    let database = Database::connect(&paths).await.expect("upgraded database");
    let original_activity = database
        .activity()
        .list()
        .await
        .expect("preserved activity");
    assert_eq!(original_activity.len(), 1);
    assert_eq!(original_activity[0].anilist_id, 101);
    assert_eq!(original_activity[0].progress_delta, 2);
    assert_eq!(original_activity[0].episode_duration_minutes, Some(24));
    assert_eq!(original_activity[0].occurred_at, "2026-07-16T08:30:00.000Z");

    let options = SqliteConnectOptions::new()
        .filename(paths.database())
        .read_only(true);
    let mut inspection = SqliteConnection::connect_with(&options)
        .await
        .expect("read-only migration inspection");
    let (local_id, event_id) = sqlx::query_as::<_, (i64, String)>(
        "SELECT id, event_id FROM watch_activity WHERE anilist_id = 101",
    )
    .fetch_one(&mut inspection)
    .await
    .expect("migrated event identity");
    assert_eq!(local_id, 1);
    uuid::Uuid::parse_str(&event_id).expect("portable UUID event identity");
    assert_eq!(&event_id[14..15], "4");
    assert!(matches!(&event_id[19..20], "8" | "9" | "a" | "b"));

    let migrations = sqlx::query_as::<_, (i64, String)>(
        "SELECT version, lower(hex(checksum)) FROM _sqlx_migrations ORDER BY version",
    )
    .fetch_all(&mut inspection)
    .await
    .expect("applied migration checksums");
    assert_eq!(
        migrations,
        vec![
            (1, MIGRATION_0001_CHECKSUM.to_owned()),
            (2, MIGRATION_0002_CHECKSUM.to_owned()),
            (3, MIGRATION_0003_CHECKSUM.to_owned()),
            (4, MIGRATION_0004_CHECKSUM.to_owned()),
        ]
    );
    inspection
        .close()
        .await
        .expect("close migration inspection");

    database
        .library()
        .upsert(library_request(4, LibraryStatus::Watching))
        .await
        .expect("post-upgrade progress");
    let updated_activity = database.activity().list().await.expect("updated activity");
    assert_eq!(updated_activity.len(), 2);
    assert_eq!(updated_activity[0].progress_delta, 1);
    assert_eq!(updated_activity[1], original_activity[0]);

    let options = SqliteConnectOptions::new()
        .filename(paths.database())
        .read_only(true);
    let mut inspection = SqliteConnection::connect_with(&options)
        .await
        .expect("post-insert migration inspection");
    let event_rows = sqlx::query_as::<_, (i64, String)>(
        "SELECT id, event_id FROM watch_activity ORDER BY id ASC",
    )
    .fetch_all(&mut inspection)
    .await
    .expect("event identities after insert");
    assert_eq!(event_rows.len(), 2);
    assert_eq!(event_rows[0], (1, event_id.clone()));
    assert_eq!(event_rows[1].0, 2);
    assert_ne!(event_rows[1].1, event_id);
    uuid::Uuid::parse_str(&event_rows[1].1).expect("new UUID event identity");
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT seq FROM sqlite_sequence WHERE name = 'watch_activity'"
        )
        .fetch_one(&mut inspection)
        .await
        .expect("watch activity autoincrement sequence"),
        2
    );
    inspection
        .close()
        .await
        .expect("close post-insert inspection");

    database.close().await;
    let reopened = Database::connect(&paths).await.expect("reopened upgrade");
    assert_eq!(
        reopened.activity().list().await.expect("reopened activity"),
        updated_activity
    );
    let options = SqliteConnectOptions::new()
        .filename(paths.database())
        .read_only(true);
    let mut inspection = SqliteConnection::connect_with(&options)
        .await
        .expect("reopened inspection");
    assert_eq!(
        sqlx::query_scalar::<_, String>("SELECT event_id FROM watch_activity WHERE id = 1")
            .fetch_one(&mut inspection)
            .await
            .expect("stable event identity"),
        event_rows[0].1
    );
    assert_eq!(
        sqlx::query_scalar::<_, String>("SELECT event_id FROM watch_activity WHERE id = 2")
            .fetch_one(&mut inspection)
            .await
            .expect("stable new event identity"),
        event_rows[1].1
    );
}
