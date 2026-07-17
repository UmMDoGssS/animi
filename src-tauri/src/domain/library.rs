use chrono::{DateTime, SecondsFormat, Utc};
use serde::{Deserialize, Serialize};

use super::CommandError;

pub const MAX_NOTES_CHARACTERS: usize = 10_000;
pub const MEDIA_DETAILS_CACHE_TTL_HOURS: i64 = 24;

/// IPC timestamps are serialized as UTC RFC 3339 strings with millisecond precision.
pub fn format_utc(timestamp: DateTime<Utc>) -> String {
    timestamp.to_rfc3339_opts(SecondsFormat::Millis, true)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum MediaKind {
    Anime,
    Manga,
}

impl MediaKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Anime => "ANIME",
            Self::Manga => "MANGA",
        }
    }

    pub fn parse(value: &str) -> Result<Self, CommandError> {
        match value {
            "ANIME" => Ok(Self::Anime),
            "MANGA" => Ok(Self::Manga),
            _ => Err(CommandError::new(
                "INVALID_MEDIA_KIND",
                "The media kind is not supported.",
                false,
            )),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum LibraryStatus {
    Watching,
    Completed,
    Planning,
}

impl LibraryStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Watching => "WATCHING",
            Self::Completed => "COMPLETED",
            Self::Planning => "PLANNING",
        }
    }

    pub fn parse(value: &str) -> Result<Self, CommandError> {
        match value {
            "WATCHING" => Ok(Self::Watching),
            "COMPLETED" => Ok(Self::Completed),
            "PLANNING" => Ok(Self::Planning),
            _ => Err(CommandError::new(
                "INVALID_LIBRARY_STATUS",
                "The library status is not supported.",
                false,
            )),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct UpsertLibraryEntryRequest {
    pub anilist_id: i64,
    pub media_kind: MediaKind,
    pub status: LibraryStatus,
    pub progress: i64,
    pub score: Option<f64>,
    pub notes: Option<String>,
}

impl UpsertLibraryEntryRequest {
    pub fn validate(self, known_total: Option<i64>) -> Result<ValidatedLibraryEntry, CommandError> {
        if !(1..=i64::from(i32::MAX)).contains(&self.anilist_id) {
            return Err(CommandError::new(
                "INVALID_ANILIST_ID",
                "The AniList identifier is outside the supported range.",
                false,
            ));
        }

        if !(0..=i64::from(i32::MAX)).contains(&self.progress) {
            return Err(CommandError::new(
                "INVALID_PROGRESS",
                "Progress is outside the supported range.",
                false,
            ));
        }

        if known_total.is_some_and(|total| self.progress > total) {
            return Err(CommandError::new(
                "PROGRESS_EXCEEDS_TOTAL",
                "Progress cannot exceed the known media total.",
                false,
            ));
        }

        if self
            .score
            .is_some_and(|score| !score.is_finite() || !(0.0..=10.0).contains(&score))
        {
            return Err(CommandError::new(
                "INVALID_SCORE",
                "Score must be between 0 and 10.",
                false,
            ));
        }

        let notes = self.notes.and_then(|notes| {
            let trimmed = notes.trim().to_owned();
            (!trimmed.is_empty()).then_some(trimmed)
        });

        if notes
            .as_ref()
            .is_some_and(|notes| notes.chars().count() > MAX_NOTES_CHARACTERS)
        {
            return Err(CommandError::new(
                "NOTES_TOO_LONG",
                format!("Notes cannot exceed {MAX_NOTES_CHARACTERS} characters."),
                false,
            ));
        }

        Ok(ValidatedLibraryEntry {
            anilist_id: self.anilist_id,
            media_kind: self.media_kind,
            status: self.status,
            progress: self.progress,
            score: self.score,
            notes,
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ValidatedLibraryEntry {
    pub anilist_id: i64,
    pub media_kind: MediaKind,
    pub status: LibraryStatus,
    pub progress: i64,
    pub score: Option<f64>,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RemoveLibraryEntryRequest {
    pub anilist_id: i64,
    pub media_kind: MediaKind,
    pub confirmed: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AdvanceLibraryProgressRequest {
    pub anilist_id: i64,
}

impl RemoveLibraryEntryRequest {
    pub fn validate(&self) -> Result<(), CommandError> {
        if !(1..=i64::from(i32::MAX)).contains(&self.anilist_id) {
            return Err(CommandError::new(
                "INVALID_ANILIST_ID",
                "The AniList identifier is outside the supported range.",
                false,
            ));
        }
        if !self.confirmed {
            return Err(CommandError::new(
                "CONFIRMATION_REQUIRED",
                "Removing a library entry requires explicit confirmation.",
                false,
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryEntry {
    pub anilist_id: i64,
    pub media_kind: MediaKind,
    pub status: LibraryStatus,
    pub progress: i64,
    pub score: Option<f64>,
    pub notes: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryMediaTag {
    pub name: String,
    pub rank: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryMediaSnapshot {
    pub title: String,
    pub cover_url: Option<String>,
    pub media_format: Option<String>,
    pub genres: Vec<String>,
    pub tags: Vec<LibraryMediaTag>,
    pub status: Option<String>,
    pub episodes: Option<i64>,
    pub duration_minutes: Option<i64>,
    pub cached_at: String,
    pub expires_at: String,
    pub is_stale: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryItem {
    pub entry: LibraryEntry,
    pub media: Option<LibraryMediaSnapshot>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WatchActivity {
    pub anilist_id: i64,
    pub media_kind: MediaKind,
    pub progress_delta: i64,
    pub episode_duration_minutes: Option<i64>,
    pub occurred_at: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoveLibraryEntryResponse {
    pub removed: bool,
}

#[cfg(test)]
mod tests {
    use super::{LibraryStatus, MediaKind, UpsertLibraryEntryRequest};

    fn request(progress: i64) -> UpsertLibraryEntryRequest {
        UpsertLibraryEntryRequest {
            anilist_id: 1,
            media_kind: MediaKind::Anime,
            status: LibraryStatus::Watching,
            progress,
            score: None,
            notes: None,
        }
    }

    #[test]
    fn rejects_progress_outside_a_known_total() {
        let error = request(13)
            .validate(Some(12))
            .expect_err("progress above a known total should fail");
        assert_eq!(error.code, "PROGRESS_EXCEEDS_TOTAL");
    }

    #[test]
    fn accepts_progress_when_the_total_is_unknown() {
        assert!(request(13).validate(None).is_ok());
    }

    #[test]
    fn trims_notes_and_converts_blank_notes_to_none() {
        let mut value = request(0);
        value.notes = Some("  paws  ".to_owned());
        assert_eq!(
            value.validate(None).expect("valid notes").notes.as_deref(),
            Some("paws")
        );

        let mut blank = request(0);
        blank.notes = Some("   ".to_owned());
        assert_eq!(blank.validate(None).expect("blank notes").notes, None);
    }

    #[test]
    fn rejects_invalid_identifiers_progress_and_scores() {
        let mut invalid_id = request(0);
        invalid_id.anilist_id = 0;
        assert_eq!(
            invalid_id.validate(None).expect_err("invalid id").code,
            "INVALID_ANILIST_ID"
        );

        assert_eq!(
            request(-1)
                .validate(None)
                .expect_err("negative progress")
                .code,
            "INVALID_PROGRESS"
        );

        let mut invalid_score = request(0);
        invalid_score.score = Some(10.1);
        assert_eq!(
            invalid_score
                .validate(None)
                .expect_err("out-of-range score")
                .code,
            "INVALID_SCORE"
        );
    }

    #[test]
    fn serializes_the_documented_camel_case_ipc_contract() {
        let entry = super::LibraryEntry {
            anilist_id: 101,
            media_kind: MediaKind::Anime,
            status: LibraryStatus::Watching,
            progress: 3,
            score: Some(8.5),
            notes: None,
            created_at: "2026-07-16T08:00:00.000Z".to_owned(),
            updated_at: "2026-07-16T08:30:00.000Z".to_owned(),
        };
        let value = serde_json::to_value(entry).expect("serializable entry");

        assert_eq!(value["anilistId"], 101);
        assert_eq!(value["mediaKind"], "ANIME");
        assert_eq!(value["createdAt"], "2026-07-16T08:00:00.000Z");
        assert!(value.get("anilist_id").is_none());
    }

    #[test]
    fn serializes_enriched_library_and_activity_contracts_in_camel_case() {
        let item = super::LibraryItem {
            entry: super::LibraryEntry {
                anilist_id: 101,
                media_kind: MediaKind::Anime,
                status: LibraryStatus::Watching,
                progress: 3,
                score: None,
                notes: None,
                created_at: "2026-07-16T08:00:00.000Z".to_owned(),
                updated_at: "2026-07-16T08:30:00.000Z".to_owned(),
            },
            media: Some(super::LibraryMediaSnapshot {
                title: "A Test Anime".to_owned(),
                cover_url: None,
                media_format: Some("TV".to_owned()),
                genres: vec!["Drama".to_owned()],
                tags: vec![super::LibraryMediaTag {
                    name: "Found Family".to_owned(),
                    rank: 91,
                }],
                status: Some("RELEASING".to_owned()),
                episodes: Some(12),
                duration_minutes: Some(24),
                cached_at: "2026-07-16T08:00:00.000Z".to_owned(),
                expires_at: "2026-07-17T08:00:00.000Z".to_owned(),
                is_stale: false,
            }),
        };
        let activity = super::WatchActivity {
            anilist_id: 101,
            media_kind: MediaKind::Anime,
            progress_delta: 2,
            episode_duration_minutes: None,
            occurred_at: "2026-07-16T08:30:00.000Z".to_owned(),
        };

        let item_value = serde_json::to_value(item).expect("serializable item");
        let activity_value = serde_json::to_value(activity).expect("serializable activity");

        assert_eq!(item_value["media"]["durationMinutes"], 24);
        assert_eq!(item_value["media"]["isStale"], false);
        assert_eq!(activity_value["progressDelta"], 2);
        assert!(activity_value["episodeDurationMinutes"].is_null());
        assert!(activity_value.get("progress_delta").is_none());
    }
}
