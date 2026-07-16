mod client;
mod queries;
mod service;
mod types;

pub use client::{
    AniListClient, AniListClock, AniListError, AniListRequest, AniListResponse, AniListTransport,
    NetworkDisabledTransport, ReqwestTransport, SystemClock, TransportError, TransportFuture,
};
pub use service::{
    AiringScheduleResponse, AniListService, CachePolicy, Freshness, MediaDetailsResponse,
    MediaPageResponse, MetadataRefreshFailure, RefreshLibraryMetadataResponse,
};
pub use types::{AiringSchedule, MediaDetails, MediaSummary, MediaTag, Sequel};

pub const GRAPHQL_ENDPOINT: &str = "https://graphql.anilist.co";

#[cfg(test)]
mod tests;
