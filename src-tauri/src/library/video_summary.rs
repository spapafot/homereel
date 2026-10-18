use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use super::folder_names::SHARED_WITH_ME;
use super::local_files::{self, LocalVideo};
use crate::db::media_info::MediaInfoRecord;
use crate::db::user_state::LocalState;
use crate::drive::models::DriveFile;
use crate::recorded_date;

/// Shown when a folder name could not be fetched (for example while offline).
const UNKNOWN_FOLDER: &str = "Google Drive";

/// Where a video's file is. Snapshots stored before folders on this computer existed are Drive's.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum VideoSource {
    #[default]
    Drive,
    Local,
}

/// Metadata of one video in Drive or on this computer, as Homereel displays it. Also the
/// snapshot stored for videos with local state.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VideoFile {
    pub id: String,
    pub file_name: String,
    pub mime_type: String,
    pub duration_ms: Option<u64>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub size_bytes: Option<u64>,
    pub modified_time: Option<String>,
    pub created_time: Option<String>,
    pub folder_id: Option<String>,
    pub folder_name: String,
    pub has_thumbnail: bool,
    #[serde(default)]
    pub md5_checksum: Option<String>,
    #[serde(default)]
    pub source: VideoSource,
}

impl VideoFile {
    pub fn from_drive(file: DriveFile, folder_names: &HashMap<String, String>) -> Self {
        let folder_id = file.parents.first().cloned();
        let folder_name = match &folder_id {
            None => SHARED_WITH_ME.to_owned(),
            Some(id) => folder_names
                .get(id)
                .cloned()
                .unwrap_or_else(|| UNKNOWN_FOLDER.to_owned()),
        };
        let metadata = file.video_media_metadata.as_ref();
        Self {
            duration_ms: metadata.and_then(|m| m.duration_millis),
            width: metadata.and_then(|m| m.width),
            height: metadata.and_then(|m| m.height),
            id: file.id,
            file_name: file.name,
            mime_type: file.mime_type,
            size_bytes: file.size,
            modified_time: file.modified_time,
            created_time: file.created_time,
            folder_id,
            folder_name,
            has_thumbnail: file.has_thumbnail,
            md5_checksum: file.md5_checksum,
            source: VideoSource::Drive,
        }
    }

