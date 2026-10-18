//! Reads every video's recording date once, in the background, so "date created" is the day a
//! video was filmed rather than the day a copy of it was made or uploaded. For videos on this
//! computer the same read also gives the duration and picture size, which Drive reports for its
//! own files.

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};

use futures_util::{StreamExt, stream};
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};

use super::video_summary::{VideoFile, VideoSource};
use crate::db::media_info::{self, MediaInfoRecord};
use crate::db::recorded_dates::{self, RecordedDateRecord};
use crate::error::AppResult;
use crate::ffmpeg;
use crate::recorded_date;
use crate::state::AppState;

/// ffmpeg reads only each file's header through the media proxy, but each read still takes
/// 1-8 s. Six at a time measured fine alongside playback and title generation.
const CONCURRENCY: usize = 6;

pub const ITEM_EVENT: &str = "recorded-dates://item";
/// Lists sorted by date created before the scan placed videos by their fallback date.
pub const DONE_EVENT: &str = "recorded-dates://done";
pub const MEDIA_INFO_EVENT: &str = "media-info://item";

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecordedDateEvent {
    pub drive_id: String,
    pub recorded_date: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaInfoEvent {
    pub drive_id: String,
    pub duration_ms: Option<u64>,
    pub width: Option<u32>,
    pub height: Option<u32>,
}

#[derive(Default)]
pub struct DateScan {
    running: AtomicBool,
    /// Set when the library changed during a scan, so another follows it.
    again: AtomicBool,
}

impl DateScan {
    /// Reads what has not been read yet, or scans again after the current scan finishes.
    pub fn start(&self, app: &AppHandle) {
        if self.running.swap(true, Ordering::SeqCst) {
            self.again.store(true, Ordering::SeqCst);
            return;
        }
        let app = app.clone();
        tauri::async_runtime::spawn(async move {
            let state = app.state::<AppState>();
            loop {
                state.date_scan.again.store(false, Ordering::SeqCst);
                if let Err(error) = run(&state, &app).await {
                    log::warn!("Stopped reading recording dates: {error}");
                }
                if !state.date_scan.again.load(Ordering::SeqCst) {
                    break;
                }
            }
            state.date_scan.running.store(false, Ordering::SeqCst);
        });
    }
}

/// What one video still needs read.
struct Pending {
    video: VideoFile,
    date: bool,
    media_info: bool,
}

async fn run(state: &AppState, app: &AppHandle) -> AppResult<()> {
    let Some(ffmpeg) = ffmpeg::locate_ffmpeg() else {
        log::warn!("The ffmpeg sidecar is missing, so recording dates cannot be read");
        return Ok(());
    };
    // Also registers every video with the media proxy, which only streams listed files.
    let videos = state.library_videos(None).await?;
    let ids: Vec<String> = videos.iter().map(|video| video.id.clone()).collect();
    let (dates, media) = state.db.with(|connection| {
        Ok((
            recorded_dates::records(connection, &ids)?,
            media_info::records(connection, &ids)?,
        ))
    })?;
    let mut pending = pending_reads(videos, &dates, &media);
    if pending.is_empty() {
        return Ok(());
    }
    // Local videos show no duration until they are read, so they go first.
    pending.sort_by_key(|item| !item.media_info);
    log::info!("Reading the details of {} videos", pending.len());

    stream::iter(pending)
        .for_each_concurrent(CONCURRENCY, |item| {
            let ffmpeg = &ffmpeg;
            async move {
                let Ok(input) = state.media_proxy.ffmpeg_input(&item.video.id).await else {
                    return;
                };
                let info = ffmpeg::probe(ffmpeg, &input).await;
                // Without a duration ffmpeg could not read the file at all, so it is tried again
                // next time instead of being stored as having no date.
                if info.duration_seconds.is_none() {
                    return;
                }
                if item.media_info {
                    save_media_info(state, app, &item.video, &info);
                }
                if item.date {
                    save_date(state, app, &item.video, info.recorded_date);
                }
            }
        })
        .await;
    if let Err(error) = app.emit(DONE_EVENT, ()) {
        log::warn!("Could not send the end of the recording-date scan: {error}");
    }
    Ok(())
}

fn pending_reads(
    videos: Vec<VideoFile>,
    dates: &HashMap<String, RecordedDateRecord>,
    media: &HashMap<String, MediaInfoRecord>,
) -> Vec<Pending> {
    videos
        .into_iter()
        .filter_map(|video| {
            let date = needs_reading(&video, dates.get(&video.id));
            let media_info = video.source == VideoSource::Local && !media.contains_key(&video.id);
            (date || media_info).then_some(Pending {
                video,
                date,
                media_info,
            })
        })
        .collect()
}

/// A date in the file name wins over the metadata (see `recorded_date`), so those files are
/// never read.
fn needs_reading(video: &VideoFile, record: Option<&RecordedDateRecord>) -> bool {
    recorded_date::from_file_name(&video.file_name).is_none()
        && recorded_dates::needs_check(record, video.md5_checksum.as_deref())
}

fn save_media_info(state: &AppState, app: &AppHandle, video: &VideoFile, info: &ffmpeg::MediaInfo) {
    let record = MediaInfoRecord {
        duration_ms: info
            .duration_seconds
            .map(|seconds| (seconds * 1000.0) as u64),
        width: info.width,
        height: info.height,
    };
    if let Err(error) = state
        .db
        .with(|connection| media_info::save(connection, &video.id, &record))
    {
        log::warn!("Could not store a video's duration and size: {error}");
        return;
    }
    let event = MediaInfoEvent {
        drive_id: video.id.clone(),
        duration_ms: record.duration_ms,
        width: record.width,
        height: record.height,
    };
    if let Err(error) = app.emit(MEDIA_INFO_EVENT, event) {
        log::warn!("Could not send a video's duration and size: {error}");
    }
}

fn save_date(state: &AppState, app: &AppHandle, video: &VideoFile, date: Option<String>) {
    if let Err(error) = state.db.with(|connection| {
        recorded_dates::save(
            connection,
            &video.id,
            date.as_deref(),
            video.md5_checksum.as_deref(),
        )
    }) {
        log::warn!("Could not store a recording date: {error}");
        return;
    }
    if let Some(recorded_date) = date {
        let event = RecordedDateEvent {
            drive_id: video.id.clone(),
            recorded_date,
        };
        if let Err(error) = app.emit(ITEM_EVENT, event) {
            log::warn!("Could not send a recording date: {error}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::test_support::video;

    #[test]
    fn reads_unread_files_without_a_date_in_their_name() {
        let read = RecordedDateRecord {
            recorded_date: None,
            md5_checksum: Some("md5".into()),
        };

        assert!(needs_reading(&video("a", "IMG_3344.MOV"), None));
        assert!(!needs_reading(&video("a", "IMG_3344.MOV"), Some(&read)));
        assert!(!needs_reading(&video("b", "08132019_224238.m4v"), None));
    }

    #[test]
    fn reads_the_size_of_local_videos_even_when_the_name_has_a_date() {
        let mut local = video("local-1", "08132019_224238.m4v");
        local.source = VideoSource::Local;
        let mut measured = video("local-2", "08132019_224239.m4v");
        measured.source = VideoSource::Local;
        let drive = video("d", "08132019_224240.m4v");
        let media = HashMap::from([("local-2".to_owned(), MediaInfoRecord::default())]);

        let pending = pending_reads(vec![local, measured, drive], &HashMap::new(), &media);

        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].video.id, "local-1");
        assert!(pending[0].media_info && !pending[0].date);
    }
}
