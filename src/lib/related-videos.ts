import type { VideoSummary } from './api';

/**
 * The watch page's related list: the feed without the playing video and without the Shorts the
 * row above it already suggests, so no video is offered twice.
 */
export function relatedVideos(
  feed: VideoSummary[],
  currentId: string,
  rowShorts: VideoSummary[],
): VideoSummary[] {
  const shownInRow = new Set(rowShorts.map((short) => short.id));
  return feed.filter((video) => video.id !== currentId && !shownInRow.has(video.id));
}
