import { useMutation, useQueryClient } from '@tanstack/react-query';

import { api, type VideoSummary } from '@/lib/api';
import { toAppError } from '@/lib/app-error';
import { showToast } from '@/lib/toast';
import { patchVideoInCaches, replaceVideoInCaches } from '@/lib/video-cache';

type ToggleInput = { video: VideoSummary; value: boolean };

/** Adds or removes a favorite, updating every list at once and rolling back on failure. */
export function useToggleFavorite() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({ video, value }: ToggleInput) => api.setFavorite(video.id, value),
    onMutate: ({ video, value }) =>
      patchVideoInCaches(queryClient, video.id, { isFavorite: value }),
    onSuccess: (updated) => {
      replaceVideoInCaches(queryClient, updated);
      void queryClient.invalidateQueries({ queryKey: ['library', 'favorites'] });
      showToast(updated.isFavorite ? 'Added to Favorites' : 'Removed from Favorites');
    },
    onError: (error, { video }) => {
      patchVideoInCaches(queryClient, video.id, { isFavorite: video.isFavorite });
      showToast(toAppError(error).message);
    },
  });
}

export function useToggleWatchLater() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({ video, value }: ToggleInput) => api.setWatchLater(video.id, value),
    onMutate: ({ video, value }) =>
      patchVideoInCaches(queryClient, video.id, { isWatchLater: value }),
    onSuccess: (updated) => {
      replaceVideoInCaches(queryClient, updated);
      void queryClient.invalidateQueries({ queryKey: ['library', 'watchLater'] });
      showToast(updated.isWatchLater ? 'Saved to Watch later' : 'Removed from Watch later');
    },
    onError: (error, { video }) => {
      patchVideoInCaches(queryClient, video.id, { isWatchLater: video.isWatchLater });
      showToast(toAppError(error).message);
    },
  });
}

/** Sets or clears the user's own title. The Drive file keeps its name. */
export function useSetCustomTitle() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({ videoId, title }: { videoId: string; title: string | null }) =>
      api.setCustomTitle(videoId, title),
    onSuccess: (updated) => {
      replaceVideoInCaches(queryClient, updated);
      showToast(updated.titleSource === 'custom' ? 'Title saved' : 'Title reset');
    },
  });
}

/** Titles one video right away. Reading frames and asking Gemini takes 10–60 seconds. */
export function useRegenerateTitle() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (videoId: string) => api.titleRegenerate(videoId),
    onMutate: () =>
      showToast('Generating a new title… This can take up to a minute.', { persistent: true }),
    onSuccess: (updated) => {
      replaceVideoInCaches(queryClient, updated);
      void queryClient.invalidateQueries({ queryKey: ['titles-status'] });
      void queryClient.invalidateQueries({ queryKey: ['video-people', updated.id] });
      showToast(
        updated.titleSource === 'custom'
          ? `New AI title saved: ${updated.aiTitle}. Your own title is still shown.`
          : `New title: ${updated.title}`,
      );
    },
    onError: (error) => showToast(toAppError(error).message, { persistent: true }),
  });
}
