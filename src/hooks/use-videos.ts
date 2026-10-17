import { useInfiniteQuery, useQuery, useQueryClient } from '@tanstack/react-query';
import { listen } from '@tauri-apps/api/event';
import { useEffect } from 'react';

import {
  api,
  RECORDED_DATE_EVENT,
  RECORDED_DATES_DONE_EVENT,
  type RecordedDateEvent,
  type VideoListRequest,
  type VideoSummary,
} from '@/lib/api';
import { pickNextVideo } from '@/lib/up-next';
import { patchVideoInCaches } from '@/lib/video-cache';

const FOLDERS_STALE_MS = 5 * 60_000;

type FeedRequest = Omit<VideoListRequest, 'pageToken'>;

/** Pages of videos from Drive, loaded as the user scrolls. */
export function useVideoFeed(request: FeedRequest, options: { enabled?: boolean } = {}) {
  return useInfiniteQuery({
    queryKey: ['videos', request],
    queryFn: ({ pageParam }) => api.videosList({ ...request, pageToken: pageParam }),
    initialPageParam: undefined as string | undefined,
    getNextPageParam: (lastPage) => lastPage.nextPageToken ?? undefined,
    enabled: options.enabled ?? true,
  });
}

/**
 * The video after `videoId` in a feed, for Next and autoplay. The current video can be past the
 * pages loaded so far, so further pages load until the next one is known.
 */
export function useNextInFeed(
  feed: ReturnType<typeof useVideoFeed>,
  videoId: string,
  { enabled }: { enabled: boolean },
): VideoSummary | null {
  const videos = feed.data?.pages.flatMap((page) => page.videos) ?? [];
  const next = pickNextVideo(videos, videoId);
  const shouldLoadMore = enabled && next == null && feed.hasNextPage && !feed.isFetchingNextPage;
  const { fetchNextPage } = feed;
  // A fast page can arrive without a render in between that shows it loading, so the page count
  // re-runs the check too.
  const pageCount = feed.data?.pages.length ?? 0;

  useEffect(() => {
    if (shouldLoadMore) void fetchNextPage();
  }, [shouldLoadMore, pageCount, fetchNextPage]);

  return next;
}

export function useVideo(driveId: string) {
  return useQuery({
    queryKey: ['video', driveId],
    queryFn: () => api.videoGet(driveId),
    enabled: driveId !== '',
  });
}

export function usePlaybackSource(driveId: string, enabled: boolean) {
  return useQuery({
    queryKey: ['playback', driveId],
    queryFn: () => api.playbackSource(driveId),
    enabled,
    staleTime: Infinity,
  });
}

export function useFolders() {
  return useQuery({
    queryKey: ['folders'],
    queryFn: api.foldersList,
    staleTime: FOLDERS_STALE_MS,
  });
}

/** Shows recording dates as the background scan reads them, without listing Drive again. */
export function useRecordedDateEvents() {
  const queryClient = useQueryClient();

  useEffect(() => {
    const unlisteners = [
      listen<RecordedDateEvent>(RECORDED_DATE_EVENT, ({ payload }) =>
        patchVideoInCaches(queryClient, payload.driveId, { recordedDate: payload.recordedDate }),
      ),
      // Feeds sorted while dates were missing re-sort the next time they are opened, rather
      // than reshuffling under the user.
      listen(RECORDED_DATES_DONE_EVENT, () =>
        queryClient.invalidateQueries({ queryKey: ['videos'], refetchType: 'none' }),
      ),
    ];
    return () => {
      for (const unlisten of unlisteners) {
        void unlisten.then((stop) => stop());
      }
    };
  }, [queryClient]);
}
