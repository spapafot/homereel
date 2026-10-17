import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { invoke } from '@tauri-apps/api/core';
import { renderHook, waitFor } from '@testing-library/react';
import type { ReactNode } from 'react';
import { afterEach, describe, expect, it, vi } from 'vitest';

import type { VideoListRequest, VideoPage } from '@/lib/api';
import { makeVideo } from '@/test/fixtures';

import { useNextInFeed, useVideoFeed } from './use-videos';

vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn(), convertFileSrc: vi.fn() }));

const PAGES: Record<string, VideoPage> = {
  first: { videos: [makeVideo({ id: 'a' }), makeVideo({ id: 'b' })], nextPageToken: 'p2' },
  p2: { videos: [makeVideo({ id: 'c' }), makeVideo({ id: 'd' })], nextPageToken: 'p3' },
  p3: { videos: [makeVideo({ id: 'e' })], nextPageToken: null },
};

function wrapper({ children }: { children: ReactNode }) {
  const queryClient = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  return <QueryClientProvider client={queryClient}>{children}</QueryClientProvider>;
}

function useFolderNext(videoId: string, enabled = true) {
  const feed = useVideoFeed({ folderId: 'fold1' });
  return { next: useNextInFeed(feed, videoId, { enabled }), feed };
}

describe('useNextInFeed', () => {
  afterEach(() => vi.mocked(invoke).mockReset());

  function servePages() {
    vi.mocked(invoke).mockImplementation(async (_command, args) => {
      const { request } = args as { request: VideoListRequest };
      return PAGES[request.pageToken ?? 'first'];
    });
  }

  it('should load later pages until it finds the video after a deep one', async () => {
    servePages();

    const { result } = renderHook(() => useFolderNext('d'), { wrapper });

    await waitFor(() => expect(result.current.next?.id).toBe('e'));
    expect(invoke).toHaveBeenCalledTimes(3);
  });

  it('should stop at the last page when the video is last', async () => {
    servePages();

    const { result } = renderHook(() => useFolderNext('e'), { wrapper });

    await waitFor(() => expect(result.current.feed.hasNextPage).toBe(false));
    expect(result.current.next).toBeNull();
  });

  it('should not load more pages while disabled', async () => {
    servePages();

    const { result } = renderHook(() => useFolderNext('d', false), { wrapper });

    await waitFor(() => expect(result.current.feed.isSuccess).toBe(true));
    expect(invoke).toHaveBeenCalledTimes(1);
  });
});
