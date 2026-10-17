import type { VideoSummary } from './api';

export type HistoryGroup = {
  label: string;
  videos: VideoSummary[];
};

const DAY_MS = 24 * 60 * 60 * 1000;
const WEEKDAY_WINDOW_DAYS = 7;

function startOfDay(date: Date): number {
  return new Date(date.getFullYear(), date.getMonth(), date.getDate()).getTime();
}

/** YouTube's history headings: Today, Yesterday, a weekday within the week, then the date. */
export function dayLabel(iso: string, now: Date = new Date()): string {
  const date = new Date(iso);
  const daysAgo = Math.round((startOfDay(now) - startOfDay(date)) / DAY_MS);
  if (daysAgo <= 0) return 'Today';
  if (daysAgo === 1) return 'Yesterday';
  if (daysAgo < WEEKDAY_WINDOW_DAYS) return date.toLocaleDateString('en-US', { weekday: 'long' });
  return date.toLocaleDateString('en-US', {
    month: 'short',
    day: 'numeric',
    ...(date.getFullYear() === now.getFullYear() ? {} : { year: 'numeric' }),
  });
}

/** Groups history (newest first) under day headings, keeping the order. */
export function groupHistoryByDay(videos: VideoSummary[], now: Date = new Date()): HistoryGroup[] {
  const groups: HistoryGroup[] = [];
  for (const video of videos) {
    if (!video.lastWatchedAt) continue;
    const label = dayLabel(video.lastWatchedAt, now);
    const last = groups[groups.length - 1];
    if (last?.label === label) last.videos.push(video);
    else groups.push({ label, videos: [video] });
  }
  return groups;
}
