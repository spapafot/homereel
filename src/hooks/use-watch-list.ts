import { useQuery } from '@tanstack/react-query';

import { api, type LibraryList, type VideoSummary } from '@/lib/api';
import { FAVORITES_LIST, WATCH_LATER_LIST } from '@/lib/paths';

import { usePlaylist } from './use-library';

export type WatchList = {
  id: string;
  title: string;
  videos: VideoSummary[];
};

const LIBRARY_LISTS: Record<string, { kind: LibraryList; title: string }> = {
  [WATCH_LATER_LIST]: { kind: 'watchLater', title: 'Watch later' },
  [FAVORITES_LIST]: { kind: 'favorites', title: 'Favorites' },
};

export function parsePlaylistId(listId: string | null): number | null {
  return listId && /^\d+$/.test(listId) ? Number(listId) : null;
}

/** The list a video is played from (`?list=`), for the playlist panel and Next. */
export function useWatchList(listId: string | null): {
  list: WatchList | null;
  isPending: boolean;
} {
  const library = listId ? LIBRARY_LISTS[listId] : undefined;
  const playlistId = parsePlaylistId(listId);
  const libraryQuery = useQuery({
    queryKey: ['library', library?.kind],
    queryFn: () => api.libraryList(library!.kind),
    enabled: library != null,
  });
  const playlistQuery = usePlaylist(playlistId);

  if (library && listId) {
    return {
      isPending: libraryQuery.isPending,
      list: libraryQuery.data
        ? { id: listId, title: library.title, videos: libraryQuery.data }
        : null,
    };
  }
  if (playlistId != null && listId) {
    return {
      isPending: playlistQuery.isPending,
      list: playlistQuery.data
        ? { id: listId, title: playlistQuery.data.playlist.name, videos: playlistQuery.data.videos }
        : null,
    };
  }
  return { list: null, isPending: false };
}
