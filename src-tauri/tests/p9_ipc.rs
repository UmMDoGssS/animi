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
use animi_lib::database::{Database, DatabasePaths, MediaSnapshotRecord, MediaTag};
use animi_lib::domain::{LibraryStatus, MediaKind, UpsertLibraryEntryRequest};
use animi_lib::register_commands;

struct TestClock {
    now: Mutex<DateTime<Utc>>,
}

impl TestClock {
    fn new() -> Self {
        Self {
            now: Mutex::new("2026-08-09T08:00:00Z".parse().expect("timestamp")),
        }
    }

    fn advance(&self, duration: Duration) {
        *self.now.lock().expect("clock lock") += duration;
    }
}

impl AniListClock for TestClock {
    fn now(&self) -> DateTime<Utc> {
        *self.now.lock().expect("clock lock")
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

    fn requests(&self) -> Vec<AniListRequest> {
        self.requests.lock().expect("request lock").clone()
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
            .expect("scripted response");
        Box::pin(async move { response })
    }
}

fn response(body: &str) -> Result<AniListResponse, TransportError> {
    status_response(200, body, BTreeMap::new())
}

fn status_response(
    status: u16,
    body: &str,
    headers: BTreeMap<String, String>,
) -> Result<AniListResponse, TransportError> {
    Ok(AniListResponse {
        status,
        headers,
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

fn media_page_request(media_kind: &str, page: u16) -> Value {
    json!({
        "request": {
            "mediaKind": media_kind,
            "page": page,
            "perPage": 20
        }
    })
}

#[test]
fn manga_discovery_details_and_cache_isolation_cross_the_ipc_boundary() {
    let root = tempdir().expect("temporary root");
    let database = tauri::async_runtime::block_on(Database::connect(
        &DatabasePaths::from_root(root.path()).expect("database paths"),
    ))
    .expect("database");
    let transport = Arc::new(FixtureTransport::new([
        response(include_str!("fixtures/anilist/manga-search-success.json")),
        response(include_str!("fixtures/anilist/search-success.json")),
        response(include_str!("fixtures/anilist/manga-details-novel.json")),
        response(include_str!("fixtures/anilist/manga-one-shot-success.json")),
        response(include_str!("fixtures/anilist/manga-partial-errors.json")),
    ]));
    let client = AniListClient::with_clock(transport.clone(), Arc::new(TestClock::new()));
    let app = build_test_app(database, client);
    let webview = main_webview(&app);
    let search_request = |media_kind: &str| {
        json!({
            "request": {
                "mediaKind": media_kind,
                "search": "cats",
                "page": 1,
                "perPage": 20
            }
        })
    };

    let manga = invoke_ok(&webview, "search_media", search_request("MANGA"));
    let anime = invoke_ok(&webview, "search_media", search_request("ANIME"));
    assert_eq!(manga["items"][0]["mediaKind"], "MANGA");
    assert_eq!(manga["items"][0]["chapters"], 120);
    assert_eq!(manga["items"][0]["episodes"], Value::Null);
    assert_eq!(anime["items"][0]["mediaKind"], "ANIME");

    let details = invoke_ok(
        &webview,
        "get_media_details",
        json!({ "request": { "mediaKind": "MANGA", "anilistId": 170_643 } }),
    );
    assert_eq!(details["media"]["mediaFormat"], "NOVEL");
    assert_eq!(details["media"]["releaseDate"], "2021-11");
    assert_eq!(details["media"]["sequels"].as_array().unwrap().len(), 1);

    let one_shot = invoke_ok(&webview, "list_trending", media_page_request("MANGA", 1));
    let partial = invoke_ok(&webview, "list_trending", media_page_request("MANGA", 2));
    assert_eq!(one_shot["items"][0]["mediaFormat"], "ONE_SHOT");
    assert_eq!(partial["items"][0]["title"], "Usable Partial Manga");

    invoke_ok(&webview, "search_media", search_request("MANGA"));
    invoke_ok(&webview, "search_media", search_request("ANIME"));
    let requests = transport.requests();
    assert_eq!(
        requests.len(),
        5,
        "both repeat searches must hit their own caches"
    );
    assert_eq!(requests[0].variables["mediaType"], "MANGA");
    assert_eq!(requests[1].variables["mediaType"], "ANIME");
    assert!(requests[0].query.contains("isAdult: false"));
}

#[test]
fn manga_details_use_stale_offline_data_and_keep_structured_failures() {
    let root = tempdir().expect("temporary root");
    let database = tauri::async_runtime::block_on(Database::connect(
        &DatabasePaths::from_root(root.path()).expect("database paths"),
    ))
    .expect("database");
    let mut retry_headers = BTreeMap::new();
    retry_headers.insert("Retry-After".to_owned(), "30".to_owned());
    let transport = Arc::new(FixtureTransport::new([
        response(include_str!("fixtures/anilist/manga-details-novel.json")),
        Err(TransportError::Unavailable("fixture offline".to_owned())),
        Err(TransportError::Unavailable("fixture offline".to_owned())),
        status_response(
            403,
            include_str!("fixtures/anilist/forbidden.json"),
            BTreeMap::new(),
        ),
        status_response(
            429,
            include_str!("fixtures/anilist/rate-limited.json"),
            retry_headers,
        ),
    ]));
    let clock = Arc::new(TestClock::new());
    let client = AniListClient::with_clock(transport.clone(), clock.clone());
    let app = build_test_app(database, client);
    let webview = main_webview(&app);
    let details_request =
        |anilist_id: i64| json!({ "request": { "mediaKind": "MANGA", "anilistId": anilist_id } });

    invoke_ok(&webview, "get_media_details", details_request(170_643));
    clock.advance(Duration::hours(25));
    let stale = invoke_ok(&webview, "get_media_details", details_request(170_643));
    assert_eq!(stale["freshness"]["isStale"], true);
    assert_eq!(
        stale["freshness"]["refreshError"]["code"],
        "ANILIST_OFFLINE"
    );

    let offline = invoke(&webview, "get_media_details", details_request(999_001)).unwrap_err();
    assert_eq!(offline["code"], "ANILIST_OFFLINE");
    let forbidden = invoke(&webview, "get_media_details", details_request(999_002)).unwrap_err();
    assert_eq!(forbidden["code"], "ANILIST_FORBIDDEN");
    let limited = invoke(&webview, "get_media_details", details_request(999_003)).unwrap_err();
    assert_eq!(limited["code"], "ANILIST_RATE_LIMITED");
    assert!(limited["message"].as_str().unwrap().contains("30 seconds"));
    assert_eq!(transport.requests().len(), 5);
}

async fn seed_title(
    database: &Database,
    anilist_id: i64,
    media_kind: MediaKind,
    status: LibraryStatus,
    title: &str,
    genres: &[&str],
    tags: &[(&str, i64)],
) {
    database
        .media()
        .upsert_snapshot(&MediaSnapshotRecord {
            anilist_id,
            media_kind,
            title: title.to_owned(),
            cover_url: None,
            media_format: Some(
                match media_kind {
                    MediaKind::Anime => "TV",
                    MediaKind::Manga => "MANGA",
                }
                .to_owned(),
            ),
            genres: genres.iter().map(|value| (*value).to_owned()).collect(),
            tags: tags
                .iter()
                .map(|(name, rank)| MediaTag {
                    name: (*name).to_owned(),
                    rank: *rank,
                })
                .collect(),
            media_status: Some("RELEASING".to_owned()),
            total_units: Some(30),
            volume_total: (media_kind == MediaKind::Manga).then_some(4),
            duration_minutes: (media_kind == MediaKind::Anime).then_some(24),
            release_date: None,
            cached_at: "2026-08-01T08:00:00Z".parse().expect("timestamp"),
        })
        .await
        .expect("snapshot");
    database
        .library()
        .upsert(UpsertLibraryEntryRequest {
            anilist_id,
            media_kind,
            status,
            progress: 1,
            volume_progress: (media_kind == MediaKind::Manga).then_some(0),
            score: Some(8.0),
            notes: Some(format!("local note for {title}")),
        })
        .await
        .expect("library entry");
}

#[test]
fn manga_refresh_and_recommendations_preserve_kind_and_local_authority() {
    let root = tempdir().expect("temporary root");
    let database = tauri::async_runtime::block_on(Database::connect(
        &DatabasePaths::from_root(root.path()).expect("database paths"),
    ))
    .expect("database");
    tauri::async_runtime::block_on(async {
        seed_title(
            &database,
            30013,
            MediaKind::Manga,
            LibraryStatus::Watching,
            "Local Yotsuba",
            &["Comedy"],
            &[("Family Life", 90)],
        )
        .await;
        seed_title(
            &database,
            154_587,
            MediaKind::Anime,
            LibraryStatus::Watching,
            "Local Anime",
            &["Drama"],
            &[("Time", 80)],
        )
        .await;
    });
    let before =
        tauri::async_runtime::block_on(database.library().list()).expect("library before refresh");
    let transport = Arc::new(FixtureTransport::new([
        response(include_str!("fixtures/anilist/manga-search-success.json")),
        Err(TransportError::Unavailable("fixture offline".to_owned())),
    ]));
    let client = AniListClient::with_clock(transport.clone(), Arc::new(TestClock::new()));
    let app = build_test_app(database.clone(), client);
    let webview = main_webview(&app);

    let refreshed = invoke_ok(
        &webview,
        "refresh_library_metadata",
        json!({ "request": { "mediaKind": "MANGA" } }),
    );
    assert_eq!(
        (
            refreshed["attempted"].as_u64(),
            refreshed["updated"].as_u64()
        ),
        (Some(1), Some(1))
    );
    let after =
        tauri::async_runtime::block_on(database.library().list()).expect("library after refresh");
    assert_eq!(
        before, after,
        "metadata refresh must not change user-owned fields"
    );
    let manga_snapshot =
        tauri::async_runtime::block_on(database.media().get_snapshot(30013, MediaKind::Manga))
            .expect("snapshot lookup")
            .expect("manga snapshot");
    assert_eq!(manga_snapshot.title, "Yotsuba&!");
    assert_eq!(
        (manga_snapshot.total_units, manga_snapshot.volume_total),
        (Some(120), Some(16))
    );

    let stale = invoke_ok(
        &webview,
        "refresh_library_metadata",
        json!({ "request": { "mediaKind": "MANGA" } }),
    );
    assert_eq!(stale["updated"], 0);
    assert_eq!(stale["stale"], 1);
    assert_eq!(stale["failures"][0]["error"]["code"], "ANILIST_OFFLINE");
    assert!(
        transport
            .requests()
            .iter()
            .all(|request| request.variables["mediaType"] == "MANGA")
    );
}

#[test]
fn manga_recommendations_use_only_manga_profile_and_candidate_identity() {
    let root = tempdir().expect("temporary root");
    let database = tauri::async_runtime::block_on(Database::connect(
        &DatabasePaths::from_root(root.path()).expect("database paths"),
    ))
    .expect("database");
    tauri::async_runtime::block_on(async {
        seed_title(
            &database,
            9001,
            MediaKind::Manga,
            LibraryStatus::Watching,
            "Reading Manga",
            &["Fantasy"],
            &[("Cats", 90)],
        )
        .await;
        seed_title(
            &database,
            9002,
            MediaKind::Manga,
            LibraryStatus::Completed,
            "Completed Manga",
            &["Fantasy"],
            &[("Cats", 80)],
        )
        .await;
        seed_title(
            &database,
            9003,
            MediaKind::Anime,
            LibraryStatus::Watching,
            "Tracked Anime With Shared ID",
            &["Sports"],
            &[("Sports", 100)],
        )
        .await;
    });
    let transport = Arc::new(FixtureTransport::new([
        response(include_str!(
            "fixtures/anilist/manga-recommendation-success.json"
        )),
        Err(TransportError::Unavailable("fixture offline".to_owned())),
    ]));
    let clock = Arc::new(TestClock::new());
    let client = AniListClient::with_clock(transport.clone(), clock.clone());
    let app = build_test_app(database, client);
    let webview = main_webview(&app);
    let request = json!({ "request": { "mediaKind": "MANGA", "perPage": 18 } });

    let first = invoke_ok(&webview, "get_recommendations", request.clone());
    assert_eq!(first["eligibleProfileTitles"], 2);
    assert_eq!(first["isFallback"], false);
    assert_eq!(first["items"][0]["media"]["anilistId"], 9100);
    let ids = first["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|item| item["media"]["anilistId"].as_i64().unwrap())
        .collect::<Vec<_>>();
    assert!(!ids.contains(&9001), "tracked manga must be excluded");
    assert!(
        ids.contains(&9003),
        "an anime entry must not hide same-ID manga"
    );
    assert!(
        first["items"]
            .as_array()
            .unwrap()
            .iter()
            .all(|item| item["media"]["mediaKind"] == "MANGA")
    );

    clock.advance(Duration::hours(7));
    let stale = invoke_ok(&webview, "get_recommendations", request);
    assert_eq!(stale["freshness"]["isStale"], true);
    assert_eq!(stale["items"], first["items"]);
    assert_eq!(transport.requests().len(), 2);
}

#[test]
fn p10_manga_library_flow_survives_restart_offline_and_confirmed_removal() {
    let root = tempdir().expect("temporary profile");
    let paths = DatabasePaths::from_root(root.path()).expect("paths");
    let database = tauri::async_runtime::block_on(Database::connect(&paths)).expect("database");
    let transport = Arc::new(FixtureTransport::new([
        response(include_str!("fixtures/anilist/manga-search-success.json")),
        response(include_str!("fixtures/anilist/manga-details-novel.json")),
    ]));
    let clock = Arc::new(TestClock::new());
    let app = build_test_app(
        database.clone(),
        AniListClient::with_clock(transport, clock.clone()),
    );
    let webview = main_webview(&app);
    let anime = json!({"request": {"anilistId": 101, "mediaKind": "ANIME", "status": "WATCHING", "progress": 3, "volumeProgress": null, "score": 8, "notes": "Preserve anime"}});
    let original_anime = invoke_ok(&webview, "upsert_library_entry", anime);
    invoke_ok(
        &webview,
        "search_media",
        json!({"request": {"mediaKind": "MANGA", "search": "cat", "page": 1, "perPage": 20}}),
    );
    let details_request = json!({"request": {"mediaKind": "MANGA", "anilistId": 170643}});
    let details = invoke_ok(&webview, "get_media_details", details_request.clone());
    assert_eq!(details["media"]["mediaFormat"], "NOVEL");
    let manga = |chapters, volumes| json!({"request": {"anilistId": 170643, "mediaKind": "MANGA", "status": "WATCHING", "progress": chapters, "volumeProgress": volumes, "score": 9, "notes": "Independent units"}});
    invoke_ok(&webview, "upsert_library_entry", manga(12, 2));
    invoke_ok(&webview, "upsert_library_entry", manga(8, 1));
    invoke_ok(&webview, "upsert_library_entry", manga(12, 2));
    let insights = invoke_ok(
        &webview,
        "get_dashboard_insights",
        json!({"request": {"mediaKind": "MANGA"}}),
    );
    assert_eq!(insights["libraryTotals"]["currentProgress"], 12);
    assert_eq!(insights["libraryTotals"]["currentVolumeProgress"], 2);
    assert_eq!(insights["activity"]["recordedChapters"], 12);
    assert_eq!(insights["activity"]["recordedVolumes"], 2);
    assert_eq!(insights["activity"]["recordedEpisodes"], 0);
    assert_eq!(insights["activity"]["eventCount"], 1);
    drop(webview);
    drop(app);
    tauri::async_runtime::block_on(database.close());
    let reopened =
        tauri::async_runtime::block_on(Database::connect(&paths)).expect("reopened database");
    clock.advance(Duration::days(2));
    let offline = Arc::new(FixtureTransport::new([
        Err(TransportError::Unavailable(
            "isolated offline test".to_owned(),
        )),
        Err(TransportError::Unavailable(
            "isolated offline test".to_owned(),
        )),
    ]));
    let app = build_test_app(reopened, AniListClient::with_clock(offline, clock));
    let webview = main_webview(&app);
    let stale = invoke_ok(&webview, "get_media_details", details_request);
    assert_eq!(stale["freshness"]["isStale"], true);
    assert_eq!(stale["media"]["chapters"], 18);
    let refreshed = invoke_ok(
        &webview,
        "refresh_library_metadata",
        json!({"request": {"mediaKind": "MANGA"}}),
    );
    assert_eq!(refreshed["stale"], 1);
    let removal = |confirmed| json!({"request": {"anilistId": 170643, "mediaKind": "MANGA", "confirmed": confirmed}});
    assert!(invoke(&webview, "remove_library_entry", removal(false)).is_err());
    let saved = invoke_ok(
        &webview,
        "get_dashboard_insights",
        json!({"request": {"mediaKind": "MANGA"}}),
    );
    assert_eq!(saved["libraryItems"][0]["entry"]["progress"], 12);
    assert_eq!(saved["libraryItems"][0]["entry"]["volumeProgress"], 2);
    invoke_ok(&webview, "remove_library_entry", removal(true));
    let remaining = invoke_ok(&webview, "list_library", json!({}));
    assert_eq!(remaining.as_array().expect("array").len(), 1);
    assert_eq!(remaining[0]["entry"], original_anime);
}
