use std::collections::{BTreeMap, BTreeSet, HashSet};

use serde::Serialize;

use crate::anilist::MediaSummary;
use crate::domain::{LibraryItem, LibraryStatus};

pub const MINIMUM_PROFILE_TITLES: usize = 2;
pub const MAX_EXPLANATION_LABELS: usize = 3;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RecommendationWeights {
    pub tags: f64,
    pub genres: f64,
    pub trending: f64,
}

impl RecommendationWeights {
    pub const V1: Self = Self {
        tags: 0.55,
        genres: 0.25,
        trending: 0.20,
    };

    pub fn total(self) -> f64 {
        self.tags + self.genres + self.trending
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Recommendation {
    pub media: MediaSummary,
    pub score: f64,
    pub explanation_labels: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RecommendationSet {
    pub items: Vec<Recommendation>,
    pub is_fallback: bool,
    pub eligible_profile_titles: usize,
}

#[derive(Debug, Default)]
struct PreferenceProfile {
    eligible_titles: usize,
    genres: BTreeSet<String>,
    tags: BTreeMap<String, f64>,
}

pub fn rank_recommendations(
    library: &[LibraryItem],
    candidates: Vec<MediaSummary>,
) -> RecommendationSet {
    let profile = build_profile(library);
    let tracked = library
        .iter()
        .map(|item| item.entry.anilist_id)
        .collect::<HashSet<_>>();
    let mut candidates = candidates
        .into_iter()
        .filter(|candidate| !tracked.contains(&candidate.anilist_id))
        .collect::<Vec<_>>();
    let has_signals = !profile.genres.is_empty() || !profile.tags.is_empty();
    let is_fallback = profile.eligible_titles < MINIMUM_PROFILE_TITLES || !has_signals;

    if is_fallback {
        candidates.sort_by(|left, right| {
            right
                .trending
                .cmp(&left.trending)
                .then_with(|| left.anilist_id.cmp(&right.anilist_id))
        });
        return RecommendationSet {
            items: candidates
                .into_iter()
                .map(|media| Recommendation {
                    media,
                    score: 0.0,
                    explanation_labels: Vec::new(),
                })
                .collect(),
            is_fallback: true,
            eligible_profile_titles: profile.eligible_titles,
        };
    }

    let (minimum_trending, maximum_trending) =
        candidates
            .iter()
            .fold((i64::MAX, i64::MIN), |(minimum, maximum), candidate| {
                (
                    minimum.min(candidate.trending),
                    maximum.max(candidate.trending),
                )
            });
    let mut items = candidates
        .into_iter()
        .map(|media| {
            let tag_similarity = tag_similarity(&profile.tags, &media);
            let genre_similarity = genre_similarity(&profile.genres, &media);
            let trending = normalize_trending(media.trending, minimum_trending, maximum_trending);
            let score = RecommendationWeights::V1.tags * tag_similarity
                + RecommendationWeights::V1.genres * genre_similarity
                + RecommendationWeights::V1.trending * trending;
            let explanation_labels = explanation_labels(&profile.tags, &media);
            Recommendation {
                media,
                score: round_score(score),
                explanation_labels,
            }
        })
        .collect::<Vec<_>>();
    items.sort_by(|left, right| {
        right
            .score
            .total_cmp(&left.score)
            .then_with(|| right.media.trending.cmp(&left.media.trending))
            .then_with(|| left.media.anilist_id.cmp(&right.media.anilist_id))
    });
    RecommendationSet {
        items,
        is_fallback: false,
        eligible_profile_titles: profile.eligible_titles,
    }
}

fn build_profile(library: &[LibraryItem]) -> PreferenceProfile {
    let mut profile = PreferenceProfile::default();
    for item in library.iter().filter(|item| {
        matches!(
            item.entry.status,
            LibraryStatus::Watching | LibraryStatus::Completed
        )
    }) {
        let Some(media) = &item.media else {
            continue;
        };
        profile.eligible_titles += 1;
        profile.genres.extend(
            media
                .genres
                .iter()
                .map(|genre| genre.trim())
                .filter(|genre| !genre.is_empty())
                .map(canonical_label),
        );
        let mut strongest_for_title = BTreeMap::<String, f64>::new();
        for tag in &media.tags {
            let label = tag.name.trim();
            if label.is_empty() || tag.rank <= 0 {
                continue;
            }
            strongest_for_title
                .entry(canonical_label(label))
                .and_modify(|rank| *rank = rank.max(tag.rank as f64))
                .or_insert(tag.rank as f64);
        }
        for (label, rank) in strongest_for_title {
            *profile.tags.entry(label).or_default() += rank;
        }
    }
    profile
}

fn tag_similarity(profile: &BTreeMap<String, f64>, candidate: &MediaSummary) -> f64 {
    let candidate_tags = candidate
        .tags
        .iter()
        .filter(|tag| !tag.name.trim().is_empty() && tag.rank > 0)
        .map(|tag| (canonical_label(&tag.name), f64::from(tag.rank)))
        .collect::<BTreeMap<_, _>>();
    let dot_product = candidate_tags
        .iter()
        .map(|(label, rank)| profile.get(label).copied().unwrap_or_default() * rank)
        .sum::<f64>();
    let profile_norm = profile.values().map(|rank| rank * rank).sum::<f64>().sqrt();
    let candidate_norm = candidate_tags
        .values()
        .map(|rank| rank * rank)
        .sum::<f64>()
        .sqrt();
    if profile_norm == 0.0 || candidate_norm == 0.0 {
        0.0
    } else {
        (dot_product / (profile_norm * candidate_norm)).clamp(0.0, 1.0)
    }
}

fn genre_similarity(profile: &BTreeSet<String>, candidate: &MediaSummary) -> f64 {
    let candidate = candidate
        .genres
        .iter()
        .map(|genre| genre.trim())
        .filter(|genre| !genre.is_empty())
        .map(canonical_label)
        .collect::<BTreeSet<_>>();
    let union = profile.union(&candidate).count();
    if union == 0 {
        0.0
    } else {
        profile.intersection(&candidate).count() as f64 / union as f64
    }
}

fn normalize_trending(value: i64, minimum: i64, maximum: i64) -> f64 {
    if minimum == i64::MAX || maximum == i64::MIN {
        return 0.0;
    }
    if minimum == maximum {
        return 1.0;
    }
    (value - minimum) as f64 / (maximum - minimum) as f64
}

fn explanation_labels(profile: &BTreeMap<String, f64>, candidate: &MediaSummary) -> Vec<String> {
    let mut matches = candidate
        .tags
        .iter()
        .filter_map(|tag| {
            let key = canonical_label(&tag.name);
            profile.get(&key).map(|profile_rank| {
                (
                    tag.name.trim().to_owned(),
                    profile_rank * f64::from(tag.rank),
                )
            })
        })
        .collect::<Vec<_>>();
    matches.sort_by(|(left_label, left_score), (right_label, right_score)| {
        right_score
            .total_cmp(left_score)
            .then_with(|| left_label.cmp(right_label))
    });
    matches
        .into_iter()
        .map(|(label, _)| label)
        .take(MAX_EXPLANATION_LABELS)
        .collect()
}

fn canonical_label(label: &str) -> String {
    label.trim().to_lowercase()
}

fn round_score(score: f64) -> f64 {
    (score.clamp(0.0, 1.0) * 10_000.0).round() / 10_000.0
}

#[cfg(test)]
mod tests {
    use super::{MINIMUM_PROFILE_TITLES, RecommendationWeights, rank_recommendations};
    use crate::anilist::{MediaSummary, MediaTag};
    use crate::domain::{
        LibraryEntry, LibraryItem, LibraryMediaSnapshot, LibraryMediaTag, LibraryStatus, MediaKind,
    };

    fn media(
        id: i64,
        title: &str,
        genres: &[&str],
        tags: &[(&str, u8)],
        trending: i64,
    ) -> MediaSummary {
        MediaSummary {
            anilist_id: id,
            title: title.to_owned(),
            cover_url: None,
            media_format: Some("TV".to_owned()),
            status: Some("RELEASING".to_owned()),
            episodes: Some(12),
            duration_minutes: Some(24),
            genres: genres.iter().map(|value| (*value).to_owned()).collect(),
            tags: tags
                .iter()
                .map(|(name, rank)| MediaTag {
                    name: (*name).to_owned(),
                    rank: *rank,
                })
                .collect(),
            trending,
        }
    }

    fn library(
        id: i64,
        status: LibraryStatus,
        genres: &[&str],
        tags: &[(&str, i64)],
    ) -> LibraryItem {
        LibraryItem {
            entry: LibraryEntry {
                anilist_id: id,
                media_kind: MediaKind::Anime,
                status,
                progress: 1,
                score: None,
                notes: None,
                created_at: "2026-07-17T08:00:00.000Z".to_owned(),
                updated_at: "2026-07-17T08:00:00.000Z".to_owned(),
            },
            media: Some(LibraryMediaSnapshot {
                title: format!("Tracked {id}"),
                cover_url: None,
                media_format: Some("TV".to_owned()),
                genres: genres.iter().map(|value| (*value).to_owned()).collect(),
                tags: tags
                    .iter()
                    .map(|(name, rank)| LibraryMediaTag {
                        name: (*name).to_owned(),
                        rank: *rank,
                    })
                    .collect(),
                status: Some("FINISHED".to_owned()),
                episodes: Some(12),
                duration_minutes: Some(24),
                cached_at: "2026-07-17T08:00:00.000Z".to_owned(),
                expires_at: "2026-07-18T08:00:00.000Z".to_owned(),
                is_stale: false,
            }),
        }
    }

    #[test]
    fn project_weights_form_a_complete_score() {
        assert!((RecommendationWeights::V1.total() - 1.0).abs() < f64::EPSILON);
        assert_eq!(MINIMUM_PROFILE_TITLES, 2);
    }

    #[test]
    fn ranks_with_fixed_weights_explanations_and_deterministic_ties() {
        let library = vec![
            library(
                1,
                LibraryStatus::Watching,
                &["Drama"],
                &[("Found Family", 90), ("Time", 70)],
            ),
            library(
                2,
                LibraryStatus::Completed,
                &["Fantasy"],
                &[("Found Family", 80)],
            ),
        ];
        let ranked = rank_recommendations(
            &library,
            vec![
                media(20, "Strong match", &["Drama"], &[("Found Family", 95)], 20),
                media(10, "Trending only", &["Comedy"], &[("Cats", 100)], 100),
                media(21, "Same match", &["Drama"], &[("Found Family", 95)], 20),
            ],
        );
        assert!(!ranked.is_fallback);
        assert_eq!(ranked.items[0].media.anilist_id, 20);
        assert_eq!(ranked.items[1].media.anilist_id, 21);
        assert_eq!(ranked.items[0].explanation_labels, vec!["Found Family"]);
        assert!(ranked.items[0].score > ranked.items[2].score);
        assert!((0.0..=1.0).contains(&ranked.items[0].score));
    }

    #[test]
    fn excludes_tracked_titles_and_falls_back_to_stable_trending_order() {
        let library = vec![library(
            1,
            LibraryStatus::Watching,
            &["Drama"],
            &[("Time", 80)],
        )];
        let ranked = rank_recommendations(
            &library,
            vec![
                media(1, "Tracked", &["Drama"], &[("Time", 80)], 999),
                media(3, "Tie later", &[], &[], 50),
                media(2, "Tie first", &[], &[], 50),
            ],
        );
        assert!(ranked.is_fallback);
        assert_eq!(ranked.eligible_profile_titles, 1);
        assert_eq!(
            ranked
                .items
                .iter()
                .map(|item| item.media.anilist_id)
                .collect::<Vec<_>>(),
            vec![2, 3]
        );
        assert!(
            ranked
                .items
                .iter()
                .all(|item| item.score == 0.0 && item.explanation_labels.is_empty())
        );
    }

    #[test]
    fn planning_and_missing_metadata_do_not_fabricate_a_profile() {
        let mut missing = library(2, LibraryStatus::Completed, &["Drama"], &[("Time", 80)]);
        missing.media = None;
        let ranked = rank_recommendations(
            &[
                library(1, LibraryStatus::Planning, &["Drama"], &[("Time", 80)]),
                missing,
            ],
            vec![media(3, "Candidate", &["Drama"], &[("Time", 80)], 10)],
        );
        assert!(ranked.is_fallback);
        assert_eq!(ranked.eligible_profile_titles, 0);
    }

    #[test]
    fn equal_trending_values_have_a_defined_normalized_score() {
        let library = vec![
            library(1, LibraryStatus::Watching, &["Drama"], &[("Time", 90)]),
            library(2, LibraryStatus::Completed, &["Drama"], &[("Time", 80)]),
        ];
        let ranked = rank_recommendations(&library, vec![media(3, "Candidate", &[], &[], 10)]);
        assert_eq!(ranked.items[0].score, 0.2);
    }
}
