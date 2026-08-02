use std::collections::{BTreeMap, BTreeSet};

use chrono::{DateTime, Datelike, Months, NaiveDate, Utc};
use serde::Serialize;

use super::{CommandError, LibraryItem, LibraryStatus, MediaKind, WatchActivity, format_utc};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DashboardLibraryTotals {
    pub tracked: i64,
    pub watching: i64,
    pub completed: i64,
    pub planning: i64,
    pub current_progress: i64,
    pub current_volume_progress: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DashboardGenreSignal {
    pub label: String,
    pub title_count: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DashboardTagSignal {
    pub label: String,
    pub title_count: i64,
    pub weighted_rank: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DashboardPreferenceSignals {
    pub eligible_titles: i64,
    pub metadata_titles: i64,
    pub missing_metadata_titles: i64,
    pub stale_metadata_titles: i64,
    pub genres: Vec<DashboardGenreSignal>,
    pub tags: Vec<DashboardTagSignal>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DashboardActivitySummary {
    pub event_count: i64,
    pub recorded_episodes: i64,
    pub known_duration_episodes: i64,
    pub recorded_minutes: i64,
    pub unknown_duration_episodes: i64,
    pub recorded_chapters: i64,
    pub recorded_volumes: i64,
    pub covered_from: Option<String>,
    pub covered_to: Option<String>,
    pub monthly: Vec<DashboardActivityBucket>,
    pub yearly: Vec<DashboardActivityBucket>,
    pub events: Vec<WatchActivity>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DashboardActivityBucket {
    pub period: String,
    pub recorded_episodes: i64,
    pub recorded_minutes: i64,
    pub unknown_duration_episodes: i64,
    pub recorded_chapters: i64,
    pub recorded_volumes: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DashboardInsights {
    pub media_kind: MediaKind,
    pub library_items: Vec<LibraryItem>,
    pub library_totals: DashboardLibraryTotals,
    pub preference_signals: DashboardPreferenceSignals,
    pub activity: DashboardActivitySummary,
}

impl DashboardInsights {
    pub fn build(
        library_items: Vec<LibraryItem>,
        events: Vec<WatchActivity>,
    ) -> Result<Self, CommandError> {
        Self::build_for_at(MediaKind::Anime, library_items, events, Utc::now())
    }

    pub fn build_for(
        media_kind: MediaKind,
        library_items: Vec<LibraryItem>,
        events: Vec<WatchActivity>,
    ) -> Result<Self, CommandError> {
        Self::build_for_at(media_kind, library_items, events, Utc::now())
    }

    pub fn build_at(
        library_items: Vec<LibraryItem>,
        events: Vec<WatchActivity>,
        now: DateTime<Utc>,
    ) -> Result<Self, CommandError> {
        Self::build_for_at(MediaKind::Anime, library_items, events, now)
    }

    pub fn build_for_at(
        media_kind: MediaKind,
        mut library_items: Vec<LibraryItem>,
        mut events: Vec<WatchActivity>,
        now: DateTime<Utc>,
    ) -> Result<Self, CommandError> {
        library_items.retain(|item| item.entry.media_kind == media_kind);
        events.retain(|event| event.media_kind == media_kind);
        let mut library_totals = DashboardLibraryTotals {
            tracked: i64::try_from(library_items.len()).map_err(|_| invalid_dashboard_data())?,
            watching: 0,
            completed: 0,
            planning: 0,
            current_progress: 0,
            current_volume_progress: 0,
        };
        let mut eligible_titles = 0_i64;
        let mut metadata_titles = 0_i64;
        let mut missing_metadata_titles = 0_i64;
        let mut stale_metadata_titles = 0_i64;
        let mut genre_counts = BTreeMap::<String, i64>::new();
        let mut tag_counts = BTreeMap::<String, (i64, i64)>::new();

        for item in &library_items {
            library_totals.current_progress = library_totals
                .current_progress
                .checked_add(item.entry.progress)
                .ok_or_else(invalid_dashboard_data)?;
            library_totals.current_volume_progress = library_totals
                .current_volume_progress
                .checked_add(item.entry.volume_progress.unwrap_or(0))
                .ok_or_else(invalid_dashboard_data)?;
            match item.entry.status {
                LibraryStatus::Watching => {
                    library_totals.watching = checked_increment(library_totals.watching)?;
                }
                LibraryStatus::Completed => {
                    library_totals.completed = checked_increment(library_totals.completed)?;
                }
                LibraryStatus::Planning => {
                    library_totals.planning = checked_increment(library_totals.planning)?;
                    continue;
                }
            }

            eligible_titles = checked_increment(eligible_titles)?;
            let Some(media) = &item.media else {
                missing_metadata_titles = checked_increment(missing_metadata_titles)?;
                continue;
            };
            metadata_titles = checked_increment(metadata_titles)?;
            stale_metadata_titles = stale_metadata_titles
                .checked_add(i64::from(media.is_stale))
                .ok_or_else(invalid_dashboard_data)?;
            for genre in media
                .genres
                .iter()
                .map(|genre| genre.trim())
                .filter(|genre| !genre.is_empty())
                .collect::<BTreeSet<_>>()
            {
                let count = genre_counts.entry(genre.to_owned()).or_default();
                *count = checked_increment(*count)?;
            }

            let mut title_tags = BTreeMap::<&str, i64>::new();
            for tag in &media.tags {
                let label = tag.name.trim();
                if label.is_empty() {
                    continue;
                }
                if tag.rank < 0 {
                    return Err(invalid_dashboard_data());
                }
                title_tags
                    .entry(label)
                    .and_modify(|rank| *rank = (*rank).max(tag.rank))
                    .or_insert(tag.rank);
            }
            for (label, rank) in title_tags {
                let (title_count, weighted_rank) = tag_counts.entry(label.to_owned()).or_default();
                *title_count = checked_increment(*title_count)?;
                *weighted_rank = weighted_rank
                    .checked_add(rank)
                    .ok_or_else(invalid_dashboard_data)?;
            }
        }

        let mut genre_signals = genre_counts
            .into_iter()
            .map(|(label, title_count)| DashboardGenreSignal { label, title_count })
            .collect::<Vec<_>>();
        genre_signals.sort_by(|left, right| {
            right
                .title_count
                .cmp(&left.title_count)
                .then_with(|| left.label.cmp(&right.label))
        });

        let mut tag_signals = tag_counts
            .into_iter()
            .map(|(label, (title_count, weighted_rank))| DashboardTagSignal {
                label,
                title_count,
                weighted_rank,
            })
            .collect::<Vec<_>>();
        tag_signals.sort_by(|left, right| {
            right
                .weighted_rank
                .cmp(&left.weighted_rank)
                .then_with(|| right.title_count.cmp(&left.title_count))
                .then_with(|| left.label.cmp(&right.label))
        });

        let preference_signals = DashboardPreferenceSignals {
            eligible_titles,
            metadata_titles,
            missing_metadata_titles,
            stale_metadata_titles,
            genres: genre_signals,
            tags: tag_signals,
        };
        let activity = DashboardActivitySummary::build(media_kind, events, now)?;
        Ok(Self {
            media_kind,
            library_items,
            library_totals,
            preference_signals,
            activity,
        })
    }
}

impl DashboardActivitySummary {
    fn build(
        media_kind: MediaKind,
        events: Vec<WatchActivity>,
        now: DateTime<Utc>,
    ) -> Result<Self, CommandError> {
        let mut recorded_episodes = 0_i64;
        let mut known_duration_episodes = 0_i64;
        let mut recorded_minutes = 0_i64;
        let mut unknown_duration_episodes = 0_i64;
        let mut recorded_chapters = 0_i64;
        let mut recorded_volumes = 0_i64;
        let current_month = NaiveDate::from_ymd_opt(now.year(), now.month(), 1)
            .ok_or_else(invalid_dashboard_data)?;
        let first_month = current_month
            .checked_sub_months(Months::new(11))
            .ok_or_else(invalid_dashboard_data)?;
        let mut monthly = BTreeMap::<String, DashboardActivityBucket>::new();
        for offset in 0..12 {
            let month = first_month
                .checked_add_months(Months::new(offset))
                .ok_or_else(invalid_dashboard_data)?;
            let period = format!("{:04}-{:02}", month.year(), month.month());
            monthly.insert(period.clone(), DashboardActivityBucket::empty(period));
        }
        let mut yearly = BTreeMap::<String, DashboardActivityBucket>::new();
        let mut covered_from: Option<DateTime<Utc>> = None;
        let mut covered_to: Option<DateTime<Utc>> = None;

        for event in &events {
            if event.progress_delta < 0
                || event.volume_delta < 0
                || event.progress_delta == 0 && event.volume_delta == 0
                || event
                    .episode_duration_minutes
                    .is_some_and(|duration| duration <= 0)
            {
                return Err(invalid_dashboard_data());
            }
            if media_kind == MediaKind::Anime && event.volume_delta != 0
                || media_kind == MediaKind::Manga && event.episode_duration_minutes.is_some()
            {
                return Err(invalid_dashboard_data());
            }
            let occurred_at = DateTime::parse_from_rfc3339(&event.occurred_at)
                .map(|timestamp| timestamp.with_timezone(&Utc))
                .map_err(|_| invalid_dashboard_data())?;
            covered_from = Some(covered_from.map_or(occurred_at, |value| value.min(occurred_at)));
            covered_to = Some(covered_to.map_or(occurred_at, |value| value.max(occurred_at)));
            if media_kind == MediaKind::Manga {
                recorded_chapters = recorded_chapters
                    .checked_add(event.progress_delta)
                    .ok_or_else(invalid_dashboard_data)?;
                recorded_volumes = recorded_volumes
                    .checked_add(event.volume_delta)
                    .ok_or_else(invalid_dashboard_data)?;
            } else {
                recorded_episodes = recorded_episodes
                    .checked_add(event.progress_delta)
                    .ok_or_else(invalid_dashboard_data)?;
            }
            if media_kind == MediaKind::Anime {
                if let Some(duration) = event.episode_duration_minutes {
                    known_duration_episodes = known_duration_episodes
                        .checked_add(event.progress_delta)
                        .ok_or_else(invalid_dashboard_data)?;
                    let minutes = event
                        .progress_delta
                        .checked_mul(duration)
                        .ok_or_else(invalid_dashboard_data)?;
                    recorded_minutes = recorded_minutes
                        .checked_add(minutes)
                        .ok_or_else(invalid_dashboard_data)?;
                } else {
                    unknown_duration_episodes = unknown_duration_episodes
                        .checked_add(event.progress_delta)
                        .ok_or_else(invalid_dashboard_data)?;
                }
            }
            let year_period = format!("{:04}", occurred_at.year());
            accumulate_bucket(
                yearly
                    .entry(year_period.clone())
                    .or_insert_with(|| DashboardActivityBucket::empty(year_period)),
                media_kind,
                event,
            )?;
            let month_period = format!("{:04}-{:02}", occurred_at.year(), occurred_at.month());
            if let Some(bucket) = monthly.get_mut(&month_period) {
                accumulate_bucket(bucket, media_kind, event)?;
            }
        }

        Ok(Self {
            event_count: i64::try_from(events.len()).map_err(|_| invalid_dashboard_data())?,
            recorded_episodes,
            known_duration_episodes,
            recorded_minutes,
            unknown_duration_episodes,
            recorded_chapters,
            recorded_volumes,
            covered_from: covered_from.map(format_utc),
            covered_to: covered_to.map(format_utc),
            monthly: monthly.into_values().collect(),
            yearly: yearly.into_values().collect(),
            events,
        })
    }
}

impl DashboardActivityBucket {
    fn empty(period: String) -> Self {
        Self {
            period,
            recorded_episodes: 0,
            recorded_minutes: 0,
            unknown_duration_episodes: 0,
            recorded_chapters: 0,
            recorded_volumes: 0,
        }
    }
}

fn accumulate_bucket(
    bucket: &mut DashboardActivityBucket,
    media_kind: MediaKind,
    event: &WatchActivity,
) -> Result<(), CommandError> {
    if media_kind == MediaKind::Manga {
        bucket.recorded_chapters = bucket
            .recorded_chapters
            .checked_add(event.progress_delta)
            .ok_or_else(invalid_dashboard_data)?;
        bucket.recorded_volumes = bucket
            .recorded_volumes
            .checked_add(event.volume_delta)
            .ok_or_else(invalid_dashboard_data)?;
        return Ok(());
    }
    bucket.recorded_episodes = bucket
        .recorded_episodes
        .checked_add(event.progress_delta)
        .ok_or_else(invalid_dashboard_data)?;
    if let Some(duration) = event.episode_duration_minutes {
        bucket.recorded_minutes = bucket
            .recorded_minutes
            .checked_add(
                event
                    .progress_delta
                    .checked_mul(duration)
                    .ok_or_else(invalid_dashboard_data)?,
            )
            .ok_or_else(invalid_dashboard_data)?;
    } else {
        bucket.unknown_duration_episodes = bucket
            .unknown_duration_episodes
            .checked_add(event.progress_delta)
            .ok_or_else(invalid_dashboard_data)?;
    }
    Ok(())
}

fn checked_increment(value: i64) -> Result<i64, CommandError> {
    value.checked_add(1).ok_or_else(invalid_dashboard_data)
}

fn invalid_dashboard_data() -> CommandError {
    CommandError::new(
        "DASHBOARD_DATA_INVALID",
        "Animi could not summarize the local Dashboard data.",
        false,
    )
}

#[cfg(test)]
mod tests {
    use chrono::{TimeZone, Utc};

    use super::DashboardInsights;
    use crate::domain::{
        LibraryEntry, LibraryItem, LibraryMediaSnapshot, LibraryMediaTag, LibraryStatus, MediaKind,
        WatchActivity,
    };

    fn item(
        anilist_id: i64,
        status: LibraryStatus,
        progress: i64,
        media: Option<LibraryMediaSnapshot>,
    ) -> LibraryItem {
        LibraryItem {
            entry: LibraryEntry {
                anilist_id,
                media_kind: MediaKind::Anime,
                status,
                progress,
                volume_progress: None,
                score: None,
                notes: None,
                created_at: "2026-07-16T08:00:00.000Z".to_owned(),
                updated_at: "2026-07-16T08:30:00.000Z".to_owned(),
            },
            media,
        }
    }

    fn media(title: &str, genres: &[&str], tags: &[(&str, i64)]) -> LibraryMediaSnapshot {
        LibraryMediaSnapshot {
            title: title.to_owned(),
            cover_url: None,
            media_format: Some("TV".to_owned()),
            genres: genres.iter().map(|genre| (*genre).to_owned()).collect(),
            tags: tags
                .iter()
                .map(|(name, rank)| LibraryMediaTag {
                    name: (*name).to_owned(),
                    rank: *rank,
                })
                .collect(),
            status: Some("FINISHED".to_owned()),
            episodes: Some(12),
            chapters: None,
            volumes: None,
            duration_minutes: Some(24),
            release_date: None,
            cached_at: "2026-07-16T08:00:00.000Z".to_owned(),
            expires_at: "2026-07-17T08:00:00.000Z".to_owned(),
            is_stale: false,
        }
    }

    fn activity(anilist_id: i64, progress_delta: i64, duration: Option<i64>) -> WatchActivity {
        WatchActivity {
            event_id: format!("00000000-0000-4000-a000-{anilist_id:012}"),
            anilist_id,
            media_kind: MediaKind::Anime,
            progress_delta,
            volume_delta: 0,
            episode_duration_minutes: duration,
            occurred_at: "2026-07-16T09:00:00.000Z".to_owned(),
        }
    }

    fn activity_at(
        anilist_id: i64,
        progress_delta: i64,
        duration: Option<i64>,
        occurred_at: &str,
    ) -> WatchActivity {
        WatchActivity {
            occurred_at: occurred_at.to_owned(),
            ..activity(anilist_id, progress_delta, duration)
        }
    }

    #[test]
    fn empty_dashboard_has_explicit_zero_totals_and_collections() {
        let insights = DashboardInsights::build(Vec::new(), Vec::new()).expect("empty insights");

        assert_eq!(insights.media_kind, MediaKind::Anime);
        assert_eq!(insights.library_totals.tracked, 0);
        assert_eq!(insights.library_totals.current_progress, 0);
        assert_eq!(insights.preference_signals.eligible_titles, 0);
        assert_eq!(insights.preference_signals.metadata_titles, 0);
        assert!(insights.preference_signals.genres.is_empty());
        assert!(insights.preference_signals.tags.is_empty());
        assert_eq!(insights.activity.event_count, 0);
        assert_eq!(insights.activity.recorded_episodes, 0);
        assert_eq!(insights.activity.recorded_minutes, 0);
        assert!(insights.activity.events.is_empty());
        assert!(insights.library_items.is_empty());
    }

    #[test]
    fn derives_truthful_library_preferences_and_activity() {
        let mut stale_media = media("Second", &["Drama"], &[("Found Family ", 80), ("Time", 70)]);
        stale_media.is_stale = true;
        let mut manga = item(
            5,
            LibraryStatus::Watching,
            99,
            Some(media("Manga", &["Action"], &[("Battle", 100)])),
        );
        manga.entry.media_kind = MediaKind::Manga;
        let mut manga_activity = activity(5, 99, Some(24));
        manga_activity.media_kind = MediaKind::Manga;
        let insights = DashboardInsights::build(
            vec![
                item(
                    1,
                    LibraryStatus::Watching,
                    3,
                    Some(media(
                        "First",
                        &["Drama", "Fantasy", " Drama "],
                        &[("Found Family", 90), ("Found Family", 85)],
                    )),
                ),
                item(2, LibraryStatus::Completed, 12, Some(stale_media)),
                item(
                    3,
                    LibraryStatus::Planning,
                    0,
                    Some(media("Planned", &["Comedy"], &[("Cats", 99)])),
                ),
                item(4, LibraryStatus::Watching, 1, None),
                manga,
            ],
            vec![
                activity(1, 2, Some(24)),
                activity(99, 3, None),
                manga_activity,
            ],
        )
        .expect("valid insights");

        assert_eq!(insights.library_totals.tracked, 4);
        assert_eq!(insights.library_totals.watching, 2);
        assert_eq!(insights.library_totals.completed, 1);
        assert_eq!(insights.library_totals.planning, 1);
        assert_eq!(insights.library_totals.current_progress, 16);
        assert_eq!(insights.media_kind, MediaKind::Anime);
        assert_eq!(insights.preference_signals.eligible_titles, 3);
        assert_eq!(insights.preference_signals.metadata_titles, 2);
        assert_eq!(insights.preference_signals.missing_metadata_titles, 1);
        assert_eq!(insights.preference_signals.stale_metadata_titles, 1);
        assert_eq!(insights.preference_signals.genres[0].label, "Drama");
        assert_eq!(insights.preference_signals.genres[0].title_count, 2);
        assert!(
            insights
                .preference_signals
                .genres
                .iter()
                .all(|signal| signal.label != "Comedy")
        );
        assert_eq!(insights.preference_signals.tags[0].label, "Found Family");
        assert_eq!(insights.preference_signals.tags[0].title_count, 2);
        assert_eq!(insights.preference_signals.tags[0].weighted_rank, 170);
        assert!(
            insights
                .preference_signals
                .tags
                .iter()
                .all(|signal| signal.label != "Cats")
        );
        assert_eq!(insights.activity.event_count, 2);
        assert_eq!(insights.activity.recorded_episodes, 5);
        assert_eq!(insights.activity.known_duration_episodes, 2);
        assert_eq!(insights.activity.recorded_minutes, 48);
        assert_eq!(insights.activity.unknown_duration_episodes, 3);
        assert_eq!(
            insights.activity.known_duration_episodes + insights.activity.unknown_duration_episodes,
            insights.activity.recorded_episodes
        );
        assert_eq!(insights.activity.events[1].anilist_id, 99);

        let value = serde_json::to_value(insights).expect("serializable insights");
        assert_eq!(value["libraryTotals"]["currentProgress"], 16);
        assert_eq!(value["preferenceSignals"]["tags"][0]["weightedRank"], 170);
        assert_eq!(value["activity"]["unknownDurationEpisodes"], 3);
        assert!(value.get("library_totals").is_none());
    }

    #[test]
    fn rejects_invalid_or_overflowing_activity_without_guessing() {
        let invalid = DashboardInsights::build(Vec::new(), vec![activity(1, 0, Some(24))])
            .expect_err("zero delta must fail");
        assert_eq!(invalid.code, "DASHBOARD_DATA_INVALID");

        let overflow = DashboardInsights::build(Vec::new(), vec![activity(1, i64::MAX, Some(2))])
            .expect_err("minute overflow must fail");
        assert_eq!(overflow.code, "DASHBOARD_DATA_INVALID");
    }

    #[test]
    fn buckets_monthly_and_yearly_activity_with_a_controllable_clock() {
        let now = Utc
            .with_ymd_and_hms(2026, 7, 17, 12, 0, 0)
            .single()
            .expect("test time");
        let insights = DashboardInsights::build_at(
            Vec::new(),
            vec![
                activity_at(1, 2, Some(24), "2026-07-16T09:00:00.000Z"),
                activity_at(2, 1, None, "2026-06-01T09:00:00.000Z"),
                activity_at(3, 3, Some(20), "2025-12-31T23:00:00.000Z"),
            ],
            now,
        )
        .expect("bucketed insights");

        assert_eq!(insights.activity.monthly.len(), 12);
        let july = insights
            .activity
            .monthly
            .iter()
            .find(|bucket| bucket.period == "2026-07")
            .expect("July bucket");
        assert_eq!((july.recorded_episodes, july.recorded_minutes), (2, 48));
        let june = insights
            .activity
            .monthly
            .iter()
            .find(|bucket| bucket.period == "2026-06")
            .expect("June bucket");
        assert_eq!(june.unknown_duration_episodes, 1);
        assert_eq!(insights.activity.yearly.len(), 2);
        assert_eq!(insights.activity.yearly[0].period, "2025");
        assert_eq!(insights.activity.yearly[0].recorded_minutes, 60);
        assert_eq!(insights.activity.yearly[1].period, "2026");
        assert_eq!(insights.activity.yearly[1].recorded_minutes, 48);
        assert_eq!(
            insights.activity.covered_from.as_deref(),
            Some("2025-12-31T23:00:00.000Z")
        );
        assert_eq!(
            insights.activity.covered_to.as_deref(),
            Some("2026-07-16T09:00:00.000Z")
        );
    }
}
