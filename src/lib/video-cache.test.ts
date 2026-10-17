import { QueryClient, type InfiniteData } from '@tanstack/react-query';
import { describe, expect, it } from 'vitest';

import { makeVideo } from '@/test/fixtures';

import type { VideoPage, VideoSummary } from './api';
import { patchVideoInCaches } from './video-cache';

describe('patchVideoInCaches', () => {
  it('should update the video in feeds, the watch page and local lists', () => {
    const queryClient = new QueryClient();
    const target = makeVideo({ id: 'a' });
    const other = makeVideo({ id: 'b' });
    queryClient.setQueryData<InfiniteData<VideoPage>>(['videos', { order: 'createdNewest' }], {
      pages: [{ videos: [target, other], nextPageToken: null }],
      pageParams: [undefined],
    });
    queryClient.setQueryData<VideoSummary>(['video', 'a'], target);
    queryClient.setQueryData<VideoSummary[]>(['library', 'favorites'], [other, target]);

    patchVideoInCaches(queryClient, 'a', { isFavorite: true });

    const feed = queryClient.getQueryData<InfiniteData<VideoPage>>([
      'videos',
      { order: 'createdNewest' },
    ]);
    expect(feed?.pages[0].videos.map((video) => video.isFavorite)).toEqual([true, false]);
    expect(queryClient.getQueryData<VideoSummary>(['video', 'a'])?.isFavorite).toBe(true);
    expect(queryClient.getQueryData<VideoSummary[]>(['library', 'favorites'])?.[1].isFavorite).toBe(
      true,
    );
  });

  it('should update the video in the paged watch history', () => {
    const queryClient = new QueryClient();
    queryClient.setQueryData<InfiniteData<VideoSummary[]>>(['history'], {
      pages: [[makeVideo({ id: 'b' })], [makeVideo({ id: 'a' })]],
      pageParams: [0, 48],
    });

    patchVideoInCaches(queryClient, 'a', { recordedDate: '2019-08-13' });

    const history = queryClient.getQueryData<InfiniteData<VideoSummary[]>>(['history']);
    expect(history?.pages[1][0].recordedDate).toBe('2019-08-13');
    expect(history?.pages[0][0].recordedDate).toBeNull();
  });
});
