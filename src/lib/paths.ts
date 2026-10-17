/** App URLs, shaped like YouTube's where there is an equivalent. */

/** YouTube's list IDs for Watch later and liked videos; DriveTube's Favorites play the liked role. */
export const WATCH_LATER_LIST = 'WL';
export const FAVORITES_LIST = 'LL';

export type WatchContext = {
  folderId?: string | null;
  /** `WL`, `LL` or a playlist ID: next and previous follow that list. */
  list?: string | null;
};

export function watchPath(videoId: string, context: WatchContext = {}): string {
  const params = new URLSearchParams({ v: videoId });
  if (context.list) params.set('list', context.list);
  else if (context.folderId) params.set('folder', context.folderId);
  return `/watch?${params.toString()}`;
}

export function folderPath(folderId: string): string {
  return `/folder/${encodeURIComponent(folderId)}`;
}

export function playlistPath(list: string | number): string {
  return `/playlist?${new URLSearchParams({ list: String(list) }).toString()}`;
}

export function searchPath(query: string): string {
  return `/results?${new URLSearchParams({ search_query: query }).toString()}`;
}
