use std::collections::{BTreeMap, VecDeque};
use std::sync::{Arc, Mutex};

use chrono::{DateTime, Duration, Utc};
use serde_json::{Value, json};
use tauri::ipc::{CallbackFn, InvokeBody};
use tauri::test::{
    INVOKE_KEY, MockRuntime, get_ipc_response, mock_builder, mock_context, noop_assets,
};
use tauri::webview::InvokeRequest;
use tauri::{App, WebviewWindow, WebviewWindowBuilder};
use tempfile::tempdir;

use animi_lib::anilist::{
    AniListClient, AniListClock, AniListRequest, AniListResponse, AniListService, AniListTransport,
    NetworkDisabledTransport, TransportError, TransportFuture,
};
use animi_lib::database::{Database, DatabasePaths};
use animi_lib::register_commands;

#[derive(Debug)]
struct FixedClock(DateTime<Utc>);

impl AniListClock for FixedClock {
    fn now(&self) -> DateTime<Utc> {
        self.0
    }
}

#[derive(Default)]
struct FixtureTransport {
    responses: Mutex<VecDeque<Result<AniListResponse, TransportError>>>,
    requests: Mutex<Vec<AniListRequest>>,
}

impl FixtureTransport {
    fn new(responses: impl IntoIterator<Item = Result<AniListResponse, TransportError>>) -> Self {
        Self {
            responses: Mutex::new(responses.into_iter().collect()),
            requests: Mutex::new(Vec::new()),
        }
    }

    fn request_count(&self) -> usize {
        self.requests.lock().expect("request lock").len()
    }
}

impl AniListTransport for FixtureTransport {
    fn send(&self, request: AniListRequest) -> TransportFuture<'_> {
        self.requests.lock().expect("request lock").push(request);
        let response = self
            .responses
            .lock()
            .expect("response lock")
            .pop_front()
            .expect("scripted AniList response");
        Box::pin(async move { response })
    }
}

fn fixture_response(body: &str) -> Result<AniListResponse, TransportError> {
    Ok(AniListResponse {
        status: 200,
        headers: BTreeMap::new(),
        body: body.to_owned(),
    })
}

fn build_test_app(database: Database, client: AniListClient) -> App<MockRuntime> {
    let service = AniListService::new(database.clone(), client);
    register_commands(mock_builder().manage(database).manage(service))
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
            url: if cfg!(any(windows, target_os = "android")) {
                "http://tauri.localhost"
            } else {
                "tauri://localhost"
            }
            .parse()
            .expect("valid mock IPC URL"),
            body: InvokeBody::Json(body),
            headers: Default::default(),
            invoke_key: INVOKE_KEY.to_owned(),
        },
    )
    .map(|response| {
        response
            .deserialize::<Value>()
            .expect("JSON command response")
    })
}

fn invoke_ok(webview: &WebviewWindow<MockRuntime>, command: &str, body: Value) -> Value {
    invoke(webview, command, body)
        .unwrap_or_else(|error| panic!("{command} returned an unexpected error: {error}"))
}

fn invoke_error(webview: &WebviewWindow<MockRuntime>, command: &str, body: Value) -> Value {
    invoke(webview, command, body)
        .expect_err(&format!("{command} should return a structured error"))
}

fn library_request(status: &str, progress: i64, score: f64, notes: &str) -> Value {
    json!({
        "request": {
            "anilistId": 154_587,
            "mediaKind": "ANIME",
            "status": status,
            "progress": progress,
            "score": score,
            "notes": notes
        }
    })
}

