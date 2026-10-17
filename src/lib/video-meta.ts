import type { VideoSummary } from './api';
import { formatBytes } from './format-bytes';
import { formatDuration } from './format-duration';
import { formatRelativeDate } from './format-relative-date';

/**
 * "Date created": the day the video was recorded, else Drive's modified date, which for an
 * upload is usually the file's date on the device. Rust sorts "Newest" and "Oldest" the same way.
 */
export function dateCreated(video: VideoSummary): string | null {
  return video.recordedDate ?? video.modifiedTime;
}

/** YouTube's views-and-date line, here duration and date created: `12:34 • 3 days ago`. */
export function videoMetaLine(video: VideoSummary, now?: Date): string {
  const created = dateCreated(video);
  return [
    video.durationMs != null ? formatDuration(video.durationMs) : null,
    created ? formatRelativeDate(created, now) : null,
  ]
    .filter(Boolean)
    .join(' • ');
}

/** How much of the video was watched, 0–1, for the red bar under thumbnails. */
export function watchProgress(video: VideoSummary): number | null {
  if (video.watched) return 1;
  if (!video.lastPositionMs || !video.durationMs) return null;
  return Math.min(1, video.lastPositionMs / video.durationMs);
}

/** Technical details for the watch page: `1920x1080 • 95.4 MB`. */
export function videoDetailsLine(video: VideoSummary): string {
  return [
    video.width && video.height ? `${video.width}x${video.height}` : null,
    video.sizeBytes != null ? formatBytes(video.sizeBytes) : null,
  ]
    .filter(Boolean)
    .join(' • ');
}
