/**
 * How many times each video's thumbnail was regenerated this session. Thumbnail URLs carry the
 * count, so every image of the video loads the new picture instead of the old one.
 */
const versions = new Map<string, number>();
const listeners = new Set<() => void>();

export function bumpThumbnailVersion(videoId: string): void {
  versions.set(videoId, getThumbnailVersion(videoId) + 1);
  listeners.forEach((listener) => listener());
}

export function getThumbnailVersion(videoId: string): number {
  return versions.get(videoId) ?? 0;
}

export function subscribeThumbnailVersions(listener: () => void): () => void {
  listeners.add(listener);
  return () => listeners.delete(listener);
}
