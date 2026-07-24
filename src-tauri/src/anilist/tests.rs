use std::collections::{BTreeMap, VecDeque};
use std::sync::{Arc, Mutex};

use chrono::{DateTime, Duration, Utc};

use super::{
    AniListClient, AniListClock, AniListError, AniListRequest, AniListResponse, AniListTransport,
    NetworkDisabledTransport, TransportError, TransportFuture,
};

struct TestClock {
    now: Mutex<DateTime<Utc>>,
}

impl TestClock {
    fn new() -> Self {
        Self {
            now: Mutex::new("2026-07-16T08:00:00Z".parse().expect("test timestamp")),
        }
    }

    fn advance(&self, duration: Duration) {
        let mut now = self.now.lock().expect("clock lock");
        *now += duration;
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
    fn with_response(response: Result<AniListResponse, TransportError>) -> Self {
        Self {
            responses: Mutex::new(VecDeque::from([response])),
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
            .expect("fixture response");
        Box::pin(async move { response })
    }
}

fn response(status: u16, fixture: &str) -> AniListResponse {
    AniListResponse {
        status,
        headers: BTreeMap::new(),
        body: fixture.to_owned(),
    }
}

#[tokio::test]
async fn default_transport_cannot_reach_the_network() {
    let client = AniListClient::new(Arc::new(NetworkDisabledTransport));
    let error = client
        .trending_media(1, 20)
        .await
        .expect_err("network must be disabled");

    assert!(matches!(error, AniListError::Offline(_)));
    assert!(error.retryable());
}

#[tokio::test]
async fn search_uses_anime_only_non_adult_query_and_maps_stable_types() {
    let transport = Arc::new(FixtureTransport::with_response(Ok(response(
        200,
        include_str!("../../tests/fixtures/anilist/search-success.json"),
    ))));
    let client = AniListClient::new(transport.clone());

    let (media, has_next_page) = client
        .search_media("  Frieren  ", 1, 20)
        .await
        .expect("search response");

    assert!(has_next_page);
    assert_eq!(media.len(), 1);
    assert_eq!(media[0].anilist_id, 154587);
    assert_eq!(media[0].title, "Frieren: Beyond Journey's End");
    assert_eq!(media[0].episodes, Some(28));
    assert_eq!(media[0].duration_minutes, Some(24));
    assert_eq!(media[0].release_date.as_deref(), Some("2023-09-29"));
    assert_eq!(
        serde_json::to_value(&media[0]).expect("serializable media")["durationMinutes"],
        24
    );
    assert_eq!(
        media[0].tags.len(),
        1,
        "adult tags are filtered defensively"
    );

    let requests = transport.requests();
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].operation_name, "SearchMedia");
    assert!(requests[0].query.contains("type: ANIME"));
    assert!(requests[0].query.contains("isAdult: false"));
    assert!(requests[0].query.contains("duration"));
    assert!(requests[0].query.contains("startDate"));
    assert_eq!(requests[0].variables["search"], "Frieren");
}

#[tokio::test]
async fn library_batch_uses_one_request_and_preserves_fuzzy_release_dates() {
    let transport = Arc::new(FixtureTransport::with_response(Ok(response(
        200,
        include_str!("../../tests/fixtures/anilist/library-media-success.json"),
    ))));
    let client = AniListClient::new(transport.clone());

    let media = client
        .library_media(&[195_539, 209_939])
        .await
        .expect("library metadata response");

    assert_eq!(media.len(), 2);
    assert_eq!(media[0].release_date.as_deref(), Some("2026"));
    assert_eq!(media[1].release_date.as_deref(), Some("2027-10"));
    let requests = transport.requests();
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].operation_name, "LibraryMedia");
    assert_eq!(
        requests[0].variables["ids"],
        serde_json::json!([195539, 209939])
    );
    assert_eq!(requests[0].variables["perPage"], 2);
}

#[tokio::test]
async fn details_preserve_unknown_totals_and_never_infer_sequel_dates() {
    let transport = FixtureTransport::with_response(Ok(response(
        200,
        include_str!("../../tests/fixtures/anilist/details-incomplete.json"),
    )));
    let client = AniListClient::new(Arc::new(transport));

    let details = client.media_details(21).await.expect("details response");

    assert_eq!(details.summary.episodes, None);
    assert_eq!(details.episodes_aired, Some(6));
    assert_eq!(details.summary.duration_minutes, None);
    assert_eq!(details.sequels.len(), 1);
    assert_eq!(details.sequels[0].anilist_id, 22);
    assert_eq!(details.sequels[0].release_date, None);
}

#[tokio::test]
async fn airing_schedule_maps_unix_timestamps_without_live_requests() {
    let transport = FixtureTransport::with_response(Ok(response(
        200,
        include_str!("../../tests/fixtures/anilist/airing-success.json"),
    )));
    let client = AniListClient::new(Arc::new(transport));

    let (schedule, has_next_page) = client
        .airing_schedule(154587, 1, 25)
        .await
        .expect("airing response");

    assert!(!has_next_page);
    assert_eq!(schedule.len(), 1);
    assert_eq!(schedule[0].episode, 29);
    assert_eq!(
        schedule[0].airing_at.to_rfc3339(),
        "2026-07-17T08:00:00+00:00"
    );
}

