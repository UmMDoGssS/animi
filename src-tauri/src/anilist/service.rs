use std::future::Future;

use chrono::Duration;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

use crate::database::{CacheRecord, Database};
use crate::domain::{CommandError, format_utc};

use super::{AiringSchedule, AniListClient, AniListError, MediaDetails, MediaSummary};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CachePolicy {
    Details,
    Discovery,
    Recommendation,
    Airing,
}

impl CachePolicy {
    pub fn ttl(self) -> Duration {
        match self {
            Self::Details => Duration::hours(24),
            Self::Discovery | Self::Recommendation => Duration::hours(6),
            Self::Airing => Duration::minutes(30),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Freshness {
    pub fetched_at: String,
    pub expires_at: String,
    pub is_stale: bool,
    pub refresh_error: Option<CommandError>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaPageResponse {
    pub items: Vec<MediaSummary>,
    pub has_next_page: bool,
    pub freshness: Freshness,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaDetailsResponse {
    pub media: MediaDetails,
    pub freshness: Freshness,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AiringScheduleResponse {
    pub items: Vec<AiringSchedule>,
    pub has_next_page: bool,
    pub freshness: Freshness,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CachedPage<T> {
    items: Vec<T>,
    has_next_page: bool,
}

#[derive(Debug, Clone)]
struct CachedResult<T> {
    value: T,
    freshness: Freshness,
}

#[derive(Debug, Clone)]
pub struct AniListService {
    database: Database,
    client: AniListClient,
}

impl AniListService {
    pub fn new(database: Database, client: AniListClient) -> Self {
        Self { database, client }
    }

    pub async fn search_media(
        &self,
        search: &str,
        page: u16,
        per_page: u8,
    ) -> Result<MediaPageResponse, CommandError> {
        validate_page(page, per_page)?;
        let search = search.trim();
        if search.is_empty() || search.chars().count() > 200 {
            return Err(CommandError::new(
                "INVALID_SEARCH",
                "Search text must contain between 1 and 200 characters.",
                false,
            ));
        }
        let key = format!(
            "discovery:search:anime:{page}:{per_page}:{}",
            search.to_lowercase()
        );
        let cached = self
            .fetch_cached(&key, CachePolicy::Discovery, || async {
                self.client.search_media(search, page, per_page).await.map(
                    |(items, has_next_page)| CachedPage {
                        items,
                        has_next_page,
                    },
                )
            })
            .await?;
        Ok(MediaPageResponse {
            items: cached.value.items,
            has_next_page: cached.value.has_next_page,
            freshness: cached.freshness,
        })
    }

    pub async fn list_trending(
        &self,
        page: u16,
        per_page: u8,
    ) -> Result<MediaPageResponse, CommandError> {
        validate_page(page, per_page)?;
        let key = format!("discovery:trending:anime:{page}:{per_page}");
        let cached = self
            .fetch_cached(&key, CachePolicy::Discovery, || async {
                self.client
                    .trending_media(page, per_page)
                    .await
                    .map(|(items, has_next_page)| CachedPage {
                        items,
                        has_next_page,
                    })
            })
            .await?;
        Ok(MediaPageResponse {
            items: cached.value.items,
            has_next_page: cached.value.has_next_page,
            freshness: cached.freshness,
        })
    }

    pub async fn get_media_details(
        &self,
        anilist_id: i64,
    ) -> Result<MediaDetailsResponse, CommandError> {
        validate_anilist_id(anilist_id)?;
        let key = format!("details:anime:{anilist_id}");
        let cached = self
            .fetch_cached(&key, CachePolicy::Details, || {
                self.client.media_details(anilist_id)
            })
            .await?;
        Ok(MediaDetailsResponse {
            media: cached.value,
            freshness: cached.freshness,
        })
    }

    pub async fn airing_schedule(
        &self,
        anilist_id: i64,
        page: u16,
        per_page: u8,
    ) -> Result<AiringScheduleResponse, CommandError> {
        validate_anilist_id(anilist_id)?;
        validate_page(page, per_page)?;
        let key = format!("airing:anime:{anilist_id}:{page}:{per_page}");
        let cached = self
            .fetch_cached(&key, CachePolicy::Airing, || async {
                self.client
                    .airing_schedule(anilist_id, page, per_page)
                    .await
                    .map(|(items, has_next_page)| CachedPage {
                        items,
                        has_next_page,
                    })
            })
            .await?;
        Ok(AiringScheduleResponse {
            items: cached.value.items,
            has_next_page: cached.value.has_next_page,
            freshness: cached.freshness,
        })
    }

    async fn fetch_cached<T, Fetch, FetchFuture>(
        &self,
        cache_key: &str,
        policy: CachePolicy,
        fetch: Fetch,
    ) -> Result<CachedResult<T>, CommandError>
    where
        T: Clone + Serialize + DeserializeOwned,
        Fetch: FnOnce() -> FetchFuture,
        FetchFuture: Future<Output = Result<T, AniListError>>,
    {
        let now = self.client.now();
        let cache = self
            .database
            .cache()
            .get(cache_key)
            .await
            .map_err(|error| error.into_command_error())?;
        let usable_cache = cache.and_then(|record| {
            serde_json::from_value::<T>(record.payload.clone())
                .ok()
                .map(|value| (record, value))
        });

        if let Some((record, value)) = usable_cache
            .as_ref()
            .filter(|(record, _)| record.expires_at > now)
        {
            return Ok(CachedResult {
                value: value.clone(),
                freshness: freshness(record, false, None),
            });
        }

        match fetch().await {
            Ok(value) => {
                let record = CacheRecord {
                    cache_key: cache_key.to_owned(),
                    payload: serde_json::to_value(&value).map_err(|_| {
                        CommandError::new(
                            "CACHE_SERIALIZATION_FAILED",
                            "Animi could not prepare AniList data for its local cache.",
                            false,
                        )
                    })?,
                    fetched_at: now,
                    expires_at: now + policy.ttl(),
                };
                self.database
                    .cache()
                    .upsert(&record)
                    .await
                    .map_err(|error| error.into_command_error())?;
                Ok(CachedResult {
                    value,
                    freshness: freshness(&record, false, None),
                })
            }
            Err(error) => {
                let command_error = error.into_command_error();
                if let Some((record, value)) = usable_cache {
                    Ok(CachedResult {
                        value,
                        freshness: freshness(&record, true, Some(command_error)),
                    })
                } else {
                    Err(command_error)
                }
            }
        }
    }
}

impl AniListError {
    pub fn into_command_error(self) -> CommandError {
        let retryable = self.retryable();
        match self {
            Self::Offline(_) => CommandError::new(
                "ANILIST_OFFLINE",
                "AniList is unavailable and no cached data could be used.",
                retryable,
            ),
            Self::Forbidden => CommandError::new(
                "ANILIST_FORBIDDEN",
                "AniList rejected the metadata request.",
                retryable,
            ),
            Self::RateLimited {
                retry_after_seconds,
            } => CommandError::new(
                "ANILIST_RATE_LIMITED",
                retry_after_seconds.map_or_else(
                    || "AniList is rate limiting requests. Try again later.".to_owned(),
                    |seconds| {
                        format!(
                            "AniList is rate limiting requests. Try again in {seconds} seconds."
                        )
                    },
                ),
                retryable,
            ),
            Self::Http { status, .. } => CommandError::new(
                "ANILIST_HTTP_ERROR",
                format!("AniList returned HTTP status {status}."),
                retryable,
            ),
            Self::Contract(_) | Self::GraphQl(_) => CommandError::new(
                "ANILIST_RESPONSE_INVALID",
                "AniList returned metadata Animi could not safely use.",
                retryable,
            ),
        }
    }
}

fn validate_page(page: u16, per_page: u8) -> Result<(), CommandError> {
    if page == 0 {
        return Err(CommandError::new(
            "INVALID_PAGE",
            "Page numbers start at 1.",
            false,
        ));
    }
    if !(1..=50).contains(&per_page) {
        return Err(CommandError::new(
            "INVALID_PAGE_SIZE",
            "Page size must be between 1 and 50.",
            false,
        ));
    }
    Ok(())
}

fn validate_anilist_id(anilist_id: i64) -> Result<(), CommandError> {
    if !(1..=i64::from(i32::MAX)).contains(&anilist_id) {
        return Err(CommandError::new(
            "INVALID_ANILIST_ID",
            "The AniList identifier is outside the supported range.",
            false,
        ));
    }
    Ok(())
}

fn freshness(
    record: &CacheRecord,
    is_stale: bool,
    refresh_error: Option<CommandError>,
) -> Freshness {
    Freshness {
        fetched_at: format_utc(record.fetched_at),
        expires_at: format_utc(record.expires_at),
        is_stale,
        refresh_error,
    }
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, VecDeque};
    use std::sync::{Arc, Mutex};

    use chrono::{DateTime, Duration, Utc};
    use tempfile::tempdir;

    use crate::anilist::{
        AniListClock, AniListRequest, AniListResponse, AniListTransport, TransportError,
        TransportFuture,
    };
    use crate::database::{Database, DatabasePaths};

    use super::{AniListClient, AniListService, CachePolicy};

    struct TestClock {
        now: Mutex<DateTime<Utc>>,
    }

    impl TestClock {
        fn new(now: DateTime<Utc>) -> Self {
            Self {
                now: Mutex::new(now),
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
    struct ScriptedTransport {
        responses: Mutex<VecDeque<Result<AniListResponse, TransportError>>>,
        request_count: Mutex<usize>,
    }

    impl ScriptedTransport {
        fn new(
            responses: impl IntoIterator<Item = Result<AniListResponse, TransportError>>,
        ) -> Self {
            Self {
                responses: Mutex::new(responses.into_iter().collect()),
                request_count: Mutex::new(0),
            }
        }

        fn request_count(&self) -> usize {
            *self.request_count.lock().expect("request count lock")
        }
    }

    impl AniListTransport for ScriptedTransport {
        fn send(&self, _request: AniListRequest) -> TransportFuture<'_> {
            *self.request_count.lock().expect("request count lock") += 1;
            let response = self
                .responses
                .lock()
                .expect("response lock")
                .pop_front()
                .expect("scripted response");
            Box::pin(async move { response })
        }
    }

    fn response(fixture: &str) -> Result<AniListResponse, TransportError> {
        Ok(AniListResponse {
            status: 200,
            headers: BTreeMap::new(),
            body: fixture.to_owned(),
        })
    }

    async fn service(
        responses: impl IntoIterator<Item = Result<AniListResponse, TransportError>>,
    ) -> (
        tempfile::TempDir,
        Database,
        AniListService,
        Arc<ScriptedTransport>,
        Arc<TestClock>,
    ) {
        let root = tempdir().expect("temporary database root");
        let paths = DatabasePaths::from_root(root.path()).expect("database paths");
        let database = Database::connect(&paths).await.expect("test database");
        let transport = Arc::new(ScriptedTransport::new(responses));
        let clock = Arc::new(TestClock::new(
            "2026-07-16T08:00:00Z".parse().expect("test timestamp"),
        ));
        let client = AniListClient::with_clock(transport.clone(), clock.clone());
        let service = AniListService::new(database.clone(), client);
        (root, database, service, transport, clock)
    }

    #[test]
    fn cache_ttls_match_the_product_contract() {
        assert_eq!(CachePolicy::Details.ttl(), Duration::hours(24));
        assert_eq!(CachePolicy::Discovery.ttl(), Duration::hours(6));
        assert_eq!(CachePolicy::Recommendation.ttl(), Duration::hours(6));
        assert_eq!(CachePolicy::Airing.ttl(), Duration::minutes(30));
    }

    #[tokio::test]
    async fn fresh_discovery_cache_avoids_an_unnecessary_request() {
        let (_root, _database, service, transport, _clock) = service([response(include_str!(
            "../../tests/fixtures/anilist/search-success.json"
        ))])
        .await;

        let first = service.list_trending(1, 20).await.expect("network result");
        let second = service.list_trending(1, 20).await.expect("cached result");

        assert_eq!(first.items, second.items);
        assert!(!second.freshness.is_stale);
        assert_eq!(transport.request_count(), 1);
    }

    #[tokio::test]
    async fn expired_cache_attempts_refresh_and_falls_back_to_stale_data() {
        let (_root, _database, service, transport, clock) = service([
            response(include_str!(
                "../../tests/fixtures/anilist/search-success.json"
            )),
            Err(TransportError::Unavailable("fixture offline".to_owned())),
        ])
        .await;
        let first = service.list_trending(1, 20).await.expect("network result");
        clock.advance(Duration::hours(7));

        let stale = service.list_trending(1, 20).await.expect("stale fallback");

        assert_eq!(stale.items, first.items);
        assert!(stale.freshness.is_stale);
        assert_eq!(
            stale
                .freshness
                .refresh_error
                .as_ref()
                .map(|error| error.code),
            Some("ANILIST_OFFLINE")
        );
        assert_eq!(transport.request_count(), 2);
    }

    #[tokio::test]
    async fn offline_without_usable_cache_returns_a_terminal_retryable_error() {
        let (_root, _database, service, transport, _clock) = service([Err(
            TransportError::Unavailable("fixture offline".to_owned()),
        )])
        .await;

        let error = service
            .list_trending(1, 20)
            .await
            .expect_err("no cache fallback");

        assert_eq!(error.code, "ANILIST_OFFLINE");
        assert!(error.retryable);
        assert_eq!(transport.request_count(), 1);
    }

    #[tokio::test]
    async fn detail_and_airing_policies_retain_their_distinct_expiry_times() {
        let (_root, _database, service, _transport, _clock) = service([
            response(include_str!(
                "../../tests/fixtures/anilist/details-incomplete.json"
            )),
            response(include_str!(
                "../../tests/fixtures/anilist/airing-success.json"
            )),
        ])
        .await;

        let details = service.get_media_details(21).await.expect("detail result");
        let airing = service
            .airing_schedule(21, 1, 25)
            .await
            .expect("airing result");
        let detail_fetched: DateTime<Utc> = details
            .freshness
            .fetched_at
            .parse()
            .expect("detail fetched time");
        let detail_expiry: DateTime<Utc> =
            details.freshness.expires_at.parse().expect("detail expiry");
        let airing_fetched: DateTime<Utc> = airing
            .freshness
            .fetched_at
            .parse()
            .expect("airing fetched time");
        let airing_expiry: DateTime<Utc> =
            airing.freshness.expires_at.parse().expect("airing expiry");

        assert_eq!(detail_expiry - detail_fetched, Duration::hours(24));
        assert_eq!(airing_expiry - airing_fetched, Duration::minutes(30));
    }

    #[tokio::test]
    async fn invalid_requests_fail_before_cache_or_transport_work() {
        let (_root, _database, service, transport, _clock) = service([]).await;

        assert_eq!(
            service
                .search_media("   ", 1, 20)
                .await
                .expect_err("empty search")
                .code,
            "INVALID_SEARCH"
        );
        assert_eq!(
            service
                .list_trending(0, 20)
                .await
                .expect_err("invalid page")
                .code,
            "INVALID_PAGE"
        );
        assert_eq!(transport.request_count(), 0);
    }
}
