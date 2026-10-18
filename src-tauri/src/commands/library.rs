use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use tauri::State;

use crate::db::user_state::{self, ListPage, VideoList};
use crate::error::AppResult;
use crate::library::date_order;
use crate::library::home_mix::{self, Mix};
use crate::library::local_files::is_local_id;
use crate::library::local_listings::{self, PagePosition};
use crate::library::search;
use crate::library::shorts;
use crate::library::sort_order::{self, VideoOrder};
use crate::library::video_summary::{VideoFile, VideoSource, VideoSummary};
use crate::state::AppState;

const DEFAULT_PAGE_SIZE: u32 = 48;

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct VideoListRequest {
    search: Option<String>,
    folder_id: Option<String>,
    order: VideoOrder,
    /// Home's mix, in place of `order`.
    mix: Option<Mix>,
    page_token: Option<String>,
    page_size: Option<u32>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ShortsListRequest {
    folder_id: Option<String>,
    /// Home's mix, in place of newest first.
    mix: Option<Mix>,
    page_token: Option<String>,
    page_size: Option<u32>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VideoPage {
    videos: Vec<VideoSummary>,
    next_page_token: Option<String>,
    /// Home's On this day videos, on the first page of a mix.
    #[serde(skip_serializing_if = "Option::is_none")]
    on_this_day: Option<Vec<VideoSummary>>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FolderSummary {
    id: String,
    name: String,
    video_count: usize,
    source: VideoSource,
}

/// Which videos a listing covers and which page of it to return.
struct Listing {
    folder_id: Option<String>,
    page_token: Option<String>,
    page_size: u32,
}

#[tauri::command]
pub async fn videos_list(
    state: State<'_, AppState>,
    request: VideoListRequest,
) -> AppResult<VideoPage> {
    let search = request
        .search
        .as_deref()
        .map(str::trim)
        .filter(|text| !text.is_empty())
        .map(str::to_owned);
    let listing = Listing {
        folder_id: request.folder_id,
        page_token: request.page_token,
        page_size: request.page_size.unwrap_or(DEFAULT_PAGE_SIZE),
    };
    if search.is_none()
        && let Some(mix) = &request.mix
    {
        return mixed_page(&state, &listing, mix).await;
    }
    let (videos, next_page_token) = match &search {
        // Videos are searched with their titles, which only Homereel knows.
        Some(text) => {
            locally_arranged_page(&state, &listing, |mut videos| {
                date_order::sort_by_date_created(&mut videos, true);
                search::matching(videos, text)
            })
            .await?
        }
        None => {
            locally_arranged_page(&state, &listing, |mut videos| {
                sort_order::sort(&mut videos, request.order);
                videos
            })
            .await?
        }
    };
    Ok(VideoPage {
        videos,
        next_page_token,
        on_this_day: None,
    })
}

/// One page of Home's mix of the folder (or library), with its On this day videos on the first
/// page. Videos in progress are left out of the mix of all videos, as Home shows them above it.
async fn mixed_page(state: &AppState, listing: &Listing, mix: &Mix) -> AppResult<VideoPage> {
    let skip_in_progress = listing.folder_id.is_none();
    let mut on_this_day = None;
    let (videos, next_page_token) = locally_arranged_page(state, listing, |videos| {
        on_this_day = Some(home_mix::on_this_day(&videos, &mix.today));
        home_mix::arrange(videos, mix, skip_in_progress)
    })
    .await?;
    Ok(VideoPage {
        videos,
        next_page_token,
        on_this_day: on_this_day.filter(|_| listing.page_token.is_none()),
    })
}

/// Shorts in the folder (or library), newest recorded first, or in Home's mix for its shelf. Like
/// YouTube's Shorts feed it has no order to choose; every video is listed and filtered by shape
/// here.
#[tauri::command]
pub async fn shorts_list(
    state: State<'_, AppState>,
    request: ShortsListRequest,
) -> AppResult<VideoPage> {
    let listing = Listing {
        folder_id: request.folder_id,
        page_token: request.page_token,
        page_size: request.page_size.unwrap_or(DEFAULT_PAGE_SIZE),
    };
    let (videos, next_page_token) = locally_arranged_page(&state, &listing, |mut videos| {
        videos.retain(|video| shorts::is_short(&video.file));
        match &request.mix {
            Some(mix) => home_mix::arrange(videos, mix, false),
            None => {
                date_order::sort_by_date_created(&mut videos, true);
                videos
            }
        }
    })
    .await?;
    Ok(VideoPage {
        videos,
        next_page_token,
        on_this_day: None,
    })
}

/// One page of a list Homereel orders or filters itself. The first page lists every video in
/// the folder (or library) and `arrange`s them; later pages come from that listing while it is
/// kept.
async fn locally_arranged_page(
    state: &AppState,
    listing: &Listing,
    arrange: impl FnOnce(Vec<VideoSummary>) -> Vec<VideoSummary>,
) -> AppResult<(Vec<VideoSummary>, Option<String>)> {
    let page_size = listing.page_size.max(1) as usize;
    let position = listing
        .page_token
        .as_deref()
        .and_then(PagePosition::from_token);
    if let Some(position) = position
        && let Some((videos, next)) = state.local_listings.page(position, page_size)
    {
        // Local state is read again so favorites and titles changed while scrolling show.
        return Ok((
            state.with_local_state(videos)?,
            next.map(PagePosition::to_token),
        ));
    }

    let files = state.library_videos(listing.folder_id.as_deref()).await?;
    let arranged = arrange(state.with_local_state(files)?);
    let videos: Vec<VideoFile> = arranged
        .iter()
        .map(|summary| summary.file.clone())
        .collect();
    let start = PagePosition {
        listing_id: state.local_listings.store(videos.clone()),
        offset: position.map_or(0, |position| position.offset),
    };
    let (_, next) = local_listings::page_of(&videos, start, page_size);
    let page = arranged
        .into_iter()
        .skip(start.offset)
        .take(page_size)
        .collect();
    Ok((page, next.map(PagePosition::to_token)))
}

#[tauri::command]
pub async fn video_get(state: State<'_, AppState>, drive_id: String) -> AppResult<VideoSummary> {
    // Always check the source, so a deleted or unshared file shows as unavailable instead of
    // stale.
    if is_local_id(&drive_id) {
        let video = state.local_video(&drive_id).await?;
        let mut videos = state.with_local_state(vec![video])?;
        return Ok(videos.remove(0));
    }
    let file = state.drive.file(&drive_id).await?;
    let mut videos = state.summarize(vec![file]).await?;
    Ok(videos.remove(0))
}

/// Favorites, Watch later, History or Continue watching, newest first: the whole list, or
/// `limit` videos from `offset` when both are given.
#[tauri::command]
pub fn library_list(
    state: State<'_, AppState>,
    list: VideoList,
    offset: Option<u32>,
    limit: Option<u32>,
) -> AppResult<Vec<VideoSummary>> {
    let page = limit.map(|limit| ListPage {
        offset: offset.unwrap_or(0),
        limit,
    });
    let ids = state
        .db
        .with(|connection| user_state::list_ids(connection, list, page))?;
    state.summaries_for_ids(&ids)
}

/// Folders in Drive or on this computer that directly contain videos, most videos first.
#[tauri::command]
pub async fn folders_list(state: State<'_, AppState>) -> AppResult<Vec<FolderSummary>> {
    let videos = state.library_videos(None).await?;
    Ok(folder_summaries(&videos))
}

fn folder_summaries(videos: &[VideoFile]) -> Vec<FolderSummary> {
    let mut folders: HashMap<&str, FolderSummary> = HashMap::new();
    for video in videos {
        let Some(id) = video.folder_id.as_deref() else {
            continue;
        };
        folders
            .entry(id)
            .or_insert_with(|| FolderSummary {
                id: id.to_owned(),
                name: video.folder_name.clone(),
                video_count: 0,
                source: video.source,
            })
            .video_count += 1;
    }
    let mut folders: Vec<FolderSummary> = folders.into_values().collect();
    folders.sort_by(|a, b| {
        b.video_count
            .cmp(&a.video_count)
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });
    folders
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::test_support::video;

    #[test]
    fn counts_videos_per_folder_across_sources() {
        let mut trip = video("local-1", "a.mp4");
        trip.folder_id = Some("localdir-trip".into());
        trip.folder_name = "Trip".into();
        trip.source = VideoSource::Local;
        let mut shared = video("s", "s.mp4");
        shared.folder_id = None;
        let videos = vec![video("d1", "1.mp4"), video("d2", "2.mp4"), trip, shared];

        let folders = folder_summaries(&videos);

        let found: Vec<(&str, usize, VideoSource)> = folders
            .iter()
            .map(|folder| (folder.name.as_str(), folder.video_count, folder.source))
            .collect();
        assert_eq!(
            found,
            [
                ("Holidays", 2, VideoSource::Drive),
                ("Trip", 1, VideoSource::Local)
            ]
        );
    }
}
