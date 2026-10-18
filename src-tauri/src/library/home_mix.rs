//! Home's mix: every video in the folder (or Drive) in a seeded, weighted random order, so Home
//! opens on different videos each time instead of the newest ones. The same seed always gives
//! the same order, so paging through a mix (or listing it again) never reshuffles it.

use std::collections::{HashSet, VecDeque};

use serde::Deserialize;

use super::date_order;
use super::video_summary::VideoSummary;
use crate::db::user_state::MIN_RESUME_MS;

/// Watched videos come up less often, and ones watched lately much less.
const WATCHED_WEIGHT: f64 = 0.3;
const RECENTLY_WATCHED_WEIGHT: f64 = 0.1;
const RECENTLY_WATCHED_DAYS: i64 = 7;
/// Uploads this new get a place in the first row, so they show up soon after being added.
const FRESH_UPLOAD_DAYS: i64 = 30;
const FRESH_SLOTS: [usize; 2] = [1, 3];
/// How far ahead the spreading looks for a video from another folder or day.
const SPREAD_LOOKAHEAD: usize = 8;

/// The seed and the user's local date, `YYYY-MM-DD`. The date comes from the UI because only it
/// knows the time zone.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Mix {
    pub seed: u32,
    pub today: String,
}

/// The mixed order. Videos in progress are left out when `skip_in_progress`, as Home's Continue
/// watching shelf already shows them.
pub fn arrange(
    mut videos: Vec<VideoSummary>,
    mix: &Mix,
    skip_in_progress: bool,
) -> Vec<VideoSummary> {
    if skip_in_progress {
        videos.retain(|video| !is_in_progress(video));
    }
    let today = Day::parse(&mix.today).map(Day::number);
    let mut keyed: Vec<(f64, VideoSummary)> = videos
        .into_iter()
        .map(|video| {
            // Weighted random sampling (Efraimidis–Spirakis): the highest keys come first.
            let key = unit_random(mix.seed, &video.file.id).ln() / weight(&video, today);
            (key, video)
        })
        .collect();
    keyed.sort_by(|a, b| b.0.total_cmp(&a.0));
    let mut arranged = spread(keyed.into_iter().map(|(_, video)| video).collect());
    if let Some(today) = today {
        promote_fresh_uploads(&mut arranged, today);
    }
    arranged
}

/// Videos recorded on today's month and day in an earlier year, newest first. Videos from
/// February 29 come up on February 28 in other years.
pub fn on_this_day(videos: &[VideoSummary], today: &str) -> Vec<VideoSummary> {
    let Some(today) = Day::parse(today) else {
        return Vec::new();
    };
    let mut matches: Vec<VideoSummary> = videos
        .iter()
        .filter(|video| {
            video
                .date_created()
                .and_then(Day::parse)
                .is_some_and(|day| {
                    day.year < today.year
                        && day.anniversary_in(today.year) == (today.month, today.day)
                })
        })
        .cloned()
        .collect();
    date_order::sort_by_date_created(&mut matches, true);
    matches
}

/// The rule of Continue watching (`user_state::list_ids`).
fn is_in_progress(video: &VideoSummary) -> bool {
    !video.watched
        && video.last_watched_at.is_some()
        && video
            .last_position_ms
            .is_some_and(|position| position >= MIN_RESUME_MS as u64)
}

fn weight(video: &VideoSummary, today: Option<i64>) -> f64 {
    let watched_days_ago = today
        .zip(day_number(video.last_watched_at.as_deref()))
        .map(|(today, watched)| today - watched);
    if watched_days_ago.is_some_and(|days| days <= RECENTLY_WATCHED_DAYS) {
        RECENTLY_WATCHED_WEIGHT
    } else if video.watched {
        WATCHED_WEIGHT
    } else {
        1.0
    }
}

/// Moves a video away from the one before it when it is from the same folder (if the mix spans
/// several) or the same day, taking the next one within reach that is not.
fn spread(videos: Vec<VideoSummary>) -> Vec<VideoSummary> {
    let folders: HashSet<Option<&str>> = videos
        .iter()
        .map(|video| video.file.folder_id.as_deref())
        .collect();
    let spread_folders = folders.len() > 1;
    let too_close = |a: &VideoSummary, b: &VideoSummary| {
        (spread_folders && a.file.folder_id == b.file.folder_id)
            || (a.date_created().is_some() && a.date_created() == b.date_created())
    };

    let mut pending: VecDeque<VideoSummary> = videos.into();
    let mut arranged = Vec::with_capacity(pending.len());
    while !pending.is_empty() {
        let pick = arranged
            .last()
            .and_then(|previous| {
                pending
                    .iter()
                    .take(SPREAD_LOOKAHEAD)
                    .position(|candidate| !too_close(previous, candidate))
            })
            .unwrap_or(0);
        if let Some(video) = pending.remove(pick) {
            arranged.push(video);
        }
    }
    arranged
}

