use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use tauri::State;

use crate::db::user_state::{self, ListPage, VideoList};
use crate::drive::client::{VideoOrder, VideoQuery};
use crate::error::AppResult;
use crate::library::date_order;
use crate::library::local_listings::{self, PagePosition};
use crate::library::search;
use crate::library::video_summary::{VideoFile, VideoSummary};
use crate::state::AppState;

const DEFAULT_PAGE_SIZE: u32 = 48;

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct VideoListRequest {
    search: Option<String>,
    folder_id: Option<String>,
    order: VideoOrder,
    page_token: Option<String>,
    page_size: Option<u32>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VideoPage {
    videos: Vec<VideoSummary>,
    next_page_token: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FolderSummary {
    id: String,
    name: String,
    video_count: usize,
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
    let query = VideoQuery {
        search: None,
        folder_id: request.folder_id,
        order: request.order,
        page_token: request.page_token,
        page_size: request.page_size.unwrap_or(DEFAULT_PAGE_SIZE),
    };
    let (videos, next_page_token) = match (&search, query.order) {
        // Drive only matches file names, so every video is searched here with its titles.
        (Some(text), _) => {
            locally_arranged_page(&state, &query, |mut videos| {
                date_order::sort_by_date_created(&mut videos, true);
                search::matching(videos, text)
            })
            .await?
        }
        (None, VideoOrder::CreatedNewest | VideoOrder::CreatedOldest) => {
            let newest_first = query.order == VideoOrder::CreatedNewest;
            locally_arranged_page(&state, &query, |mut videos| {
                date_order::sort_by_date_created(&mut videos, newest_first);
                videos
            })
            .await?
        }
        (None, _) => {
            let page = state.drive.list_videos(&query).await?;
            (state.summarize(page.files).await?, page.next_page_token)
        }
    };
    Ok(VideoPage {
        videos,
        next_page_token,
    })
}

/// One page of a list DriveTube orders or filters itself. The first page lists every video in
/// the folder (or Drive) and `arrange`s them; later pages come from that listing while it is
/// kept.
async fn locally_arranged_page(
    state: &AppState,
    query: &VideoQuery,
    arrange: impl FnOnce(Vec<VideoSummary>) -> Vec<VideoSummary>,
) -> AppResult<(Vec<VideoSummary>, Option<String>)> {
    let page_size = query.page_size.max(1) as usize;
    let position = query
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

    let files = state
        .drive
        .list_all_videos(query.folder_id.as_deref())
        .await?;
    let arranged = arrange(state.summarize(files).await?);
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
    // Always ask Drive, so a deleted or unshared file shows as unavailable instead of stale.
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

/// Folders that directly contain videos, most videos first.
#[tauri::command]
pub async fn folders_list(state: State<'_, AppState>) -> AppResult<Vec<FolderSummary>> {
    let parents = state.drive.list_video_parents().await?;
    let mut counts: HashMap<String, usize> = HashMap::new();
    for file in parents {
        if let Some(folder_id) = file.parents.into_iter().next() {
            *counts.entry(folder_id).or_default() += 1;
        }
    }
    let folder_ids: Vec<String> = counts.keys().cloned().collect();
    let names = state.folder_names(&folder_ids).await;

    let mut folders: Vec<FolderSummary> = counts
        .into_iter()
        .filter_map(|(id, video_count)| {
            names.get(&id).map(|name| FolderSummary {
                name: name.clone(),
                id,
                video_count,
            })
        })
        .collect();
    folders.sort_by(|a, b| {
        b.video_count
            .cmp(&a.video_count)
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });
    Ok(folders)
}
