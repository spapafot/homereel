//! Library data served to the UI. Videos are read live from Drive; only DriveTube's own data is
//! stored locally (see `db`), including each video's recording date once it has been read.

pub mod date_order;
pub mod date_scan;
pub mod folder_names;
pub mod local_listings;
pub mod search;
pub mod video_summary;

use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use crate::drive::models::DriveFile;
use video_summary::VideoFile;

#[derive(Debug, Clone, Default)]
pub struct KnownFile {
    pub thumbnail_link: Option<String>,
    pub video: Option<VideoFile>,
}

/// Drive files DriveTube has listed this session. The media proxy only streams these, the
/// thumbnail protocol reuses their `thumbnailLink`, and actions reuse their metadata.
#[derive(Clone, Default)]
pub struct KnownFiles(Arc<RwLock<HashMap<String, KnownFile>>>);

impl KnownFiles {
    pub fn remember(&self, files: &[DriveFile]) {
        if let Ok(mut known) = self.0.write() {
            for file in files {
                known.entry(file.id.clone()).or_default().thumbnail_link =
                    file.thumbnail_link.clone();
            }
        }
    }

    pub fn remember_videos(&self, videos: &[VideoFile]) {
        if let Ok(mut known) = self.0.write() {
            for video in videos {
                known.entry(video.id.clone()).or_default().video = Some(video.clone());
            }
        }
    }

    pub fn contains(&self, id: &str) -> bool {
        self.0.read().is_ok_and(|known| known.contains_key(id))
    }

    pub fn thumbnail_link(&self, id: &str) -> Option<String> {
        self.0
            .read()
            .ok()
            .and_then(|known| known.get(id).and_then(|file| file.thumbnail_link.clone()))
    }

    pub fn video(&self, id: &str) -> Option<VideoFile> {
        self.0
            .read()
            .ok()
            .and_then(|known| known.get(id).and_then(|file| file.video.clone()))
    }

    pub fn clear(&self) {
        if let Ok(mut known) = self.0.write() {
            known.clear();
        }
    }
}
