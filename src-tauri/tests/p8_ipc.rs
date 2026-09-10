use std::path::PathBuf;

use serde_json::{Value, json};
use tauri::ipc::{CallbackFn, InvokeBody};
use tauri::test::{
    INVOKE_KEY, MockRuntime, get_ipc_response, mock_builder, mock_context, noop_assets,
};
use tauri::webview::InvokeRequest;
use tauri::{App, WebviewWindow, WebviewWindowBuilder};
use tempfile::tempdir;

use animi_lib::backups::ImportSessionStore;
use animi_lib::commands::backups::{BackupDestinationPicker, BackupSourcePicker};
use animi_lib::database::{Database, DatabasePaths, MediaSnapshotRecord};
use animi_lib::domain::MediaKind;
use animi_lib::register_commands;

fn build_test_app(database: Database) -> App<MockRuntime> {
    register_commands(
        mock_builder()
            .manage(database)
            .manage(BackupDestinationPicker::fixed_path(None))
            .manage(BackupSourcePicker::fixed_path(None))
            .manage(ImportSessionStore::default()),
    )
    .build(mock_context(noop_assets()))
    .expect("mock Tauri application")
}

fn main_webview(app: &App<MockRuntime>) -> WebviewWindow<MockRuntime> {
    WebviewWindowBuilder::new(app, "main", Default::default())
        .build()
        .expect("mock main webview")
}

fn invoke(
    webview: &WebviewWindow<MockRuntime>,
    command: &str,
    body: Value,
) -> Result<Value, Value> {
    get_ipc_response(
        webview,
        InvokeRequest {
            cmd: command.to_owned(),
            callback: CallbackFn(0),
            error: CallbackFn(1),
            url: webview.url().expect("local webview URL"),
            body: InvokeBody::Json(body),
            headers: Default::default(),
            invoke_key: INVOKE_KEY.to_owned(),
        },
    )
    .map(|response| response.deserialize::<Value>().expect("JSON response"))
}

fn invoke_ok(webview: &WebviewWindow<MockRuntime>, command: &str, body: Value) -> Value {
    invoke(webview, command, body)
        .unwrap_or_else(|error| panic!("{command} returned an unexpected error: {error}"))
}

fn manga_request(progress: i64, volume_progress: i64) -> Value {
    json!({
        "request": {
            "anilistId": 202,
            "mediaKind": "MANGA",
            "status": "WATCHING",
            "progress": progress,
            "volumeProgress": volume_progress,
            "score": 8.0,
            "notes": "kind-aware command boundary"
        }
    })
}