#[tokio::test]
async fn partial_graphql_errors_keep_usable_data() {
    let transport = FixtureTransport::with_response(Ok(response(
        200,
        include_str!("../../tests/fixtures/anilist/partial-errors.json"),
    )));
    let client = AniListClient::new(Arc::new(transport));

    let (media, _) = client
        .trending_media(1, 20)
        .await
        .expect("partial data remains usable");

    assert_eq!(media.len(), 1);
    assert_eq!(media[0].title, "Partial Result");
}

#[tokio::test]
async fn malformed_data_is_a_non_retryable_contract_error() {
    let transport = FixtureTransport::with_response(Ok(response(
        200,
        include_str!("../../tests/fixtures/anilist/malformed.json"),
    )));
    let client = AniListClient::new(Arc::new(transport));

    let error = client
        .trending_media(1, 20)
        .await
        .expect_err("malformed fixture must fail");

    assert!(matches!(error, AniListError::Contract(_)));
    assert!(!error.retryable());
}

#[tokio::test]
async fn non_positive_episode_duration_is_a_contract_error() {
    let transport = FixtureTransport::with_response(Ok(response(
        200,
        include_str!("../../tests/fixtures/anilist/duration-invalid.json"),
    )));
    let client = AniListClient::new(Arc::new(transport));

    let error = client
        .trending_media(1, 20)
        .await
        .expect_err("zero duration must fail");

    assert!(matches!(error, AniListError::Contract(_)));
    assert!(!error.retryable());
}

#[tokio::test]
async fn forbidden_and_offline_failures_have_distinct_retryability() {
    let forbidden = FixtureTransport::with_response(Ok(response(
        403,
        include_str!("../../tests/fixtures/anilist/forbidden.json"),
    )));
    let error = AniListClient::new(Arc::new(forbidden))
        .trending_media(1, 20)
        .await
        .expect_err("forbidden response");
    assert_eq!(error, AniListError::Forbidden);
    assert!(!error.retryable());

    let offline_message: serde_json::Value =
        serde_json::from_str(include_str!("../../tests/fixtures/anilist/offline.json"))
            .expect("offline fixture");
    let offline = FixtureTransport::with_response(Err(TransportError::Unavailable(
        offline_message["message"]
            .as_str()
            .expect("offline message")
            .to_owned(),
    )));
    let error = AniListClient::new(Arc::new(offline))
        .trending_media(1, 20)
        .await
        .expect_err("offline response");
    assert!(matches!(error, AniListError::Offline(_)));
    assert!(error.retryable());
}

#[tokio::test]
async fn rate_limit_preserves_retry_after_header() {
    let mut limited = response(
        429,
        include_str!("../../tests/fixtures/anilist/rate-limited.json"),
    );
    limited
        .headers
        .insert("retry-after".to_owned(), "30".to_owned());
    let transport = FixtureTransport::with_response(Ok(limited));

    let error = AniListClient::new(Arc::new(transport))
        .trending_media(1, 20)
        .await
        .expect_err("rate limited response");

    assert_eq!(
        error,
        AniListError::RateLimited {
            retry_after_seconds: Some(30)
        }
    );
    assert!(error.retryable());
}

#[tokio::test]
async fn retry_after_cooldown_prevents_a_request_storm() {
    let mut limited = response(
        429,
        include_str!("../../tests/fixtures/anilist/rate-limited.json"),
    );
    limited
        .headers
        .insert("Retry-After".to_owned(), "30".to_owned());
    let transport = Arc::new(FixtureTransport {
        responses: Mutex::new(VecDeque::from([
            Ok(limited),
            Ok(response(
                200,
                include_str!("../../tests/fixtures/anilist/search-success.json"),
            )),
        ])),
        requests: Mutex::new(Vec::new()),
    });
    let clock = Arc::new(TestClock::new());
    let client = AniListClient::with_clock(transport.clone(), clock.clone());

    assert!(matches!(
        client.trending_media(1, 20).await,
        Err(AniListError::RateLimited { .. })
    ));
    assert!(matches!(
        client.trending_media(1, 20).await,
        Err(AniListError::RateLimited { .. })
    ));
    assert_eq!(transport.requests().len(), 1);

    clock.advance(Duration::seconds(31));
    assert!(client.trending_media(1, 20).await.is_ok());
    assert_eq!(transport.requests().len(), 2);
}

#[tokio::test]
async fn exhausted_remaining_header_honors_the_reset_timestamp() {
    let clock = Arc::new(TestClock::new());
    let reset = clock.now() + Duration::seconds(45);
    let mut success = response(
        200,
        include_str!("../../tests/fixtures/anilist/search-success.json"),
    );
    success
        .headers
        .insert("X-RateLimit-Remaining".to_owned(), "0".to_owned());
    success.headers.insert(
        "X-RateLimit-Reset".to_owned(),
        reset.timestamp().to_string(),
    );
    let transport = Arc::new(FixtureTransport::with_response(Ok(success)));
    let client = AniListClient::with_clock(transport.clone(), clock);

    client
        .trending_media(1, 20)
        .await
        .expect("first response remains usable");
    let error = client
        .trending_media(1, 20)
        .await
        .expect_err("reset window should block another request");

    assert_eq!(
        error,
        AniListError::RateLimited {
            retry_after_seconds: Some(45)
        }
    );
    assert_eq!(transport.requests().len(), 1);
}
