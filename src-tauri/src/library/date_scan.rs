//! Reads every video's recording date once, in the background, so "date created" is the day a
//! video was filmed rather than the day a copy of it was made or uploaded.

use std::sync::atomic::{AtomicBool, Ordering};

use futures_util::{StreamExt, stream};
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};

use super::video_summary::VideoFile;
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

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecordedDateEvent {
    pub drive_id: String,
    pub recorded_date: String,
}

#[derive(Default)]
pub struct DateScan {
    running: AtomicBool,
}

impl DateScan {
    /// Reads the dates not read yet, unless a scan is already running.
    pub fn start(&self, app: &AppHandle) {
        if self.running.swap(true, Ordering::SeqCst) {
            return;
        }
        let app = app.clone();
        tauri::async_runtime::spawn(async move {
            let state = app.state::<AppState>();
            if let Err(error) = run(&state, &app).await {
                log::warn!("Stopped reading recording dates: {error}");
            }
            state.date_scan.running.store(false, Ordering::SeqCst);
        });
    }
}

async fn run(state: &AppState, app: &AppHandle) -> AppResult<()> {
    let Some(ffmpeg) = ffmpeg::locate_ffmpeg() else {
        log::warn!("The ffmpeg sidecar is missing, so recording dates cannot be read");
        return Ok(());
    };
    let files = state.drive.list_all_videos(None).await?;
    // Also registers every video with the media proxy, which only streams listed files.
    let videos: Vec<VideoFile> = state
        .summarize(files)
        .await?
        .into_iter()
        .map(|summary| summary.file)
        .collect();
    let ids: Vec<String> = videos.iter().map(|video| video.id.clone()).collect();
    let records = state
        .db
        .with(|connection| recorded_dates::records(connection, &ids))?;
    let pending: Vec<VideoFile> = videos
        .into_iter()
        .filter(|video| needs_reading(video, records.get(&video.id)))
        .collect();
    if pending.is_empty() {
        return Ok(());
    }
    log::info!("Reading the recording dates of {} videos", pending.len());

    stream::iter(pending)
        .for_each_concurrent(CONCURRENCY, |video| {
            let ffmpeg = &ffmpeg;
            async move {
                let info = ffmpeg::probe(ffmpeg, &state.media_proxy.media_url(&video.id)).await;
                // Without a duration ffmpeg could not read the file at all, so it is tried again
                // next time instead of being stored as having no date.
                if info.duration_seconds.is_none() {
                    return;
                }
                let date = info.recorded_date;
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
                        drive_id: video.id,
                        recorded_date,
                    };
                    if let Err(error) = app.emit(ITEM_EVENT, event) {
                        log::warn!("Could not send a recording date: {error}");
                    }
                }
            }
        })
        .await;
    if let Err(error) = app.emit(DONE_EVENT, ()) {
        log::warn!("Could not send the end of the recording-date scan: {error}");
    }
    Ok(())
}

/// A date in the file name wins over the metadata (see `recorded_date`), so those files are
/// never read.
fn needs_reading(video: &VideoFile, record: Option<&RecordedDateRecord>) -> bool {
    recorded_date::from_file_name(&video.file_name).is_none()
        && recorded_dates::needs_check(record, video.md5_checksum.as_deref())
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
}