    /// `media_info` is what ffmpeg has read from the file so far, if anything.
    pub fn from_local(video: LocalVideo, media_info: Option<&MediaInfoRecord>) -> Self {
        Self {
            duration_ms: media_info.and_then(|info| info.duration_ms),
            width: media_info.and_then(|info| info.width),
            height: media_info.and_then(|info| info.height),
            id: video.id,
            file_name: video.file_name,
            mime_type: video.mime_type.to_owned(),
            size_bytes: Some(video.size_bytes),
            modified_time: video.modified.and_then(local_files::rfc3339),
            // "Date uploaded" of a file on this computer is when it arrived there.
            created_time: video.created.and_then(local_files::rfc3339),
            folder_id: Some(video.folder_id),
            folder_name: video.folder_name,
            // Every local video gets a frame as its thumbnail.
            has_thumbnail: true,
            md5_checksum: None,
            source: VideoSource::Local,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum TitleSource {
    Custom,
    Ai,
    FileName,
}

/// A video with Homereel's local state, as cards, rows and the watch page show it.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VideoSummary {
    #[serde(flatten)]
    pub file: VideoFile,
    /// The day the video was recorded, `YYYY-MM-DD`, when known; see [`recorded_date`].
    pub recorded_date: Option<String>,
    pub title: String,
    pub title_source: TitleSource,
    pub custom_title: Option<String>,
    pub ai_title: Option<String>,
    pub ai_title_status: Option<String>,
    pub is_favorite: bool,
    pub is_watch_later: bool,
    pub last_position_ms: Option<u64>,
    pub watched: bool,
    pub last_watched_at: Option<String>,
}

impl VideoSummary {
    /// `stored_recorded_date` is the date read from the file, if it has been read yet.
    pub fn new(
        file: VideoFile,
        state: Option<&LocalState>,
        stored_recorded_date: Option<String>,
    ) -> Self {
        let state = state.cloned().unwrap_or_default();
        let ai_title = state
            .ai_title
            .filter(|_| state.ai_status.as_deref() == Some("done"));
        let (title, title_source) = display_title(
            &file.file_name,
            state.custom_title.as_deref(),
            ai_title.as_deref(),
        );
        // A date in the file name wins over the file's metadata, so it needs no read.
        let recorded_date = recorded_date::from_file_name(&file.file_name).or(stored_recorded_date);
        Self {
            file,
            recorded_date,
            title,
            title_source,
            custom_title: state.custom_title,
            ai_title,
            ai_title_status: state.ai_status,
            is_favorite: state.is_favorite,
            is_watch_later: state.is_watch_later,
            last_position_ms: state.last_position_ms,
            watched: state.watched,
            last_watched_at: state.last_watched_at,
        }
    }

    /// "Date created" as the UI shows and sorts it, `YYYY-MM-DD`: the recording date, else
    /// Drive's modified date, which for an upload is usually the file's date on the device.
    pub fn date_created(&self) -> Option<&str> {
        self.recorded_date.as_deref().or_else(|| {
            let modified = self.file.modified_time.as_deref()?;
            modified.get(..10).or(Some(modified))
        })
    }
}

/// The user's edit wins, then the AI title, then the Drive filename.
pub fn display_title(
    file_name: &str,
    custom_title: Option<&str>,
    ai_title: Option<&str>,
) -> (String, TitleSource) {
    fn non_blank(title: Option<&str>) -> Option<&str> {
        title.map(str::trim).filter(|text| !text.is_empty())
    }
    if let Some(custom) = non_blank(custom_title) {
        return (custom.to_owned(), TitleSource::Custom);
    }
    if let Some(ai) = non_blank(ai_title) {
        return (ai.to_owned(), TitleSource::Ai);
    }
    (file_name.to_owned(), TitleSource::FileName)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn drive_file(parents: &[&str]) -> DriveFile {
        serde_json::from_value(serde_json::json!({
            "id": "f1",
            "name": "IMG_0817.m4v",
            "mimeType": "video/mp4",
            "parents": parents,
            "md5Checksum": "abc",
            "videoMediaMetadata": { "width": 1080, "height": 1920, "durationMillis": "17000" }
        }))
        .unwrap()
    }

    #[test]
    fn uses_the_first_parent_folder_name() {
        let names = HashMap::from([("fold1".to_string(), "Holidays".to_string())]);

        let video = VideoFile::from_drive(drive_file(&["fold1"]), &names);

        assert_eq!(video.folder_id.as_deref(), Some("fold1"));
        assert_eq!(video.folder_name, "Holidays");
        assert_eq!(video.duration_ms, Some(17_000));
        assert_eq!(video.md5_checksum.as_deref(), Some("abc"));
    }

    #[test]
    fn files_without_parents_are_shared_with_me() {
        let video = VideoFile::from_drive(drive_file(&[]), &HashMap::new());

        assert_eq!(video.folder_id, None);
        assert_eq!(video.folder_name, SHARED_WITH_ME);
    }

    #[test]
    fn unresolved_folders_fall_back_to_google_drive() {
        let video = VideoFile::from_drive(drive_file(&["fold9"]), &HashMap::new());

        assert_eq!(video.folder_name, UNKNOWN_FOLDER);
    }

    #[test]
    fn local_videos_take_their_size_and_duration_from_the_media_info() {
        let video = LocalVideo {
            id: "local-1".into(),
            path: r"C:\Videos\Trip\IMG_1.MOV".into(),
            file_name: "IMG_1.MOV".into(),
            mime_type: "video/quicktime",
            size_bytes: 2_000,
            modified: Some(std::time::UNIX_EPOCH + std::time::Duration::from_secs(1_718_216_095)),
            created: None,
            folder_id: "localdir-1".into(),
            folder_name: "Trip".into(),
        };
        let info = MediaInfoRecord {
            duration_ms: Some(15_000),
            width: Some(1080),
            height: Some(1920),
        };

        let file = VideoFile::from_local(video.clone(), Some(&info));
        let unread = VideoFile::from_local(video, None);

        assert_eq!(file.source, VideoSource::Local);
        assert_eq!((file.width, file.height), (Some(1080), Some(1920)));
        assert_eq!(
            file.modified_time.as_deref(),
            Some("2024-06-12T18:14:55.000Z")
        );
        assert_eq!(file.folder_id.as_deref(), Some("localdir-1"));
        assert_eq!(unread.duration_ms, None);
        assert_eq!(serde_json::to_value(&file).unwrap()["source"], "local");
    }

    #[test]
    fn snapshots_without_a_source_are_from_drive() {
        let mut json = serde_json::to_value(crate::db::test_support::video("a", "a.mp4")).unwrap();
        json.as_object_mut().unwrap().remove("source");

        let file: VideoFile = serde_json::from_value(json).unwrap();

        assert_eq!(file.source, VideoSource::Drive);
    }

    #[test]
    fn title_precedence_is_custom_then_ai_then_filename() {
        assert_eq!(
            display_title("IMG_1.mp4", Some(" Beach day "), Some("Kids at the beach")),
            ("Beach day".to_string(), TitleSource::Custom)
        );
        assert_eq!(
            display_title("IMG_1.mp4", Some("  "), Some("Kids at the beach")),
            ("Kids at the beach".to_string(), TitleSource::Ai)
        );
        assert_eq!(
            display_title("IMG_1.mp4", None, None),
            ("IMG_1.mp4".to_string(), TitleSource::FileName)
        );
    }

    #[test]
    fn summary_ignores_ai_titles_that_are_not_done() {
        let file = VideoFile::from_drive(drive_file(&[]), &HashMap::new());
        let state = LocalState {
            ai_title: Some("Old title".into()),
            ai_status: Some("failed".into()),
            is_favorite: true,
            ..LocalState::default()
        };

        let summary = VideoSummary::new(file, Some(&state), None);

        assert_eq!(summary.title, "IMG_0817.m4v");
        assert_eq!(summary.title_source, TitleSource::FileName);
        assert!(summary.is_favorite);
        let json = serde_json::to_value(&summary).unwrap();
        assert_eq!(json["fileName"], "IMG_0817.m4v");
        assert_eq!(json["titleSource"], "fileName");
    }

    #[test]
    fn date_created_prefers_the_file_name_then_the_read_date_then_drive_modified() {
        let mut file = VideoFile::from_drive(drive_file(&[]), &HashMap::new());
        file.modified_time = Some("2026-06-16T09:30:00.000Z".into());

        let unread = VideoSummary::new(file.clone(), None, None);
        let read = VideoSummary::new(file.clone(), None, Some("2017-07-09".into()));
        file.file_name = "08132019_224238.m4v".into();
        let dated_name = VideoSummary::new(file, None, Some("2026-06-16".into()));

        assert_eq!(unread.recorded_date, None);
        assert_eq!(unread.date_created(), Some("2026-06-16"));
        assert_eq!(read.date_created(), Some("2017-07-09"));
        assert_eq!(dated_name.date_created(), Some("2019-08-13"));
        assert_eq!(
            serde_json::to_value(&read).unwrap()["recordedDate"],
            "2017-07-09"
        );
    }
}