#[test]
fn manga_progress_is_transactional_kind_aware_and_durable_across_restart() {
    let root = tempdir().expect("temporary database root");
    let paths = DatabasePaths::from_root(root.path()).expect("database paths");
    let database = tauri::async_runtime::block_on(Database::connect(&paths)).expect("database");
    tauri::async_runtime::block_on(database.media().upsert_snapshot(&MediaSnapshotRecord {
        anilist_id: 202,
        media_kind: MediaKind::Manga,
        title: "A Test Manga".to_owned(),
        cover_url: None,
        media_format: Some("MANGA".to_owned()),
        genres: vec!["Drama".to_owned()],
        tags: Vec::new(),
        media_status: Some("RELEASING".to_owned()),
        total_units: Some(20),
        volume_total: Some(4),
        duration_minutes: None,
        release_date: Some("2026-01".to_owned()),
        cached_at: "2026-08-02T08:00:00Z".parse().expect("timestamp"),
    }))
    .expect("manga snapshot");

    let app = build_test_app(database.clone());
    let webview = main_webview(&app);
    let initial = invoke_ok(&webview, "upsert_library_entry", manga_request(5, 1));
    assert_eq!(initial["volumeProgress"], 1);
    invoke_ok(&webview, "upsert_library_entry", manga_request(3, 0));
    invoke_ok(&webview, "upsert_library_entry", manga_request(5, 1));
    invoke_ok(&webview, "upsert_library_entry", manga_request(6, 2));
    invoke_ok(&webview, "upsert_library_entry", manga_request(6, 3));

    let activity = tauri::async_runtime::block_on(database.activity().list()).expect("activity");
    assert_eq!(activity.len(), 3);
    assert_eq!(
        (activity[0].progress_delta, activity[0].volume_delta),
        (0, 1)
    );
    assert_eq!(
        (activity[1].progress_delta, activity[1].volume_delta),
        (1, 1)
    );
    assert_eq!(
        (activity[2].progress_delta, activity[2].volume_delta),
        (5, 1)
    );
    assert!(
        activity
            .iter()
            .all(|event| event.episode_duration_minutes.is_none())
    );

    let invalid_chapters = invoke(&webview, "upsert_library_entry", manga_request(21, 3))
        .expect_err("known chapter bound");
    assert_eq!(invalid_chapters["code"], "PROGRESS_EXCEEDS_TOTAL");
    let invalid_volumes = invoke(&webview, "upsert_library_entry", manga_request(6, 5))
        .expect_err("known volume bound");
    assert_eq!(invalid_volumes["code"], "VOLUME_PROGRESS_EXCEEDS_TOTAL");
    let invalid_anime = invoke(
        &webview,
        "upsert_library_entry",
        json!({
            "request": {
                "anilistId": 303,
                "mediaKind": "ANIME",
                "status": "WATCHING",
                "progress": 1,
                "volumeProgress": 1,
                "score": null,
                "notes": null
            }
        }),
    )
    .expect_err("anime volume progress must fail");
    assert_eq!(invalid_anime["code"], "INVALID_VOLUME_PROGRESS");

    let dashboard = invoke_ok(
        &webview,
        "get_dashboard_insights",
        json!({ "request": { "mediaKind": "MANGA" } }),
    );
    assert_eq!(dashboard["libraryTotals"]["currentProgress"], 6);
    assert_eq!(dashboard["libraryTotals"]["currentVolumeProgress"], 3);
    assert_eq!(dashboard["activity"]["recordedChapters"], 6);
    assert_eq!(dashboard["activity"]["recordedVolumes"], 3);
    assert_eq!(dashboard["activity"]["recordedMinutes"], 0);

    invoke_ok(
        &webview,
        "upsert_library_entry",
        json!({
            "request": {
                "anilistId": 204,
                "mediaKind": "MANGA",
                "status": "WATCHING",
                "progress": 2,
                "volumeProgress": 1,
                "score": null,
                "notes": null
            }
        }),
    );
    let listed = invoke_ok(&webview, "list_library", json!({}));
    let missing_metadata = listed
        .as_array()
        .expect("library array")
        .iter()
        .find(|item| item["entry"]["anilistId"] == 204)
        .expect("local manga without metadata");
    assert!(missing_metadata["media"].is_null());
    invoke_ok(
        &webview,
        "remove_library_entry",
        json!({
            "request": {
                "anilistId": 204,
                "mediaKind": "MANGA",
                "confirmed": true
            }
        }),
    );
    assert_eq!(
        tauri::async_runtime::block_on(database.activity().list())
            .expect("retained activity")
            .len(),
        4
    );

    drop(webview);
    drop(app);
    tauri::async_runtime::block_on(database.close());
    let reopened = tauri::async_runtime::block_on(Database::connect(&paths)).expect("reopened");
    let entries = tauri::async_runtime::block_on(reopened.library().list()).expect("library");
    assert_eq!(entries[0].media_kind, MediaKind::Manga);
    assert_eq!(
        (entries[0].progress, entries[0].volume_progress),
        (6, Some(3))
    );
    assert_eq!(
        tauri::async_runtime::block_on(reopened.activity().list())
            .unwrap()
            .len(),
        4
    );
}

#[tokio::test]
async fn v1001_fixture_upgrades_anime_and_preexisting_manga_without_identity_loss() {
    let root = tempdir().expect("temporary database root");
    let paths = DatabasePaths::from_root(root.path()).expect("database paths");
    let fixture =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/database/v0007.sqlite3");
    std::fs::copy(fixture, paths.database()).expect("copied immutable fixture");

    let database = Database::connect(&paths).await.expect("upgraded database");
    let entries = database.library().list().await.expect("library");
    assert_eq!(entries.len(), 2);
    let anime = entries
        .iter()
        .find(|entry| entry.media_kind == MediaKind::Anime)
        .expect("anime entry");
    assert_eq!((anime.progress, anime.volume_progress), (3, None));
    let manga = entries
        .iter()
        .find(|entry| entry.media_kind == MediaKind::Manga)
        .expect("manga entry");
    assert_eq!((manga.progress, manga.volume_progress), (7, Some(0)));

    let activity = database.activity().list().await.expect("activity");
    let manga_event = activity
        .iter()
        .find(|event| event.media_kind == MediaKind::Manga)
        .expect("preserved manga event");
    assert_eq!(manga_event.event_id, "20000000-0000-4000-a000-000000000202");
    assert_eq!(
        (manga_event.progress_delta, manga_event.volume_delta),
        (9, 0)
    );
    assert!(manga_event.episode_duration_minutes.is_none());
    assert_eq!(manga_event.occurred_at, "2026-07-16T10:00:00.000Z");

    database.close().await;
    Database::connect(&paths)
        .await
        .expect("upgraded fixture reopens cleanly");
}
