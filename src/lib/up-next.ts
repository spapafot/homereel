import type { VideoSummary } from './api';

/**
 * The video after the current one in its list, used for the Next button and autoplay. Returns
 * null when the current video is last or not in the loaded part of the list.
 */
export function pickNextVideo(list: VideoSummary[], currentId: string): VideoSummary | null {
  const index = list.findIndex((video) => video.id === currentId);
  if (index < 0 || index + 1 >= list.length) return null;
  return list[index + 1];
}
