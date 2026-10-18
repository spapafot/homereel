import type { VideoSummary } from './api';
import { formatBytes } from './format-bytes';
import { formatRelativeDate } from './format-relative-date';

/**
 * "Date created": the day the video was recorded, else the file's modified date, which for a Drive
 * upload is usually its date on the device. Rust sorts "Newest" and "Oldest" the same way.
 */
export function dateCreated(video: VideoSummary): string | null {
  return video.recordedDate ?? video.modifiedTime;
}

/**
 * The date in YouTube's metadata line (`3 days ago`), from the date created. The duration stays on
 * the thumbnail badge, as on YouTube.
 */
export function relativeDateCreated(video: VideoSummary, now?: Date): string | null {
  const created = dateCreated(video);
  return created ? formatRelativeDate(created, now) : null;
}

/** How much of the video was watched, 0–1, for the red bar under thumbnails. */
export function watchProgress(video: VideoSummary): number | null {
  if (video.watched) return 1;
  if (!video.lastPositionMs || !video.durationMs) return null;
  return Math.min(1, video.lastPositionMs / video.durationMs);
}

/** Technical details for the watch page and list rows: `1920x1080`, `95.4 MB`. */
export function videoDetailsParts(video: VideoSummary): string[] {
  return [
    video.width && video.height ? `${video.width}x${video.height}` : null,
    video.sizeBytes != null ? formatBytes(video.sizeBytes) : null,
  ].filter((part) => part != null);
}
