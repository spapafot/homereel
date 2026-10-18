import { useSyncExternalStore } from 'react';

import { thumbnailUrl } from '@/lib/api';
import { getThumbnailVersion, subscribeThumbnailVersions } from '@/lib/thumbnail-versions';

/** A video's thumbnail URL, which changes when the thumbnail is regenerated. */
export function useThumbnailUrl(videoId: string | null | undefined): string | undefined {
  const version = useSyncExternalStore(subscribeThumbnailVersions, () =>
    videoId ? getThumbnailVersion(videoId) : 0,
  );
  return videoId ? thumbnailUrl(videoId, version) : undefined;
}