/// Puts the first recent uploads of the mix into the first row's `FRESH_SLOTS`, unless they
/// already come earlier.
fn promote_fresh_uploads(videos: &mut Vec<VideoSummary>, today: i64) {
    let is_fresh = |video: &VideoSummary| {
        day_number(video.file.created_time.as_deref())
            .is_some_and(|uploaded| today - uploaded <= FRESH_UPLOAD_DAYS)
    };
    let mut start = 0;
    for slot in FRESH_SLOTS {
        let Some(index) = (start..videos.len()).find(|&index| is_fresh(&videos[index])) else {
            return;
        };
        let placed = if index > slot {
            let video = videos.remove(index);
            videos.insert(slot, video);
            slot
        } else {
            index
        };
        start = placed + 1;
    }
}

/// A random number in (0, 1] for this video under this seed. It depends only on the two, so a
/// video keeps its place in a mix when other videos are added or listed in another order.
fn unit_random(seed: u32, id: &str) -> f64 {
    let bits = splitmix64(fnv1a(id) ^ splitmix64(u64::from(seed)));
    ((bits >> 11) as f64 + 1.0) / (1u64 << 53) as f64
}

fn fnv1a(text: &str) -> u64 {
    text.bytes().fold(0xCBF2_9CE4_8422_2325, |hash, byte| {
        (hash ^ u64::from(byte)).wrapping_mul(0x0000_0100_0000_01B3)
    })
}

