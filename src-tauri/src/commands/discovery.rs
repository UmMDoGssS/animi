use serde::Deserialize;
use tauri::State;

use crate::anilist::{
    AniListService, HomeAiringResponse, MediaDetailsResponse, MediaPageResponse,
    RecommendationsResponse, RefreshLibraryMetadataResponse,
};
use crate::domain::{CommandError, MediaKind};

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SearchMediaRequest {
    pub media_kind: MediaKind,
    pub search: String,
    pub page: u16,
    pub per_page: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MediaPageRequest {
    pub media_kind: MediaKind,
    pub page: u16,
    pub per_page: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MediaDetailsRequest {
    pub media_kind: MediaKind,
    pub anilist_id: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RecommendationsRequest {
    pub media_kind: MediaKind,
    pub per_page: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RefreshLibraryMetadataRequest {
    pub media_kind: MediaKind,
}

#[tauri::command(rename_all = "camelCase")]
pub async fn search_media(
    service: State<'_, AniListService>,
    request: SearchMediaRequest,
) -> Result<MediaPageResponse, CommandError> {
    search_media_with_service(&service, request).await
}

#[tauri::command(rename_all = "camelCase")]
pub async fn list_trending(
    service: State<'_, AniListService>,
    request: MediaPageRequest,
) -> Result<MediaPageResponse, CommandError> {
    list_trending_with_service(&service, request).await
}

#[tauri::command(rename_all = "camelCase")]
pub async fn get_media_details(
    service: State<'_, AniListService>,
    request: MediaDetailsRequest,
) -> Result<MediaDetailsResponse, CommandError> {
    get_media_details_with_service(&service, request).await
}

#[tauri::command(rename_all = "camelCase")]
pub async fn refresh_library_metadata(
    service: State<'_, AniListService>,
    request: RefreshLibraryMetadataRequest,
) -> Result<RefreshLibraryMetadataResponse, CommandError> {
    refresh_library_metadata_with_service(&service, request).await
}

#[tauri::command(rename_all = "camelCase")]
pub async fn get_recommendations(
    service: State<'_, AniListService>,
    request: RecommendationsRequest,
) -> Result<RecommendationsResponse, CommandError> {
    get_recommendations_with_service(&service, request).await
}

#[tauri::command(rename_all = "camelCase")]
pub async fn get_home_airing(
    service: State<'_, AniListService>,
) -> Result<HomeAiringResponse, CommandError> {
    get_home_airing_with_service(&service).await
}

pub(crate) async fn search_media_with_service(
    service: &AniListService,
    request: SearchMediaRequest,
) -> Result<MediaPageResponse, CommandError> {
    service
        .search_media_for_kind(
            request.media_kind,
            &request.search,
            request.page,
            request.per_page,
        )
        .await
}

pub(crate) async fn list_trending_with_service(
    service: &AniListService,
    request: MediaPageRequest,
) -> Result<MediaPageResponse, CommandError> {
    service
        .list_trending_for_kind(request.media_kind, request.page, request.per_page)
        .await
}

pub(crate) async fn get_media_details_with_service(
    service: &AniListService,
    request: MediaDetailsRequest,
) -> Result<MediaDetailsResponse, CommandError> {
    service
        .get_media_details_for_kind(request.media_kind, request.anilist_id)
        .await
}

pub(crate) async fn refresh_library_metadata_with_service(
    service: &AniListService,
    request: RefreshLibraryMetadataRequest,
) -> Result<RefreshLibraryMetadataResponse, CommandError> {
    service
        .refresh_library_metadata_for_kind(request.media_kind)
        .await
}

pub(crate) async fn get_recommendations_with_service(
    service: &AniListService,
    request: RecommendationsRequest,
) -> Result<RecommendationsResponse, CommandError> {
    service
        .recommendations_for_kind(request.media_kind, request.per_page)
        .await
}

pub(crate) async fn get_home_airing_with_service(
    service: &AniListService,
) -> Result<HomeAiringResponse, CommandError> {
    service.home_airing().await
}
