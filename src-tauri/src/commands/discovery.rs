use serde::Deserialize;
use tauri::State;

use crate::anilist::{
    AniListService, HomeAiringResponse, MediaDetailsResponse, MediaPageResponse,
    RecommendationsResponse, RefreshLibraryMetadataResponse,
};
use crate::domain::CommandError;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SearchMediaRequest {
    pub search: String,
    pub page: u16,
    pub per_page: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MediaPageRequest {
    pub page: u16,
    pub per_page: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MediaDetailsRequest {
    pub anilist_id: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RecommendationsRequest {
    pub per_page: u8,
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
) -> Result<RefreshLibraryMetadataResponse, CommandError> {
    refresh_library_metadata_with_service(&service).await
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
        .search_media(&request.search, request.page, request.per_page)
        .await
}

pub(crate) async fn list_trending_with_service(
    service: &AniListService,
    request: MediaPageRequest,
) -> Result<MediaPageResponse, CommandError> {
    service.list_trending(request.page, request.per_page).await
}

pub(crate) async fn get_media_details_with_service(
    service: &AniListService,
    request: MediaDetailsRequest,
) -> Result<MediaDetailsResponse, CommandError> {
    service.get_media_details(request.anilist_id).await
}

pub(crate) async fn refresh_library_metadata_with_service(
    service: &AniListService,
) -> Result<RefreshLibraryMetadataResponse, CommandError> {
    service.refresh_library_metadata().await
}

pub(crate) async fn get_recommendations_with_service(
    service: &AniListService,
    request: RecommendationsRequest,
) -> Result<RecommendationsResponse, CommandError> {
    service.recommendations(request.per_page).await
}

pub(crate) async fn get_home_airing_with_service(
    service: &AniListService,
) -> Result<HomeAiringResponse, CommandError> {
    service.home_airing().await
}