#[test]
fn p4_flow_crosses_the_mock_tauri_ipc_boundary_without_live_network() {
    let root = tempdir().expect("temporary database root");
    let paths = DatabasePaths::from_root(root.path()).expect("temporary database paths");
    let database =
        tauri::async_runtime::block_on(Database::connect(&paths)).expect("temporary database");
    let initial_time = Utc::now();
    let transport = Arc::new(FixtureTransport::new([
        fixture_response(include_str!(
            "../tests/fixtures/anilist/search-success.json"
        )),
        fixture_response(include_str!("../tests/fixtures/anilist/details-known.json")),
    ]));
    let client = AniListClient::with_clock(transport.clone(), Arc::new(FixedClock(initial_time)));
    let app = build_test_app(database.clone(), client);
    let webview = main_webview(&app);

    let search = invoke_ok(
        &webview,
        "search_media",
        json!({
            "request": {
                "search": "Frieren",
                "page": 1,
                "perPage": 20
            }
        }),
    );
    assert_eq!(search["items"][0]["anilistId"], 154_587);
    assert!(search["items"][0].get("anilist_id").is_none());
    assert!(search["freshness"].get("fetchedAt").is_some());

    let details = invoke_ok(
        &webview,
        "get_media_details",
        json!({ "request": { "anilistId": 154_587 } }),
    );
    assert_eq!(details["media"]["title"], "Frieren: Beyond Journey's End");
    assert_eq!(details["media"]["episodes"], 28);
    assert_eq!(details["media"]["durationMinutes"], 24);
    assert_eq!(transport.request_count(), 2);

    let baseline = invoke_ok(
        &webview,
        "upsert_library_entry",
        library_request("WATCHING", 3, 7.5, "  baseline note  "),
    );
    assert_eq!(baseline["progress"], 3);
    assert_eq!(baseline["notes"], "baseline note");
    assert!(baseline.get("createdAt").is_some());
    assert!(baseline.get("created_at").is_none());

    let updated = invoke_ok(
        &webview,
        "upsert_library_entry",
        library_request("COMPLETED", 5, 9.0, "  final local note  "),
    );
    assert_eq!(updated["status"], "COMPLETED");
    assert_eq!(updated["progress"], 5);
    assert_eq!(updated["score"], 9.0);
    assert_eq!(updated["notes"], "final local note");

    let dashboard = invoke_ok(&webview, "get_dashboard_insights", json!({}));
    assert_eq!(dashboard["libraryTotals"]["tracked"], 1);
    assert_eq!(dashboard["libraryTotals"]["completed"], 1);
    assert_eq!(dashboard["libraryTotals"]["currentProgress"], 5);
    assert_eq!(dashboard["activity"]["eventCount"], 2);
    assert_eq!(dashboard["activity"]["recordedEpisodes"], 5);
    assert_eq!(dashboard["activity"]["recordedMinutes"], 120);
    assert_eq!(dashboard["activity"]["events"][0]["progressDelta"], 2);
    assert_eq!(
        dashboard["activity"]["events"][0]["episodeDurationMinutes"],
        24
    );

    let invalid = invoke_error(
        &webview,
        "upsert_library_entry",
        library_request("PLANNING", 29, 1.0, "must not persist"),
    );
    assert_eq!(invalid["code"], "PROGRESS_EXCEEDS_TOTAL");
    assert_eq!(invalid["retryable"], false);

    let before_restart = invoke_ok(&webview, "list_library", json!({}));
    assert_eq!(before_restart[0]["entry"]["status"], "COMPLETED");
    assert_eq!(before_restart[0]["entry"]["progress"], 5);
    assert_eq!(before_restart[0]["entry"]["score"], 9.0);
    assert_eq!(before_restart[0]["entry"]["notes"], "final local note");
    assert_eq!(dashboard["activity"]["eventCount"], 2);

    drop(webview);
    drop(app);
    tauri::async_runtime::block_on(database.close());

    let reopened = tauri::async_runtime::block_on(Database::connect(&paths))
        .expect("reopened temporary database");
    let offline_client = AniListClient::with_clock(
        Arc::new(NetworkDisabledTransport),
        Arc::new(FixedClock(initial_time + Duration::hours(25))),
    );
    let reopened_app = build_test_app(reopened.clone(), offline_client);
    let reopened_webview = main_webview(&reopened_app);

    let cached_offline = invoke_ok(
        &reopened_webview,
        "get_media_details",
        json!({ "request": { "anilistId": 154_587 } }),
    );
    assert_eq!(
        cached_offline["media"]["title"],
        "Frieren: Beyond Journey's End"
    );
    assert_eq!(cached_offline["freshness"]["isStale"], true);
    assert_eq!(
        cached_offline["freshness"]["refreshError"]["code"],
        "ANILIST_OFFLINE"
    );

    let persisted = invoke_ok(&reopened_webview, "list_library", json!({}));
    let persisted_entry = persisted[0]["entry"].clone();
    assert_eq!(persisted_entry, before_restart[0]["entry"]);

    let refresh = invoke_ok(&reopened_webview, "refresh_library_metadata", json!({}));
    assert_eq!(refresh["attempted"], 1);
    assert_eq!(refresh["updated"], 1);
    assert_eq!(refresh["stale"], 1);
    assert_eq!(refresh["failures"], json!([]));

    let after_refresh = invoke_ok(&reopened_webview, "list_library", json!({}));
    assert_eq!(after_refresh[0]["entry"], persisted_entry);
    let refreshed_dashboard = invoke_ok(&reopened_webview, "get_dashboard_insights", json!({}));
    assert_eq!(refreshed_dashboard["activity"]["eventCount"], 2);
    assert_eq!(refreshed_dashboard["activity"]["recordedEpisodes"], 5);

    let unconfirmed = invoke_error(
        &reopened_webview,
        "remove_library_entry",
        json!({
            "request": {
                "anilistId": 154_587,
                "mediaKind": "ANIME",
                "confirmed": false
            }
        }),
    );
    assert_eq!(unconfirmed["code"], "CONFIRMATION_REQUIRED");
    assert_eq!(
        invoke_ok(&reopened_webview, "list_library", json!({}))
            .as_array()
            .expect("library array")
            .len(),
        1
    );

    let removed = invoke_ok(
        &reopened_webview,
        "remove_library_entry",
        json!({
            "request": {
                "anilistId": 154_587,
                "mediaKind": "ANIME",
                "confirmed": true
            }
        }),
    );
    assert_eq!(removed["removed"], true);
    assert!(
        invoke_ok(&reopened_webview, "list_library", json!({}))
            .as_array()
            .expect("library array")
            .is_empty()
    );

    let after_removal = invoke_ok(&reopened_webview, "get_dashboard_insights", json!({}));
    assert_eq!(after_removal["libraryTotals"]["tracked"], 0);
    assert_eq!(after_removal["activity"]["eventCount"], 2);
    assert_eq!(after_removal["activity"]["recordedEpisodes"], 5);
    assert_eq!(after_removal["activity"]["events"][0]["anilistId"], 154_587);

    drop(reopened_webview);
    drop(reopened_app);
    tauri::async_runtime::block_on(reopened.close());
}
