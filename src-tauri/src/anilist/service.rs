use std::collections::BTreeMap;
use std::future::Future;

use chrono::Duration;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

use crate::database::{CacheRecord, Database, MediaSnapshotRecord, MediaTag as StoredMediaTag};
use crate::domain::{
    CommandError, LibraryItem, LibraryStatus, MEDIA_DETAILS_CACHE_TTL_HOURS, MediaKind, format_utc,
};
use crate::recommendations::{Recommendation, rank_recommendations};

use super::{AiringSchedule, AniListClient, AniListError, MediaDetails, MediaSummary};

// Bump this whenever a persisted detail payload gains a field that library snapshots need.
// Older payloads deserialize safely, but reusing them would leave the new snapshot field empty.
const MEDIA_DETAILS_CACHE_SCHEMA_VERSION: u8 = 4;
const LIBRARY_METADATA_BATCH_SIZE: usize = 50;

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
            Self::Details => Duration::hours(MEDIA_DETAILS_CACHE_TTL_HOURS),
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

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecommendationsResponse {
    pub items: Vec<Recommendation>,
    pub is_fallback: bool,
    pub eligible_profile_titles: usize,
    pub freshness: Freshness,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AiringScheduleResponse {
    pub items: Vec<AiringSchedule>,
    pub has_next_page: bool,
    pub freshness: Freshness,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HomeAiringItem {
    pub library: LibraryItem,
    pub next_airing: Option<AiringSchedule>,
    pub schedule_freshness: Option<Freshness>,
    pub schedule_error: Option<CommandError>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HomeAiringResponse {
    pub items: Vec<HomeAiringItem>,
    pub checked_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MetadataRefreshFailure {
    pub anilist_id: i64,
    pub error: CommandError,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RefreshLibraryMetadataResponse {
    pub attempted: usize,
    pub updated: usize,
    pub stale: usize,
    pub failures: Vec<MetadataRefreshFailure>,
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

    pub async fn recommendations(
        &self,
        per_page: u8,
    ) -> Result<RecommendationsResponse, CommandError> {
        validate_page(1, per_page)?;
        let library = self
            .database
            .library()
            .list_items()
            .await
            .map_err(|error| error.into_command_error())?;
        let key = format!("recommendation:candidates:anime:1:{per_page}");
        let cached = self
            .fetch_cached(&key, CachePolicy::Recommendation, || async {
                self.client
                    .trending_media(1, per_page)
                    .await
                    .map(|(items, has_next_page)| CachedPage {
                        items,
                        has_next_page,
                    })
            })
            .await?;
        let ranked = rank_recommendations(&library, cached.value.items);
        Ok(RecommendationsResponse {
            items: ranked.items,
            is_fallback: ranked.is_fallback,
            eligible_profile_titles: ranked.eligible_profile_titles,
            freshness: cached.freshness,
        })
    }

    pub async fn get_media_details(
        &self,
        anilist_id: i64,
    ) -> Result<MediaDetailsResponse, CommandError> {
        let cached = self.cached_media_details(anilist_id).await?;
        let snapshot = media_snapshot(&cached.value, &cached.freshness)?;
        self.database
            .media()
            .upsert_snapshot(&snapshot)
            .await
            .map_err(|error| error.into_command_error())?;
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
        if self
            .database
            .media()
            .get_snapshot(anilist_id, MediaKind::Anime)
            .await
            .map_err(|error| error.into_command_error())?
            .is_some()
        {
            let fetched_at = self.client.now();
            let schedules = cached
                .value
                .items
                .iter()
                .map(|schedule| crate::database::AiringDataRecord {
                    anilist_id,
                    media_kind: MediaKind::Anime,
                    episode_number: schedule.episode,
                    airing_at: schedule.airing_at,
                    fetched_at,
                })
                .collect::<Vec<_>>();
            self.database
                .media()
                .replace_airing(anilist_id, MediaKind::Anime, &schedules)
                .await
                .map_err(|error| error.into_command_error())?;
        }
        Ok(AiringScheduleResponse {
            items: cached.value.items,
            has_next_page: cached.value.has_next_page,
            freshness: cached.freshness,
        })
    }

    pub async fn home_airing(&self) -> Result<HomeAiringResponse, CommandError> {
        let watching = self
            .database
            .library()
            .list_items()
            .await
            .map_err(|error| error.into_command_error())?
            .into_iter()
            .filter(|item| {
                item.entry.media_kind == MediaKind::Anime
                    && item.entry.status == LibraryStatus::Watching
            })
            .collect::<Vec<_>>();
        let mut items = Vec::with_capacity(watching.len());
        for library in watching {
            match self.airing_schedule(library.entry.anilist_id, 1, 1).await {
                Ok(response) => items.push(HomeAiringItem {
                    library,
                    next_airing: response.items.into_iter().next(),
                    schedule_freshness: Some(response.freshness),
                    schedule_error: None,
                }),
                Err(error) => items.push(HomeAiringItem {
                    library,
                    next_airing: None,
                    schedule_freshness: None,
                    schedule_error: Some(error),
                }),
            }
        }
        Ok(HomeAiringResponse {
            items,
            checked_at: format_utc(self.client.now()),
        })
    }

    pub async fn refresh_library_metadata(
        &self,
    ) -> Result<RefreshLibraryMetadataResponse, CommandError> {
        let tracked_anime = self
            .database
            .library()
            .list()
            .await
            .map_err(|error| error.into_command_error())?
            .into_iter()
            .filter(|entry| entry.media_kind == MediaKind::Anime)
            .map(|entry| entry.anilist_id)
            .collect::<Vec<_>>();
        let mut response = RefreshLibraryMetadataResponse {
            attempted: tracked_anime.len(),
            updated: 0,
            stale: 0,
            failures: Vec::new(),
        };
        let mut snapshots = Vec::with_capacity(tracked_anime.len());

        // AniList accepts up to 50 IDs in one media page. Batching prevents a normal-sized local
        // library refresh from exhausting the shared request budget before all titles are updated.
        for batch in tracked_anime.chunks(LIBRARY_METADATA_BATCH_SIZE) {
            match self.client.library_media(batch).await {
                Ok(media) => {
                    let mut by_id = media
                        .into_iter()
                        .map(|summary| (summary.anilist_id, summary))
                        .collect::<BTreeMap<_, _>>();
                    for anilist_id in batch {
                        if let Some(summary) = by_id.remove(anilist_id) {
                            snapshots.push(media_summary_snapshot(&summary, self.client.now()));
                        } else {
                            response.failures.push(MetadataRefreshFailure {
                                anilist_id: *anilist_id,
                                error: CommandError::new(
                                    "ANILIST_RESPONSE_INVALID",
                                    "AniList did not return metadata for this tracked title.",
                                    false,
                                ),
                            });
                        }
                    }
                }
                Err(error) => {
                    let error = error.into_command_error();
                    response.failures.extend(batch.iter().map(|anilist_id| {
                        MetadataRefreshFailure {
                            anilist_id: *anilist_id,
                            error: error.clone(),
                        }
                    }));
                }
            }
        }

        self.database
            .media()
            .upsert_snapshots(&snapshots)
            .await
            .map_err(|error| error.into_command_error())?;
        response.updated = snapshots.len();

        Ok(response)
    }

    async fn cached_media_details(
        &self,
        anilist_id: i64,
    ) -> Result<CachedResult<MediaDetails>, CommandError> {
        validate_anilist_id(anilist_id)?;
        let key = format!("details:v{MEDIA_DETAILS_CACHE_SCHEMA_VERSION}:anime:{anilist_id}");
        self.fetch_cached(&key, CachePolicy::Details, || {
            self.client.media_details(anilist_id)
        })
        .await
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

fn media_snapshot(
    details: &MediaDetails,
    freshness: &Freshness,
) -> Result<MediaSnapshotRecord, CommandError> {
    let cached_at = chrono::DateTime::parse_from_rfc3339(&freshness.fetched_at)
        .map(|timestamp| timestamp.with_timezone(&chrono::Utc))
        .map_err(|_| {
            CommandError::new(
                "CACHE_TIMESTAMP_INVALID",
                "Animi could not read the cached metadata timestamp.",
                false,
            )
        })?;
    Ok(media_summary_snapshot(&details.summary, cached_at))
}

fn media_summary_snapshot(
    summary: &MediaSummary,
    cached_at: chrono::DateTime<chrono::Utc>,
) -> MediaSnapshotRecord {
    MediaSnapshotRecord {
        anilist_id: summary.anilist_id,
        media_kind: MediaKind::Anime,
        title: summary.title.clone(),
        cover_url: summary.cover_url.clone(),
        media_format: summary.media_format.clone(),
        genres: summary.genres.clone(),
        tags: summary
            .tags
            .iter()
            .map(|tag| StoredMediaTag {
                name: tag.name.clone(),
                rank: i64::from(tag.rank),
            })
            .collect(),
        media_status: summary.status.clone(),
        total_units: summary.episodes,
        duration_minutes: summary.duration_minutes,
        release_date: summary.release_date.clone(),
        cached_at,
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
    use crate::commands::discovery::{
        MediaDetailsRequest, SearchMediaRequest, get_media_details_with_service,
        search_media_with_service,
    };
    use crate::commands::library::{
        list_library_with_database, remove_library_entry_with_database,
        upsert_library_entry_with_database,
    };
    use crate::database::{
        CacheRecord, Database, DatabasePaths, MediaSnapshotRecord, MediaTag as StoredMediaTag,
    };
    use crate::domain::{
        LibraryStatus, MediaKind, RemoveLibraryEntryRequest, UpsertLibraryEntryRequest,
    };

    use super::{AniListClient, AniListService, CachePolicy, LIBRARY_METADATA_BATCH_SIZE};

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

        fn extend_responses(
            &self,
            responses: impl IntoIterator<Item = Result<AniListResponse, TransportError>>,
        ) {
            self.responses
                .lock()
                .expect("response lock")
                .extend(responses);
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

    fn library_response(ids: &[i64]) -> Result<AniListResponse, TransportError> {
        let media = ids
            .iter()
            .map(|id| {
                serde_json::json!({
                    "id": id,
                    "title": { "romaji": format!("Tracked {id}"), "english": null, "native": null },
                    "coverImage": { "large": null },
                    "format": "TV",
                    "status": "FINISHED",
                    "episodes": 12,
                    "duration": 24,
                    "startDate": { "year": 2025, "month": 10, "day": 4 },
                    "genres": [],
                    "tags": [],
                    "trending": 0
                })
            })
            .collect::<Vec<_>>();
        Ok(AniListResponse {
            status: 200,
            headers: BTreeMap::new(),
            body: serde_json::json!({
                "data": {
                    "Page": {
                        "pageInfo": { "hasNextPage": false },
                        "media": media
                    }
                }
            })
            .to_string(),
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

    async fn seed_profile_title(
        database: &Database,
        anilist_id: i64,
        status: LibraryStatus,
        genres: &[&str],
        tags: &[(&str, i64)],
    ) {
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
                total_units: Some(12),
                duration_minutes: Some(24),
                release_date: None,
                cached_at: "2026-07-16T08:00:00Z".parse().expect("seed time"),
            })
            .await
            .expect("seed snapshot");
        database
            .library()
            .upsert(UpsertLibraryEntryRequest {
                anilist_id,
                media_kind: MediaKind::Anime,
                status,
                progress: 1,
                score: None,
                notes: None,
            })
            .await
            .expect("seed library entry");
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
        let ipc = serde_json::to_value(second).expect("serializable response");
        assert_eq!(ipc["items"][0]["anilistId"], 154587);
        assert!(ipc["freshness"].get("fetchedAt").is_some());
        assert!(ipc["items"][0].get("anilist_id").is_none());
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
    async fn home_airing_checks_use_the_thirty_minute_cache_and_keep_empty_schedules_unknown() {
        let (_root, database, service, transport, clock) = service([
            response(include_str!(
                "../../tests/fixtures/anilist/airing-success.json"
            )),
            response(include_str!(
                "../../tests/fixtures/anilist/airing-empty.json"
            )),
        ])
        .await;
        seed_profile_title(
            &database,
            21,
            LibraryStatus::Watching,
            &["Drama"],
            &[("Time", 80)],
        )
        .await;

        let first = service.home_airing().await.expect("launch check");
        let cached = service.home_airing().await.expect("cached check");
        clock.advance(Duration::minutes(29));
        service.home_airing().await.expect("still cached");
        assert_eq!(transport.request_count(), 1);
        assert_eq!(
            first.items[0].next_airing.as_ref().map(|item| item.episode),
            Some(29)
        );
        assert_eq!(cached.items[0].next_airing, first.items[0].next_airing);

        clock.advance(Duration::minutes(2));
        let refreshed = service.home_airing().await.expect("interval refresh");
        assert_eq!(transport.request_count(), 2);
        assert!(refreshed.items[0].next_airing.is_none());
        assert!(refreshed.items[0].schedule_error.is_none());
        assert!(
            database
                .media()
                .list_airing(21, MediaKind::Anime)
                .await
                .expect("cleared schedule rows")
                .is_empty()
        );
    }

    #[tokio::test]
    async fn recommendations_recompute_from_local_history_and_use_stale_candidates_offline() {
        let (_root, database, service, transport, clock) = service([
            response(include_str!(
                "../../tests/fixtures/anilist/recommendation-success.json"
            )),
            Err(TransportError::Unavailable("fixture offline".to_owned())),
        ])
        .await;
        seed_profile_title(
            &database,
            1,
            LibraryStatus::Watching,
            &["Drama"],
            &[("Found Family", 90)],
        )
        .await;
        seed_profile_title(
            &database,
            2,
            LibraryStatus::Completed,
            &["Fantasy"],
            &[("Found Family", 80)],
        )
        .await;

        let first = service.recommendations(18).await.expect("recommendations");
        assert!(!first.is_fallback);
        assert_eq!(first.eligible_profile_titles, 2);
        assert_eq!(first.items[0].media.anilist_id, 301);
        assert_eq!(first.items[0].explanation_labels, vec!["Found Family"]);

        clock.advance(Duration::hours(6) + Duration::seconds(1));
        let offline = service
            .recommendations(18)
            .await
            .expect("stale offline recommendations");
        assert!(offline.freshness.is_stale);
        assert_eq!(offline.items, first.items);
        assert_eq!(transport.request_count(), 2);
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
        let cached_details = service
            .get_media_details(21)
            .await
            .expect("cached detail result");
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
        assert_eq!(cached_details.media, details.media);
        assert_eq!(airing_expiry - airing_fetched, Duration::minutes(30));
    }

    #[tokio::test]
    async fn command_boundary_flow_survives_restart_and_requires_confirmed_removal() {
        let (root, database, service, transport, _clock) = service([
            response(include_str!(
                "../../tests/fixtures/anilist/search-success.json"
            )),
            response(include_str!(
                "../../tests/fixtures/anilist/details-known.json"
            )),
        ])
        .await;

        let search = search_media_with_service(
            &service,
            SearchMediaRequest {
                search: "Frieren".to_owned(),
                page: 1,
                per_page: 20,
            },
        )
        .await
        .expect("mocked search");
        assert_eq!(search.items[0].anilist_id, 154_587);
        let details = get_media_details_with_service(
            &service,
            MediaDetailsRequest {
                anilist_id: search.items[0].anilist_id,
            },
        )
        .await
        .expect("mocked details");
        assert_eq!(details.media.summary.title, "Frieren: Beyond Journey's End");
        upsert_library_entry_with_database(
            &database,
            UpsertLibraryEntryRequest {
                anilist_id: 154_587,
                media_kind: MediaKind::Anime,
                status: LibraryStatus::Watching,
                progress: 3,
                score: Some(8.0),
                notes: Some("local note".to_owned()),
            },
        )
        .await
        .expect("tracked entry");

        let first = list_library_with_database(&database)
            .await
            .expect("enriched library");
        assert_eq!(first.len(), 1);
        assert_eq!(first[0].entry.progress, 3);
        assert_eq!(first[0].entry.notes.as_deref(), Some("local note"));
        let media = first[0].media.as_ref().expect("opened snapshot");
        assert_eq!(media.title, "Frieren: Beyond Journey's End");
        assert_eq!(media.episodes, Some(28));
        assert_eq!(media.duration_minutes, Some(24));
        assert_eq!(media.release_date.as_deref(), Some("2023-09-29"));
        assert_eq!(transport.request_count(), 2);

        let paths = DatabasePaths::from_root(root.path()).expect("database paths");
        drop(service);
        database.close().await;
        let reopened = Database::connect(&paths).await.expect("reopened database");
        let persisted = list_library_with_database(&reopened)
            .await
            .expect("persisted enriched library");
        assert_eq!(persisted[0].entry, first[0].entry);
        assert_eq!(persisted[0].media, first[0].media);
        let activity = reopened.activity().list().await.expect("activity");
        assert_eq!(activity.len(), 1);
        assert_eq!(activity[0].progress_delta, 3);
        assert_eq!(activity[0].episode_duration_minutes, Some(24));

        let error = remove_library_entry_with_database(
            &reopened,
            RemoveLibraryEntryRequest {
                anilist_id: 154_587,
                media_kind: MediaKind::Anime,
                confirmed: false,
            },
        )
        .await
        .expect_err("unconfirmed removal must fail");
        assert_eq!(error.code, "CONFIRMATION_REQUIRED");
        assert_eq!(
            list_library_with_database(&reopened)
                .await
                .expect("retained library")
                .len(),
            1
        );

        let removed = remove_library_entry_with_database(
            &reopened,
            RemoveLibraryEntryRequest {
                anilist_id: 154_587,
                media_kind: MediaKind::Anime,
                confirmed: true,
            },
        )
        .await
        .expect("confirmed removal");
        assert!(removed.removed);
        assert!(
            list_library_with_database(&reopened)
                .await
                .expect("empty library")
                .is_empty()
        );
    }

    #[tokio::test]
    async fn versioned_detail_cache_refetches_payloads_without_release_dates() {
        let (_root, database, service, transport, clock) = service([response(include_str!(
            "../../tests/fixtures/anilist/details-known.json"
        ))])
        .await;
        let now = clock.now();
        database
            .cache()
            .upsert(&CacheRecord {
                cache_key: "details:v3:anime:154587".to_owned(),
                payload: serde_json::json!({
                    "anilistId": 154587,
                    "title": "Legacy cached title",
                    "coverUrl": null,
                    "mediaFormat": "TV",
                    "status": "FINISHED",
                    "episodes": 28,
                    "durationMinutes": 24,
                    "genres": [],
                    "tags": [],
                    "trending": 0,
                    "sequels": []
                }),
                fetched_at: now,
                expires_at: now + Duration::hours(24),
            })
            .await
            .expect("legacy cache");

        let details = service
            .get_media_details(154_587)
            .await
            .expect("release-date detail refresh");

        assert_eq!(details.media.summary.duration_minutes, Some(24));
        assert_eq!(
            details.media.summary.release_date,
            Some("2023-09-29".to_owned())
        );
        assert_eq!(transport.request_count(), 1);
        assert!(
            database
                .cache()
                .get("details:v4:anime:154587")
                .await
                .expect("v4 cache lookup")
                .is_some()
        );
    }

    #[tokio::test]
    async fn stale_detail_fallback_persists_truthful_library_freshness() {
        let (_root, database, service, _transport, clock) = service([
            response(include_str!(
                "../../tests/fixtures/anilist/details-known.json"
            )),
            Err(TransportError::Unavailable("fixture offline".to_owned())),
        ])
        .await;
        let first = service
            .get_media_details(154_587)
            .await
            .expect("initial details");
        database
            .library()
            .upsert(UpsertLibraryEntryRequest {
                anilist_id: 154_587,
                media_kind: MediaKind::Anime,
                status: LibraryStatus::Planning,
                progress: 0,
                score: None,
                notes: None,
            })
            .await
            .expect("tracked entry");
        clock.advance(Duration::hours(25));

        let stale = service
            .get_media_details(154_587)
            .await
            .expect("stale fallback");
        let items = database
            .library()
            .list_items_at(clock.now())
            .await
            .expect("library freshness");

        assert!(stale.freshness.is_stale);
        assert_eq!(
            stale
                .freshness
                .refresh_error
                .as_ref()
                .expect("refresh error")
                .code,
            "ANILIST_OFFLINE"
        );
        let media = items[0].media.as_ref().expect("persisted snapshot");
        assert!(media.is_stale);
        assert_eq!(media.cached_at, first.freshness.fetched_at);
        assert_eq!(media.duration_minutes, Some(24));
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

    #[tokio::test]
    async fn library_refresh_fetches_only_tracked_anime_and_persists_after_network_work() {
        let (_root, database, service, transport, _clock) = service([response(include_str!(
            "../../tests/fixtures/anilist/library-media-success.json"
        ))])
        .await;
        for (anilist_id, media_kind) in [(195_539, MediaKind::Anime), (22, MediaKind::Manga)] {
            database
                .library()
                .upsert(UpsertLibraryEntryRequest {
                    anilist_id,
                    media_kind,
                    status: LibraryStatus::Planning,
                    progress: 0,
                    score: None,
                    notes: None,
                })
                .await
                .expect("tracked entry");
        }

        let result = service
            .refresh_library_metadata()
            .await
            .expect("metadata refresh");

        assert_eq!(result.attempted, 1);
        assert_eq!(result.updated, 1);
        assert_eq!(result.stale, 0);
        assert!(result.failures.is_empty());
        assert_eq!(transport.request_count(), 1);
        assert!(
            database
                .media()
                .get_snapshot(195_539, MediaKind::Anime)
                .await
                .expect("anime snapshot")
                .is_some_and(|snapshot| snapshot.release_date.as_deref() == Some("2026"))
        );
        assert!(
            database
                .media()
                .get_snapshot(22, MediaKind::Manga)
                .await
                .expect("manga snapshot")
                .is_none()
        );
    }

    #[tokio::test]
    async fn library_refresh_batches_more_than_fifty_titles_into_two_requests() {
        let (_root, database, service, transport, _clock) = service([]).await;
        for anilist_id in 1..=63 {
            database
                .library()
                .upsert(UpsertLibraryEntryRequest {
                    anilist_id,
                    media_kind: MediaKind::Anime,
                    status: LibraryStatus::Planning,
                    progress: 0,
                    score: None,
                    notes: None,
                })
                .await
                .expect("tracked anime");
        }
        let tracked = database
            .library()
            .list()
            .await
            .expect("tracked library")
            .into_iter()
            .map(|entry| entry.anilist_id)
            .collect::<Vec<_>>();
        transport.extend_responses(
            tracked
                .chunks(LIBRARY_METADATA_BATCH_SIZE)
                .map(library_response),
        );

        let result = service
            .refresh_library_metadata()
            .await
            .expect("batched metadata refresh");

        assert_eq!(result.attempted, 63);
        assert_eq!(result.updated, 63);
        assert!(result.failures.is_empty());
        assert_eq!(transport.request_count(), 2);
    }

    #[tokio::test]
    async fn library_refresh_reports_remote_failures_without_partial_snapshot_writes() {
        let (_root, database, service, transport, _clock) = service([Err(
            TransportError::Unavailable("fixture offline".to_owned()),
        )])
        .await;
        database
            .library()
            .upsert(UpsertLibraryEntryRequest {
                anilist_id: 21,
                media_kind: MediaKind::Anime,
                status: LibraryStatus::Watching,
                progress: 1,
                score: None,
                notes: None,
            })
            .await
            .expect("tracked anime");

        let result = service
            .refresh_library_metadata()
            .await
            .expect("refresh summary");

        assert_eq!(result.attempted, 1);
        assert_eq!(result.updated, 0);
        assert_eq!(result.failures.len(), 1);
        assert_eq!(result.failures[0].error.code, "ANILIST_OFFLINE");
        assert_eq!(transport.request_count(), 1);
        assert!(
            database
                .media()
                .get_snapshot(21, MediaKind::Anime)
                .await
                .expect("snapshot lookup")
                .is_none()
        );
        assert_eq!(database.library().list().await.expect("library").len(), 1);
    }
}
