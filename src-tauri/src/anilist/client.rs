use std::collections::BTreeMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::time::Duration as StdDuration;

use chrono::{DateTime, Duration, Utc};
use serde::de::DeserializeOwned;
use serde_json::{Value, json};

use super::GRAPHQL_ENDPOINT;
use super::queries::{AIRING_SCHEDULE, LIBRARY_MEDIA, MEDIA_DETAILS, SEARCH_MEDIA, TRENDING_MEDIA};
use super::types::{
    AiringPageData, AiringSchedule, GraphQlEnvelope, MediaDetails, MediaDetailsData, MediaPageData,
    MediaSummary,
};

#[derive(Debug, Clone, PartialEq)]
pub struct AniListRequest {
    pub operation_name: &'static str,
    pub query: &'static str,
    pub variables: Value,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AniListResponse {
    pub status: u16,
    pub headers: BTreeMap<String, String>,
    pub body: String,
}

impl AniListResponse {
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(header, _)| header.eq_ignore_ascii_case(name))
            .map(|(_, value)| value.as_str())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum TransportError {
    #[error("network access is disabled")]
    NetworkDisabled,
    #[error("AniList is unavailable: {0}")]
    Unavailable(String),
}

pub type TransportFuture<'a> =
    Pin<Box<dyn Future<Output = Result<AniListResponse, TransportError>> + Send + 'a>>;

/// The transport is injected into the adapter so tests never need network access.
pub trait AniListTransport: Send + Sync {
    fn send(&self, request: AniListRequest) -> TransportFuture<'_>;
}

pub trait AniListClock: Send + Sync {
    fn now(&self) -> DateTime<Utc>;
}

#[derive(Debug, Default)]
pub struct SystemClock;

impl AniListClock for SystemClock {
    fn now(&self) -> DateTime<Utc> {
        Utc::now()
    }
}

/// Safe default for tests and code paths that have not explicitly installed a client.
#[derive(Debug, Default)]
pub struct NetworkDisabledTransport;

impl AniListTransport for NetworkDisabledTransport {
    fn send(&self, _request: AniListRequest) -> TransportFuture<'_> {
        Box::pin(async { Err(TransportError::NetworkDisabled) })
    }
}

#[derive(Debug, Clone)]
pub struct ReqwestTransport {
    client: reqwest::Client,
}

impl ReqwestTransport {
    pub fn new() -> Result<Self, TransportError> {
        let client = reqwest::Client::builder()
            .user_agent(concat!("Animi/", env!("CARGO_PKG_VERSION")))
            .connect_timeout(StdDuration::from_secs(10))
            .timeout(StdDuration::from_secs(30))
            .build()
            .map_err(|error| TransportError::Unavailable(error.to_string()))?;
        Ok(Self { client })
    }
}

