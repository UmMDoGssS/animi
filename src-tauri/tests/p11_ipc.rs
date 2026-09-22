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
fn reading_feed_and_quick_progress_are_local_bounded_and_persistent() {
    let root = tempdir().expect("isolated profile");
    let paths = DatabasePaths::from_root(root.path()).expect("paths");
    let database = tauri::async_runtime::block_on(Database::connect(&paths)).expect("database");
    let app = build_test_app(database.clone());
    let webview = main_webview(&app);
    assert_eq!(
        invoke_ok(&webview, "get_home_reading", json!({})),
        json!([])
    );
    invoke_ok(&webview, "upsert_library_entry", manga_request(5, 2));
    let mut other = manga_request(0, 0);
    other["request"]["anilistId"] = json!(303);
    other["request"]["status"] = json!("PLANNING");
    invoke_ok(&webview, "upsert_library_entry", other.clone());
    let feed = invoke_ok(&webview, "get_home_reading", json!({}));
    assert_eq!(feed.as_array().expect("array").len(), 1);
    assert!(feed[0]["media"].is_null());
    let quick = json!({"request": {"anilistId":202,"mediaKind":"MANGA"}});
    let advanced = invoke_ok(&webview, "advance_library_progress", quick.clone());
    assert_eq!(advanced["progress"], 6);
    assert_eq!(advanced["volumeProgress"], 2);
    other["request"]["status"] = json!("WATCHING");
    invoke_ok(&webview, "upsert_library_entry", other);
    let feed = invoke_ok(&webview, "get_home_reading", json!({}));
    assert!(feed[0]["entry"]["updatedAt"].as_str() >= feed[1]["entry"]["updatedAt"].as_str());
    tauri::async_runtime::block_on(database.media().upsert_snapshot(&MediaSnapshotRecord {
        anilist_id: 202,
        media_kind: MediaKind::Manga,
        title: "Known total".into(),
        cover_url: None,
        media_format: Some("MANGA".into()),
        genres: vec![],
        tags: vec![],
        media_status: None,
        total_units: Some(6),
        volume_total: Some(3),
        duration_minutes: None,
        release_date: None,
        cached_at: "2000-01-01T00:00:00Z".parse().expect("time"),
    }))
    .expect("cached metadata");
    assert_eq!(
        invoke(&webview, "advance_library_progress", quick).expect_err("bound")["code"],
        "PROGRESS_NOT_ADVANCEABLE"
    );
    let feed = invoke_ok(&webview, "get_home_reading", json!({}));
    assert!(
        feed.as_array()
            .expect("array")
            .iter()
            .any(|item| item["media"]["isStale"] == true)
    );
    invoke_ok(&webview, "upsert_library_entry", manga_request(4, 1));
    invoke_ok(
        &webview,
        "advance_library_progress",
        json!({"request":{"anilistId":202,"mediaKind":"MANGA"}}),
    );
    let insights = invoke_ok(
        &webview,
        "get_dashboard_insights",
        json!({"request":{"mediaKind":"MANGA"}}),
    );
    assert_eq!(insights["activity"]["recordedChapters"], 6);
    assert_eq!(insights["activity"]["recordedVolumes"], 2);
    assert_eq!(insights["activity"]["recordedEpisodes"], 0);
    assert_eq!(insights["activity"]["recordedMinutes"], 0);
    assert_eq!(
        insights["activity"]["monthly"]
            .as_array()
            .expect("months")
            .last()
            .expect("month")["recordedChapters"],
        6
    );
    assert_eq!(
        insights["activity"]["yearly"]
            .as_array()
            .expect("years")
            .last()
            .expect("year")["recordedVolumes"],
        2
    );
    drop(webview);
    drop(app);
    tauri::async_runtime::block_on(database.close());
    let reopened = tauri::async_runtime::block_on(Database::connect(&paths)).expect("reopen");
    let app = build_test_app(reopened);
    let webview = main_webview(&app);
    assert_eq!(
        invoke_ok(
            &webview,
            "get_dashboard_insights",
            json!({"request":{"mediaKind":"MANGA"}})
        ),
        insights
    );
    let mut limit = manga_request(i64::from(i32::MAX), 0);
    limit["request"]["anilistId"] = json!(404);
    invoke_ok(&webview, "upsert_library_entry", limit);
    assert_eq!(
        invoke(
            &webview,
            "advance_library_progress",
            json!({"request":{"anilistId":404,"mediaKind":"MANGA"}})
        )
        .expect_err("limit")["code"],
        "INVALID_PROGRESS"
    );
}
