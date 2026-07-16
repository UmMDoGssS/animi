use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};

use super::AniListError;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaTag {
    pub name: String,
    pub rank: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaSummary {
    pub anilist_id: i64,
    pub title: String,
    pub cover_url: Option<String>,
    pub media_format: Option<String>,
    pub status: Option<String>,
    pub episodes: Option<i64>,
    pub duration_minutes: Option<i64>,
    pub genres: Vec<String>,
    pub tags: Vec<MediaTag>,
    pub trending: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Sequel {
    pub anilist_id: i64,
    pub title: String,
    pub release_date: Option<NaiveDate>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaDetails {
    #[serde(flatten)]
    pub summary: MediaSummary,
    pub sequels: Vec<Sequel>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiringSchedule {
    pub episode: i64,
    pub airing_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub(super) struct GraphQlEnvelope<T> {
    pub data: Option<T>,
    #[serde(default)]
    pub errors: Vec<GraphQlError>,
}

#[derive(Debug, Deserialize)]
pub(super) struct GraphQlError {
    pub message: String,
}

#[derive(Debug, Deserialize)]
pub(super) struct MediaPageData {
    #[serde(rename = "Page")]
    pub page: Option<MediaPage>,
}

#[derive(Debug, Deserialize)]
pub(super) struct MediaPage {
    #[serde(rename = "pageInfo")]
    pub page_info: PageInfo,
    pub media: Vec<Option<RawMedia>>,
}

#[derive(Debug, Deserialize)]
pub(super) struct PageInfo {
    #[serde(rename = "hasNextPage")]
    pub has_next_page: bool,
}

#[derive(Debug, Deserialize)]
pub(super) struct MediaDetailsData {
    #[serde(rename = "Media")]
    pub media: Option<RawMedia>,
}

#[derive(Debug, Deserialize)]
pub(super) struct AiringPageData {
    #[serde(rename = "Page")]
    pub page: Option<AiringPage>,
}

#[derive(Debug, Deserialize)]
pub(super) struct AiringPage {
    #[serde(rename = "pageInfo")]
    pub page_info: PageInfo,
    #[serde(rename = "airingSchedules")]
    pub airing_schedules: Vec<Option<RawAiringSchedule>>,
}

#[derive(Debug, Deserialize)]
pub(super) struct RawAiringSchedule {
    pub episode: i64,
    #[serde(rename = "airingAt")]
    pub airing_at: i64,
}

#[derive(Debug, Deserialize)]
pub(super) struct RawMedia {
    pub id: i64,
    pub title: RawTitle,
    #[serde(rename = "coverImage")]
    pub cover_image: Option<RawCoverImage>,
    pub format: Option<String>,
    pub status: Option<String>,
    pub episodes: Option<i64>,
    pub duration: Option<i64>,
    #[serde(default)]
    pub genres: Vec<String>,
    #[serde(default)]
    pub tags: Vec<RawTag>,
    #[serde(default)]
    pub trending: i64,
    pub relations: Option<RawRelations>,
}

#[derive(Debug, Deserialize)]
pub(super) struct RawTitle {
    pub romaji: Option<String>,
    pub english: Option<String>,
    pub native: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(super) struct RawCoverImage {
    pub large: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(super) struct RawTag {
    pub name: String,
    pub rank: i64,
    #[serde(rename = "isAdult", default)]
    pub is_adult: bool,
}

#[derive(Debug, Deserialize)]
pub(super) struct RawRelations {
    #[serde(default)]
    pub edges: Vec<Option<RawRelationEdge>>,
}

#[derive(Debug, Deserialize)]
pub(super) struct RawRelationEdge {
    #[serde(rename = "relationType")]
    pub relation_type: String,
    pub node: RawRelationNode,
}

#[derive(Debug, Deserialize)]
pub(super) struct RawRelationNode {
    pub id: i64,
    #[serde(rename = "type")]
    pub media_type: String,
    #[serde(rename = "isAdult", default)]
    pub is_adult: bool,
    pub title: RawTitle,
    #[serde(rename = "startDate")]
    pub start_date: Option<RawFuzzyDate>,
}

#[derive(Debug, Deserialize)]
pub(super) struct RawFuzzyDate {
    pub year: Option<i32>,
    pub month: Option<u32>,
    pub day: Option<u32>,
}

impl TryFrom<RawMedia> for MediaSummary {
    type Error = AniListError;

    fn try_from(media: RawMedia) -> Result<Self, Self::Error> {
        if media.id <= 0 {
            return Err(AniListError::contract(
                "AniList returned an invalid media ID.",
            ));
        }
        if media.episodes.is_some_and(|episodes| episodes < 0) {
            return Err(AniListError::contract(
                "AniList returned an invalid episode total.",
            ));
        }
        if media.duration.is_some_and(|duration| duration <= 0) {
            return Err(AniListError::contract(
                "AniList returned an invalid episode duration.",
            ));
        }

        let title = media.title.into_title()?;
        let tags = media
            .tags
            .into_iter()
            .filter(|tag| !tag.is_adult)
            .map(|tag| {
                let rank = u8::try_from(tag.rank)
                    .map_err(|_| AniListError::contract("AniList returned an invalid tag rank."))?;
                Ok(MediaTag {
                    name: tag.name,
                    rank,
                })
            })
            .collect::<Result<Vec<_>, AniListError>>()?;

        Ok(Self {
            anilist_id: media.id,
            title,
            cover_url: media.cover_image.and_then(|cover| cover.large),
            media_format: media.format,
            status: media.status,
            episodes: media.episodes,
            duration_minutes: media.duration,
            genres: media.genres,
            tags,
            trending: media.trending,
        })
    }
}

impl RawTitle {
    fn into_title(self) -> Result<String, AniListError> {
        [self.english, self.romaji, self.native]
            .into_iter()
            .flatten()
            .find_map(|title| {
                let trimmed = title.trim();
                (!trimmed.is_empty()).then(|| trimmed.to_owned())
            })
            .ok_or_else(|| AniListError::contract("AniList returned media without a title."))
    }
}

impl RawRelationEdge {
    pub(super) fn into_sequel(self) -> Result<Option<Sequel>, AniListError> {
        if self.relation_type != "SEQUEL" || self.node.media_type != "ANIME" || self.node.is_adult {
            return Ok(None);
        }
        if self.node.id <= 0 {
            return Err(AniListError::contract(
                "AniList returned an invalid sequel ID.",
            ));
        }
        let release_date = self.node.start_date.and_then(RawFuzzyDate::complete_date);
        Ok(Some(Sequel {
            anilist_id: self.node.id,
            title: self.node.title.into_title()?,
            release_date,
        }))
    }
}

impl RawFuzzyDate {
    fn complete_date(self) -> Option<NaiveDate> {
        NaiveDate::from_ymd_opt(self.year?, self.month?, self.day?)
    }
}

impl TryFrom<RawAiringSchedule> for AiringSchedule {
    type Error = AniListError;

    fn try_from(schedule: RawAiringSchedule) -> Result<Self, Self::Error> {
        if schedule.episode <= 0 {
            return Err(AniListError::contract(
                "AniList returned an invalid episode number.",
            ));
        }
        let airing_at = DateTime::from_timestamp(schedule.airing_at, 0)
            .ok_or_else(|| AniListError::contract("AniList returned an invalid airing time."))?;
        Ok(Self {
            episode: schedule.episode,
            airing_at,
        })
    }
}
