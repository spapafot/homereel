//! Lists Homereel orders or filters itself (date created, search). Drive cannot page through
//! these, so the first page lists every match and later pages come from that listing.

use std::collections::VecDeque;
use std::sync::Mutex;

use super::video_summary::VideoFile;

/// Listings kept for their later pages; an older one is listed again when needed.
const KEPT_LISTINGS: usize = 4;
const TOKEN_PREFIX: &str = "local:";

/// Where the next page of a listing starts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PagePosition {
    pub listing_id: u64,
    pub offset: usize,
}

impl PagePosition {
    pub fn to_token(self) -> String {
        format!("{TOKEN_PREFIX}{}:{}", self.listing_id, self.offset)
    }

    pub fn from_token(token: &str) -> Option<Self> {
        let (listing_id, offset) = token.strip_prefix(TOKEN_PREFIX)?.split_once(':')?;
        Some(Self {
            listing_id: listing_id.parse().ok()?,
            offset: offset.parse().ok()?,
        })
    }
}

/// Recent listings, so scrolling pages through one list instead of re-listing Drive for every
/// page.
#[derive(Default)]
pub struct LocalListings {
    state: Mutex<ListingsState>,
}

#[derive(Default)]
struct ListingsState {
    next_id: u64,
    listings: VecDeque<(u64, Vec<VideoFile>)>,
}

impl LocalListings {
    pub fn store(&self, videos: Vec<VideoFile>) -> u64 {
        let mut state = self.lock();
        state.next_id += 1;
        let id = state.next_id;
        state.listings.push_back((id, videos));
        if state.listings.len() > KEPT_LISTINGS {
            state.listings.pop_front();
        }
        id
    }

    /// The videos of one page, and the position after it when more follow. `None` when the
    /// listing is no longer kept.
    pub fn page(
        &self,
        position: PagePosition,
        page_size: usize,
    ) -> Option<(Vec<VideoFile>, Option<PagePosition>)> {
        let state = self.lock();
        let (_, videos) = state
            .listings
            .iter()
            .find(|(id, _)| *id == position.listing_id)?;
        Some(page_of(videos, position, page_size))
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, ListingsState> {
        self.state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

pub fn page_of(
    videos: &[VideoFile],
    position: PagePosition,
    page_size: usize,
) -> (Vec<VideoFile>, Option<PagePosition>) {
    let page: Vec<VideoFile> = videos
        .iter()
        .skip(position.offset)
        .take(page_size)
        .cloned()
        .collect();
    let end = position.offset + page.len();
    let next = (end < videos.len()).then_some(PagePosition {
        listing_id: position.listing_id,
        offset: end,
    });
    (page, next)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::test_support::video;

    #[test]
    fn page_tokens_round_trip_and_reject_drive_tokens() {
        let position = PagePosition {
            listing_id: 7,
            offset: 48,
        };

        assert_eq!(
            PagePosition::from_token(&position.to_token()),
            Some(position)
        );
        assert_eq!(PagePosition::from_token("ChwLEhYKBGZpbGU"), None);
        assert_eq!(PagePosition::from_token("local:x:1"), None);
    }

    #[test]
    fn pages_through_a_stored_listing_and_forgets_old_ones() {
        let listings = LocalListings::default();
        let videos: Vec<VideoFile> = (0..5).map(|n| video(&n.to_string(), "a.mp4")).collect();
        let id = listings.store(videos);
        let start = PagePosition {
            listing_id: id,
            offset: 0,
        };

        let (first, next) = listings.page(start, 3).unwrap();
        let (second, end) = listings.page(next.unwrap(), 3).unwrap();
        assert_eq!(first.len(), 3);
        assert_eq!(
            second.iter().map(|v| v.id.as_str()).collect::<Vec<_>>(),
            ["3", "4"]
        );
        assert_eq!(end, None);

        for _ in 0..KEPT_LISTINGS {
            listings.store(Vec::new());
        }
        assert!(listings.page(start, 3).is_none());
    }
}
