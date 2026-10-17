import type { InfiniteData, QueryClient } from '@tanstack/react-query';

import type { PlaylistDetail, VideoPage, VideoSummary } from './api';

type VideoPatch = Partial<VideoSummary> | ((video: VideoSummary) => VideoSummary);

function apply(video: VideoSummary, patch: VideoPatch): VideoSummary {
  return typeof patch === 'function' ? patch(video) : { ...video, ...patch };
}

/**
 * Updates one video everywhere it is cached (feeds, the watch page, local lists, history,
 * playlists), so a favorite or a new title shows at once without re-listing Drive.
 */
export function patchVideoInCaches(queryClient: QueryClient, videoId: string, patch: VideoPatch) {
  const patchList = (videos: VideoSummary[]) =>
    videos.map((video) => (video.id === videoId ? apply(video, patch) : video));

  queryClient.setQueriesData<InfiniteData<VideoPage>>({ queryKey: ['videos'] }, (data) =>
    data
      ? { ...data, pages: data.pages.map((page) => ({ ...page, videos: patchList(page.videos) })) }
      : data,
  );
  queryClient.setQueriesData<VideoSummary>({ queryKey: ['video', videoId] }, (video) =>
    video ? apply(video, patch) : video,
  );
  queryClient.setQueriesData<VideoSummary[]>({ queryKey: ['library'] }, (videos) =>
    videos ? patchList(videos) : videos,
  );
  queryClient.setQueriesData<InfiniteData<VideoSummary[]>>({ queryKey: ['history'] }, (data) =>
    data ? { ...data, pages: data.pages.map(patchList) } : data,
  );
  queryClient.setQueriesData<PlaylistDetail>({ queryKey: ['playlist'] }, (detail) =>
    detail ? { ...detail, videos: patchList(detail.videos) } : detail,
  );
}

/** Applies the summary Rust returned after a change, keeping cached positions unchanged. */
export function replaceVideoInCaches(queryClient: QueryClient, updated: VideoSummary) {
  patchVideoInCaches(queryClient, updated.id, () => updated);
}