impl AniListTransport for ReqwestTransport {
    fn send(&self, request: AniListRequest) -> TransportFuture<'_> {
        Box::pin(async move {
            let response = self
                .client
                .post(GRAPHQL_ENDPOINT)
                .json(&json!({
                    "operationName": request.operation_name,
                    "query": request.query,
                    "variables": request.variables,
                }))
                .send()
                .await
                .map_err(|error| TransportError::Unavailable(error.to_string()))?;
            let status = response.status().as_u16();
            let headers = response
                .headers()
                .iter()
                .filter_map(|(name, value)| {
                    value
                        .to_str()
                        .ok()
                        .map(|value| (name.as_str().to_owned(), value.to_owned()))
                })
                .collect();
            let body = response
                .text()
                .await
                .map_err(|error| TransportError::Unavailable(error.to_string()))?;
            Ok(AniListResponse {
                status,
                headers,
                body,
            })
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum AniListError {
    #[error("{0}")]
    Offline(String),
    #[error("AniList rejected the request")]
    Forbidden,
    #[error("AniList rate limit reached")]
    RateLimited { retry_after_seconds: Option<u64> },
    #[error("AniList returned HTTP {status}")]
    Http { status: u16, retryable: bool },
    #[error("AniList returned an invalid response: {0}")]
    Contract(String),
    #[error("AniList returned GraphQL errors: {0}")]
    GraphQl(String),
}

impl AniListError {
    pub(super) fn contract(message: impl Into<String>) -> Self {
        Self::Contract(message.into())
    }

    pub fn retryable(&self) -> bool {
        matches!(
            self,
            Self::Offline(_)
                | Self::RateLimited { .. }
                | Self::Http {
                    retryable: true,
                    ..
                }
        )
    }
}

#[derive(Clone)]
pub struct AniListClient {
    transport: Arc<dyn AniListTransport>,
    clock: Arc<dyn AniListClock>,
    blocked_until: Arc<Mutex<Option<DateTime<Utc>>>>,
}

impl std::fmt::Debug for AniListClient {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("AniListClient")
            .finish_non_exhaustive()
    }
}

impl AniListClient {
    pub fn new(transport: Arc<dyn AniListTransport>) -> Self {
        Self::with_clock(transport, Arc::new(SystemClock))
    }

    pub fn with_clock(transport: Arc<dyn AniListTransport>, clock: Arc<dyn AniListClock>) -> Self {
        Self {
            transport,
            clock,
            blocked_until: Arc::new(Mutex::new(None)),
        }
    }

    pub fn now(&self) -> DateTime<Utc> {
        self.clock.now()
    }

    pub async fn search_media(
        &self,
        search: &str,
        page: u16,
        per_page: u8,
    ) -> Result<(Vec<MediaSummary>, bool), AniListError> {
        let search = search.trim();
        if search.is_empty() {
            return Err(AniListError::contract("Search text must not be empty."));
        }
        let data: MediaPageData = self
            .execute(
                "SearchMedia",
                SEARCH_MEDIA,
                json!({ "search": search, "page": page, "perPage": per_page }),
            )
            .await?;
        map_media_page(data)
    }

    pub async fn trending_media(
        &self,
        page: u16,
        per_page: u8,
    ) -> Result<(Vec<MediaSummary>, bool), AniListError> {
        let data: MediaPageData = self
            .execute(
                "TrendingMedia",
                TRENDING_MEDIA,
                json!({ "page": page, "perPage": per_page }),
            )
            .await?;
        map_media_page(data)
    }

    pub async fn library_media(&self, ids: &[i64]) -> Result<Vec<MediaSummary>, AniListError> {
        if ids.is_empty() || ids.len() > 50 || ids.iter().any(|id| *id <= 0) {
            return Err(AniListError::contract(
                "Library metadata batches must contain 1 to 50 positive AniList IDs.",
            ));
        }
        let per_page = u8::try_from(ids.len())
            .map_err(|_| AniListError::contract("Library metadata batch is too large."))?;
        let data: MediaPageData = self
            .execute(
                "LibraryMedia",
                LIBRARY_MEDIA,
                json!({ "ids": ids, "perPage": per_page }),
            )
            .await?;
        let (media, _) = map_media_page(data)?;
        Ok(media)
    }

    pub async fn media_details(&self, id: i64) -> Result<MediaDetails, AniListError> {
        if id <= 0 {
            return Err(AniListError::contract("AniList ID must be positive."));
        }
        let data: MediaDetailsData = self
            .execute("MediaDetails", MEDIA_DETAILS, json!({ "id": id }))
            .await?;
        let mut media = data
            .media
            .ok_or_else(|| AniListError::contract("AniList did not return the requested media."))?;
        let relations = media.relations.take();
        let summary = MediaSummary::try_from(media)?;
        let sequels = relations
            .into_iter()
            .flat_map(|relations| relations.edges)
            .flatten()
            .map(|edge| edge.into_sequel())
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .flatten()
            .collect();
        Ok(MediaDetails { summary, sequels })
    }

    pub async fn airing_schedule(
        &self,
        media_id: i64,
        page: u16,
        per_page: u8,
    ) -> Result<(Vec<AiringSchedule>, bool), AniListError> {
        if media_id <= 0 {
            return Err(AniListError::contract("AniList ID must be positive."));
        }
        let data: AiringPageData = self
            .execute(
                "AiringSchedule",
                AIRING_SCHEDULE,
                json!({ "mediaId": media_id, "page": page, "perPage": per_page }),
            )
            .await?;
        let page = data.page.ok_or_else(|| {
            AniListError::contract("AniList did not return an airing schedule page.")
        })?;
        let schedules = page
            .airing_schedules
            .into_iter()
            .flatten()
            .map(AiringSchedule::try_from)
            .collect::<Result<Vec<_>, _>>()?;
        Ok((schedules, page.page_info.has_next_page))
    }

    async fn execute<T: DeserializeOwned>(
        &self,
        operation_name: &'static str,
        query: &'static str,
        variables: Value,
    ) -> Result<T, AniListError> {
        self.check_rate_limit()?;
        let response = self
            .transport
            .send(AniListRequest {
                operation_name,
                query,
                variables,
            })
            .await
            .map_err(|error| AniListError::Offline(error.to_string()))?;

        let header_retry_after = self.observe_rate_limit_headers(&response);

        match response.status {
            200..=299 => {}
            403 => return Err(AniListError::Forbidden),
            429 => {
                return Err(AniListError::RateLimited {
                    retry_after_seconds: header_retry_after,
                });
            }
            status => {
                return Err(AniListError::Http {
                    status,
                    retryable: status >= 500,
                });
            }
        }

        let envelope: GraphQlEnvelope<T> = serde_json::from_str(&response.body)
            .map_err(|error| AniListError::contract(error.to_string()))?;
        if let Some(data) = envelope.data {
            return Ok(data);
        }
        let message = envelope
            .errors
            .into_iter()
            .map(|error| error.message)
            .collect::<Vec<_>>()
            .join("; ");
        if message.is_empty() {
            Err(AniListError::contract(
                "AniList returned neither data nor an error.",
            ))
        } else {
            Err(AniListError::GraphQl(message))
        }
    }

    fn check_rate_limit(&self) -> Result<(), AniListError> {
        let now = self.now();
        let blocked_until = match self.blocked_until.lock() {
            Ok(blocked_until) => *blocked_until,
            Err(poisoned) => *poisoned.into_inner(),
        };
        if let Some(blocked_until) = blocked_until.filter(|blocked_until| *blocked_until > now) {
            let seconds = (blocked_until - now).num_seconds().max(1) as u64;
            return Err(AniListError::RateLimited {
                retry_after_seconds: Some(seconds),
            });
        }
        Ok(())
    }

    fn observe_rate_limit_headers(&self, response: &AniListResponse) -> Option<u64> {
        let now = self.now();
        let retry_after = response
            .header("Retry-After")
            .and_then(|value| value.parse::<u64>().ok());
        let retry_deadline = retry_after
            .and_then(|seconds| i64::try_from(seconds).ok())
            .and_then(|seconds| now.checked_add_signed(Duration::seconds(seconds)));
        let reset_deadline = (response.header("X-RateLimit-Remaining") == Some("0"))
            .then(|| response.header("X-RateLimit-Reset"))
            .flatten()
            .and_then(|value| value.parse::<i64>().ok())
            .and_then(|timestamp| DateTime::from_timestamp(timestamp, 0));
        let deadline = retry_deadline.into_iter().chain(reset_deadline).max();

        if let Some(deadline) = deadline.filter(|deadline| *deadline > now) {
            let mut blocked_until = match self.blocked_until.lock() {
                Ok(blocked_until) => blocked_until,
                Err(poisoned) => poisoned.into_inner(),
            };
            if blocked_until.is_none_or(|current| current < deadline) {
                *blocked_until = Some(deadline);
            }
            Some((deadline - now).num_seconds().max(1) as u64)
        } else {
            retry_after
        }
    }
}

fn map_media_page(data: MediaPageData) -> Result<(Vec<MediaSummary>, bool), AniListError> {
    let page = data
        .page
        .ok_or_else(|| AniListError::contract("AniList did not return a media page."))?;
    let media = page
        .media
        .into_iter()
        .flatten()
        .map(MediaSummary::try_from)
        .collect::<Result<Vec<_>, _>>()?;
    Ok((media, page.page_info.has_next_page))
}
