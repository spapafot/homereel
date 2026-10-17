import { useInfiniteQuery, useMutation, useQuery, useQueryClient } from '@tanstack/react-query';

import { api, type LibraryList, type PlaylistDetail } from '@/lib/api';
import { toAppError } from '@/lib/app-error';
import { showToast } from '@/lib/toast';

export const HISTORY_PAGE_SIZE = 48;

export function useLibraryList(list: LibraryList) {
  return useQuery({ queryKey: ['library', list], queryFn: () => api.libraryList(list) });
}

/** Watch history, a page at a time as the user scrolls; it keeps every video ever watched. */
export function useHistoryFeed() {
  return useInfiniteQuery({
    queryKey: ['history'],
    queryFn: ({ pageParam }) =>
      api.libraryList('history', { offset: pageParam, limit: HISTORY_PAGE_SIZE }),
    initialPageParam: 0,
    getNextPageParam: (lastPage, pages) =>
      lastPage.length < HISTORY_PAGE_SIZE ? undefined : pages.length * HISTORY_PAGE_SIZE,
  });
}

export function usePlaylists() {
  return useQuery({ queryKey: ['playlists'], queryFn: api.playlistsList });
}

export function usePlaylist(playlistId: number | null) {
  return useQuery({
    queryKey: ['playlist', playlistId],
    queryFn: () => api.playlistGet(playlistId!),
    enabled: playlistId != null,
  });
}

export function usePlaylistMemberships(videoId: string, enabled: boolean) {
  return useQuery({
    queryKey: ['playlist-memberships', videoId],
    queryFn: () => api.playlistsForVideo(videoId),
    enabled,
  });
}

/** Invalidates everything that shows playlists after a change. */
function usePlaylistInvalidation() {
  const queryClient = useQueryClient();
  return () =>
    Promise.all([
      queryClient.invalidateQueries({ queryKey: ['playlists'] }),
      queryClient.invalidateQueries({ queryKey: ['playlist'] }),
      queryClient.invalidateQueries({ queryKey: ['playlist-memberships'] }),
    ]);
}

export function useCreatePlaylist() {
  const invalidate = usePlaylistInvalidation();
  return useMutation({
    mutationFn: ({ name, videoId }: { name: string; videoId?: string }) =>
      api.playlistCreate(name, videoId),
    onSuccess: (playlist, { videoId }) => {
      void invalidate();
      showToast(videoId ? `Saved to ${playlist.name}` : `Created ${playlist.name}`);
    },
  });
}

export function useTogglePlaylistVideo() {
  const invalidate = usePlaylistInvalidation();
  return useMutation({
    mutationFn: ({
      playlistId,
      videoId,
      add,
    }: {
      playlistId: number;
      videoId: string;
      add: boolean;
      name: string;
    }) =>
      add
        ? api.playlistAddVideo(playlistId, videoId)
        : api.playlistRemoveVideo(playlistId, videoId),
    onSuccess: (_result, { add, name }) => {
      void invalidate();
      showToast(add ? `Saved to ${name}` : `Removed from ${name}`);
    },
    onError: (error) => showToast(toAppError(error).message),
  });
}

export function useRenamePlaylist() {
  const invalidate = usePlaylistInvalidation();
  return useMutation({
    mutationFn: ({ playlistId, name }: { playlistId: number; name: string }) =>
      api.playlistRename(playlistId, name),
    onSuccess: () => void invalidate(),
  });
}

export function useDeletePlaylist() {
  const invalidate = usePlaylistInvalidation();
  return useMutation({
    mutationFn: ({ playlistId }: { playlistId: number; name: string }) =>
      api.playlistDelete(playlistId),
    onSuccess: (_result, { name }) => {
      void invalidate();
      showToast(`Deleted ${name}`);
    },
    onError: (error) => showToast(toAppError(error).message),
  });
}

/** Reorders immediately on screen, then saves; the server order wins once it answers. */
export function useReorderPlaylist() {
  const queryClient = useQueryClient();
  const invalidate = usePlaylistInvalidation();
  return useMutation({
    mutationFn: ({ playlistId, videoIds }: { playlistId: number; videoIds: string[] }) =>
      api.playlistReorder(playlistId, videoIds),
    onMutate: ({ playlistId, videoIds }) => {
      queryClient.setQueryData<PlaylistDetail>(['playlist', playlistId], (detail) => {
        if (!detail) return detail;
        const byId = new Map(detail.videos.map((video) => [video.id, video]));
        const videos = videoIds.flatMap((id) => byId.get(id) ?? []);
        return { ...detail, videos };
      });
    },
    onSettled: () => void invalidate(),
    onError: (error) => showToast(toAppError(error).message),
  });
}

function useHistoryInvalidation() {
  const queryClient = useQueryClient();
  return () =>
    Promise.all([
      queryClient.invalidateQueries({ queryKey: ['history'] }),
      queryClient.invalidateQueries({ queryKey: ['library', 'continueWatching'] }),
      queryClient.invalidateQueries({ queryKey: ['videos'] }),
      queryClient.invalidateQueries({ queryKey: ['video'] }),
    ]);
}

export function useRemoveFromHistory() {
  const invalidate = useHistoryInvalidation();
  return useMutation({
    mutationFn: (videoId: string) => api.historyRemove(videoId),
    onSuccess: () => {
      void invalidate();
      showToast('Removed from watch history');
    },
    onError: (error) => showToast(toAppError(error).message),
  });
}

export function useClearHistory() {
  const invalidate = useHistoryInvalidation();
  return useMutation({
    mutationFn: api.historyClear,
    onSuccess: () => {
      void invalidate();
      showToast('Watch history cleared');
    },
    onError: (error) => showToast(toAppError(error).message),
  });
}
