import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { listen } from '@tauri-apps/api/event';
import { useEffect } from 'react';

import {
  api,
  TITLE_ITEM_EVENT,
  TITLE_PROGRESS_EVENT,
  type TitleItemEvent,
  type TitleQueueStatus,
  type TitleScope,
  type TitlesOverview,
} from '@/lib/api';
import { toAppError } from '@/lib/app-error';
import { showToast } from '@/lib/toast';
import { patchVideoInCaches } from '@/lib/video-cache';

const GEMINI_SETTINGS_KEY = ['gemini-settings'] as const;
const TITLES_STATUS_KEY = ['titles-status'] as const;

export function useGeminiSettings() {
  return useQuery({
    queryKey: GEMINI_SETTINGS_KEY,
    queryFn: api.geminiSettings,
    staleTime: Infinity,
  });
}

export function useGeminiModels(enabled: boolean) {
  return useQuery({
    queryKey: ['gemini-models'],
    queryFn: api.geminiModels,
    enabled,
    staleTime: Infinity,
  });
}

export function useSetGeminiKey() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (apiKey: string) => api.geminiKeySet(apiKey),
    onSuccess: (settings) => {
      queryClient.setQueryData(GEMINI_SETTINGS_KEY, settings);
      void queryClient.invalidateQueries({ queryKey: ['gemini-models'] });
      showToast('Gemini API key saved');
    },
  });
}

export function useRemoveGeminiKey() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: api.geminiKeyRemove,
    onSuccess: (settings) => {
      queryClient.setQueryData(GEMINI_SETTINGS_KEY, settings);
      showToast('Gemini API key removed');
    },
    onError: (error) => showToast(toAppError(error).message),
  });
}

export function useSetGeminiModel() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (model: string) => api.geminiModelSet(model),
    onSuccess: (settings) => queryClient.setQueryData(GEMINI_SETTINGS_KEY, settings),
    onError: (error) => showToast(toAppError(error).message),
  });
}

export function useSetGeminiInstructions() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (instructions: string) => api.geminiInstructionsSet(instructions),
    onSuccess: (settings) => {
      queryClient.setQueryData(GEMINI_SETTINGS_KEY, settings);
      showToast(
        settings.instructions
          ? 'Instructions saved. They apply to titles generated from now on.'
          : 'Instructions removed',
      );
    },
    onError: (error) => showToast(toAppError(error).message),
  });
}

export function useTitlesStatus() {
  return useQuery({ queryKey: TITLES_STATUS_KEY, queryFn: api.titlesStatus });
}

export function useStartTitles() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (scope: TitleScope) => api.titlesStart(scope),
    onSuccess: (queue) => setQueue(queryClient, queue),
    onError: (error) => showToast(toAppError(error).message),
  });
}

export function usePauseTitles() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: api.titlesPause,
    onSuccess: (queue) => setQueue(queryClient, queue),
  });
}

function setQueue(queryClient: ReturnType<typeof useQueryClient>, queue: TitleQueueStatus) {
  queryClient.setQueryData<TitlesOverview>(TITLES_STATUS_KEY, (overview) =>
    overview ? { ...overview, queue } : overview,
  );
}

/**
 * Applies background title results as they arrive, so new titles appear on cards without a
 * reload. Mounted once by the app shell.
 */
export function useTitleEvents() {
  const queryClient = useQueryClient();

  useEffect(() => {
    const unlisteners = [
      listen<TitleQueueStatus>(TITLE_PROGRESS_EVENT, ({ payload }) => {
        setQueue(queryClient, payload);
        if (!payload.running) void queryClient.invalidateQueries({ queryKey: TITLES_STATUS_KEY });
      }),
      listen<TitleItemEvent>(TITLE_ITEM_EVENT, ({ payload }) => {
        const { title } = payload;
        if (!title) return;
        void queryClient.invalidateQueries({ queryKey: ['video-people', payload.driveId] });
        patchVideoInCaches(queryClient, payload.driveId, (video) => ({
          ...video,
          aiTitle: title,
          aiTitleStatus: 'done',
          ...(video.titleSource === 'custom' ? {} : { title, titleSource: 'ai' as const }),
        }));
      }),
    ];
    return () => {
      for (const unlisten of unlisteners) {
        void unlisten.then((stop) => stop());
      }
    };
  }, [queryClient]);
}