fn splitmix64(value: u64) -> u64 {
    let mut z = value.wrapping_add(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// The day number of a `YYYY-MM-DD` date or of a timestamp that starts with one.
fn day_number(text: Option<&str>) -> Option<i64> {
    text.and_then(Day::parse).map(Day::number)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Day {
    year: i64,
    month: u32,
    day: u32,
}

impl Day {
    fn parse(text: &str) -> Option<Self> {
        let mut parts = text.get(..10)?.split('-');
        let year = parts.next()?.parse().ok()?;
        let month = parts.next()?.parse().ok()?;
        let day = parts.next()?.parse().ok()?;
        ((1..=12).contains(&month) && (1..=31).contains(&day)).then_some(Self { year, month, day })
    }

    /// Days since 1970-01-01 (Howard Hinnant's `days_from_civil`).
    fn number(self) -> i64 {
        let month = i64::from(self.month);
        let year = if month <= 2 { self.year - 1 } else { self.year };
        let era = year.div_euclid(400);
        let year_of_era = year - era * 400;
        let day_of_year = (153 * ((month + 9) % 12) + 2) / 5 + i64::from(self.day) - 1;
        let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
        era * 146_097 + day_of_era - 719_468
    }

    /// This day's month and day in `year`.
    fn anniversary_in(self, year: i64) -> (u32, u32) {
        if (self.month, self.day) == (2, 29) && !is_leap_year(year) {
            (2, 28)
        } else {
            (self.month, self.day)
        }
    }
}

fn is_leap_year(year: i64) -> bool {
    year % 4 == 0 && (year % 100 != 0 || year % 400 == 0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::test_support::video;

    const TODAY: &str = "2026-10-18";

    fn mix(seed: u32) -> Mix {
        Mix {
            seed,
            today: TODAY.to_owned(),
        }
    }

    /// Videos `v0`, `v1`, … each in its own folder and on its own day, so spreading leaves the
    /// shuffled order alone.
    fn distinct_videos(count: usize) -> Vec<VideoSummary> {
        (0..count)
            .map(|index| {
                let mut file = video(&format!("v{index}"), &format!("v{index}.mp4"));
                file.folder_id = Some(format!("folder{index}"));
                let recorded = format!("2020-01-{:02}", index % 28 + 1);
                VideoSummary::new(file, None, Some(recorded))
            })
            .collect()
    }

    fn ids(videos: &[VideoSummary]) -> Vec<&str> {
        videos.iter().map(|video| video.file.id.as_str()).collect()
    }

    fn recorded_on(id: &str, recorded: &str) -> VideoSummary {
        VideoSummary::new(
            video(id, &format!("{id}.mp4")),
            None,
            Some(recorded.to_owned()),
        )
    }

    #[test]
    fn the_same_seed_gives_the_same_order_and_another_seed_a_different_one() {
        let first = arrange(distinct_videos(20), &mix(1), false);
        let again = arrange(distinct_videos(20), &mix(1), false);
        let other = arrange(distinct_videos(20), &mix(2), false);

        assert_eq!(ids(&first), ids(&again));
        assert_ne!(ids(&first), ids(&other));
    }

    #[test]
    fn a_video_keeps_its_place_whatever_order_the_videos_are_listed_in() {
        let mut reversed = distinct_videos(20);
        reversed.reverse();

        let listed = arrange(distinct_videos(20), &mix(7), false);
        let listed_reversed = arrange(reversed, &mix(7), false);

        assert_eq!(ids(&listed), ids(&listed_reversed));
    }

    #[test]
    fn watched_and_recently_watched_videos_come_up_less_often() {
        let top_three_counts = |change: fn(&mut VideoSummary)| {
            (0..200)
                .filter(|&seed| {
                    let mut videos = distinct_videos(10);
                    change(&mut videos[0]);
                    arrange(videos, &mix(seed), false)
                        .iter()
                        .take(3)
                        .any(|video| video.file.id == "v0")
                })
                .count()
        };

        let unwatched = top_three_counts(|_| {});
        let watched = top_three_counts(|video| {
            video.watched = true;
            video.last_watched_at = Some("2025-01-01T10:00:00.000Z".into());
        });
        let recently_watched = top_three_counts(|video| {
            video.watched = true;
            video.last_watched_at = Some("2026-10-16T10:00:00.000Z".into());
        });

        assert!(
            watched < unwatched / 2,
            "watched {watched}, unwatched {unwatched}"
        );
        assert!(
            recently_watched < watched,
            "recently watched {recently_watched}, watched {watched}"
        );
    }

    #[test]
    fn videos_in_progress_are_left_out_only_when_asked() {
        let mut videos = distinct_videos(3);
        videos[1].last_watched_at = Some("2026-10-17T10:00:00.000Z".into());
        videos[1].last_position_ms = Some(30_000);

        let skipped = arrange(videos.clone(), &mix(3), true);
        let kept = arrange(videos, &mix(3), false);

        assert!(!ids(&skipped).contains(&"v1"));
        assert_eq!(skipped.len(), 2);
        assert!(ids(&kept).contains(&"v1"));
    }

    #[test]
    fn neighbours_come_from_different_folders_when_possible() {
        let videos: Vec<VideoSummary> = (0..12)
            .map(|index| {
                let mut file = video(&format!("v{index}"), &format!("v{index}.mp4"));
                file.folder_id = Some(if index < 6 { "a" } else { "b" }.to_owned());
                VideoSummary::new(file, None, Some(format!("2020-02-{:02}", index + 1)))
            })
            .collect();

        for seed in 0..20 {
            let arranged = arrange(videos.clone(), &mix(seed), false);
            for pair in arranged.windows(2) {
                assert_ne!(
                    pair[0].file.folder_id, pair[1].file.folder_id,
                    "seed {seed}"
                );
            }
        }
    }

    #[test]
    fn neighbours_in_one_folder_come_from_different_days_when_possible() {
        let videos: Vec<VideoSummary> = (0..8)
            .map(|index| {
                let day = if index % 2 == 0 {
                    "2021-03-01"
                } else {
                    "2021-03-02"
                };
                recorded_on(&format!("v{index}"), day)
            })
            .collect();

        for seed in 0..20 {
            let arranged = arrange(videos.clone(), &mix(seed), false);
            for pair in arranged.windows(2) {
                assert_ne!(
                    pair[0].date_created(),
                    pair[1].date_created(),
                    "seed {seed}"
                );
            }
        }
    }

    #[test]
    fn recent_uploads_get_the_first_rows_fresh_slots() {
        for seed in 0..20 {
            let mut videos = distinct_videos(30);
            videos[17].file.created_time = Some("2026-10-10T08:00:00.000Z".into());
            videos[23].file.created_time = Some("2026-09-25T08:00:00.000Z".into());
            videos[5].file.created_time = Some("2026-01-01T08:00:00.000Z".into());

            let arranged = arrange(videos, &mix(seed), false);
            let fresh: Vec<usize> = arranged
                .iter()
                .enumerate()
                .filter(|(_, video)| ["v17", "v23"].contains(&video.file.id.as_str()))
                .map(|(index, _)| index)
                .collect();

            assert!(fresh[0] <= 1 && fresh[1] <= 3, "seed {seed}: {fresh:?}");
        }
    }

    #[test]
    fn on_this_day_finds_this_day_in_earlier_years_newest_first() {
        let videos = vec![
            recorded_on("older", "2019-10-18"),
            recorded_on("newer", "2024-10-18"),
            recorded_on("this-year", "2026-10-18"),
            recorded_on("other-day", "2024-10-19"),
            VideoSummary::new(video("undated", "undated.mp4"), None, None),
        ];

        assert_eq!(ids(&on_this_day(&videos, TODAY)), ["newer", "older"]);
    }

    #[test]
    fn on_this_day_shows_leap_day_videos_on_february_28_in_other_years() {
        let videos = vec![recorded_on("leap", "2024-02-29")];

        assert_eq!(ids(&on_this_day(&videos, "2026-02-28")), ["leap"]);
        assert_eq!(ids(&on_this_day(&videos, "2028-02-29")), ["leap"]);
        assert!(on_this_day(&videos, "2028-02-28").is_empty());
    }

    #[test]
    fn day_numbers_count_days_across_months_and_leap_years() {
        let number = |text: &str| Day::parse(text).unwrap().number();

        assert_eq!(number("1970-01-01"), 0);
        assert_eq!(number("2024-03-01") - number("2024-02-28"), 2);
        assert_eq!(number("2023-03-01") - number("2023-02-28"), 1);
        assert_eq!(number("2026-10-18") - number("2025-10-18"), 365);
        assert_eq!(
            day_number(Some("2026-10-18T23:59:59.000Z")),
            Some(number("2026-10-18"))
        );
        assert_eq!(Day::parse("not a date"), None);
    }
}
