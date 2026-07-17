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
    TransportError, TransportFuture,
};
use animi_lib::database::{
    Database, DatabasePaths, MediaSnapshotRecord, MediaTag as StoredMediaTag,
};
use animi_lib::domain::{LibraryStatus, MediaKind, UpsertLibraryEntryRequest};
use animi_lib::register_commands;

#[derive(Debug)]
struct FixedClock(DateTime<Utc>);

impl AniListClock for FixedClock {
    fn now(&self) -> DateTime<Utc> {
        self.0
    }
}

struct FixtureTransport {
    responses: Mutex<VecDeque<Result<AniListResponse, TransportError>>>,
}

impl FixtureTransport {
    fn new(responses: impl IntoIterator<Item = Result<AniListResponse, TransportError>>) -> Self {
        Self {
            responses: Mutex::new(responses.into_iter().collect()),
        }
    }
}

impl AniListTransport for FixtureTransport {
    fn send(&self, _request: AniListRequest) -> TransportFuture<'_> {
        let response = self
            .responses
            .lock()
            .expect("response lock")
            .pop_front()
            .expect("scripted AniList response");
        Box::pin(async move { response })
    }
}

fn fixture_response(body: impl Into<String>) -> Result<AniListResponse, TransportError> {
    Ok(AniListResponse {
        status: 200,
        headers: BTreeMap::new(),
        body: body.into(),
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
            url: "http://tauri.localhost".parse().expect("mock URL"),
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

async fn seed_profile_title(
    database: &Database,
    anilist_id: i64,
    status: LibraryStatus,
    progress: i64,
    total_units: i64,
    signals: (&[&str], &[(&str, i64)]),
    now: DateTime<Utc>,
) {
    let (genres, tags) = signals;
    database
        .media()
        .upsert_snapshot(&MediaSnapshotRecord {
            anilist_id,
            media_kind: MediaKind::Anime,
            title: format!("Tracked {anilist_id}"),
            cover_url: None,
            media_format: Some("TV".to_owned()),
            genres: genres.iter().map(|value| (*value).to_owned()).collect(),
            tags: tags
                .iter()
                .map(|(name, rank)| StoredMediaTag {
                    name: (*name).to_owned(),
                    rank: *rank,
                })
                .collect(),
            media_status: Some("RELEASING".to_owned()),
            total_units: Some(total_units),
            duration_minutes: Some(24),
            cached_at: now,
        })
        .await
        .expect("snapshot");
    database
        .library()
        .upsert(UpsertLibraryEntryRequest {
            anilist_id,
            media_kind: MediaKind::Anime,
            status,
            progress,
            score: None,
            notes: None,
        })
        .await
        .expect("library entry");
}

#[test]
fn p5_home_recommendations_and_activity_cross_the_real_ipc_boundary() {
    let root = tempdir().expect("temporary database root");
    let paths = DatabasePaths::from_root(root.path()).expect("database paths");
    let database =
        tauri::async_runtime::block_on(Database::connect(&paths)).expect("temporary database");
    let now = Utc::now();
    tauri::async_runtime::block_on(seed_profile_title(
        &database,
        101,
        LibraryStatus::Watching,
        1,
        3,
        (&["Drama"], &[("Found Family", 90)]),
        now,
    ));
    tauri::async_runtime::block_on(seed_profile_title(
        &database,
        102,
        LibraryStatus::Completed,
        12,
        12,
        (&["Fantasy"], &[("Found Family", 80)]),
        now,
    ));
    let airing = json!({
        "data": {
            "Page": {
                "pageInfo": { "hasNextPage": false },
                "airingSchedules": [{
                    "episode": 4,
                    "airingAt": (now + Duration::hours(2)).timestamp()
                }]
            }
        }
    });
    let transport = Arc::new(FixtureTransport::new([
        fixture_response(airing.to_string()),
        fixture_response(include_str!(
            "../tests/fixtures/anilist/recommendation-success.json"
        )),
    ]));
    let client = AniListClient::with_clock(transport, Arc::new(FixedClock(now)));
    let app = build_test_app(database.clone(), client);
    let webview = main_webview(&app);

    let home = invoke_ok(&webview, "get_home_airing", json!({}));
    assert_eq!(home["items"].as_array().expect("Home items").len(), 1);
    assert_eq!(home["items"][0]["library"]["entry"]["progress"], 1);
    assert_eq!(home["items"][0]["nextAiring"]["episode"], 4);
    assert_eq!(home["items"][0]["scheduleFreshness"]["isStale"], false);

    for expected in [2, 3] {
        let advanced = invoke_ok(
            &webview,
            "advance_library_progress",
            json!({ "request": { "anilistId": 101 } }),
        );
        assert_eq!(advanced["progress"], expected);
    }
    let bounded = invoke(
        &webview,
        "advance_library_progress",
        json!({ "request": { "anilistId": 101 } }),
    )
    .expect_err("episode total has been reached");
    assert_eq!(bounded["code"], "PROGRESS_NOT_ADVANCEABLE");

    let recommendations = invoke_ok(
        &webview,
        "get_recommendations",
        json!({ "request": { "perPage": 18 } }),
    );
    assert_eq!(recommendations["isFallback"], false);
    assert_eq!(recommendations["eligibleProfileTitles"], 2);
    assert_eq!(recommendations["items"][0]["media"]["anilistId"], 301);
    assert_eq!(
        recommendations["items"][0]["explanationLabels"],
        json!(["Found Family"])
    );

    let dashboard = invoke_ok(&webview, "get_dashboard_insights", json!({}));
    assert_eq!(dashboard["activity"]["recordedEpisodes"], 2);
    assert_eq!(dashboard["activity"]["recordedMinutes"], 48);
    assert_eq!(
        dashboard["activity"]["monthly"].as_array().unwrap().len(),
        12
    );
    assert_eq!(dashboard["activity"]["yearly"][0]["recordedMinutes"], 48);

    drop(webview);
    drop(app);
    tauri::async_runtime::block_on(database.close());
}
