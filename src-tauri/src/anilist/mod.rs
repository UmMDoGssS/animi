mod client;
mod queries;
mod types;

pub use client::{
    AniListClient, AniListError, AniListRequest, AniListResponse, AniListTransport,
    NetworkDisabledTransport, ReqwestTransport, TransportError, TransportFuture,
};
pub use types::{AiringSchedule, MediaDetails, MediaSummary, MediaTag, Sequel};

pub const GRAPHQL_ENDPOINT: &str = "https://graphql.anilist.co";

#[cfg(test)]
mod tests;
