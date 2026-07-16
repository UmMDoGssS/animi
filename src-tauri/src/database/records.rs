use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::domain::MediaKind;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaTag {
    pub name: String,
    pub rank: i64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MediaSnapshotRecord {
    pub anilist_id: i64,
    pub media_kind: MediaKind,
    pub title: String,
    pub cover_url: Option<String>,
    pub media_format: Option<String>,
    pub genres: Vec<String>,
    pub tags: Vec<MediaTag>,
    pub media_status: Option<String>,
    pub total_units: Option<i64>,
    pub cached_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AiringDataRecord {
    pub anilist_id: i64,
    pub media_kind: MediaKind,
    pub episode_number: i64,
    pub airing_at: DateTime<Utc>,
    pub fetched_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SettingRecord {
    pub key: String,
    pub value: serde_json::Value,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CacheRecord {
    pub cache_key: String,
    pub payload: serde_json::Value,
    pub fetched_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}
