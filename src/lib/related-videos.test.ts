import { describe, expect, it } from 'vitest';

import { makeVideo } from '@/test/fixtures';

import { relatedVideos } from './related-videos';

const ids = (videos: { id: string }[]) => videos.map((video) => video.id);

describe('relatedVideos', () => {
  it('should leave out the Shorts the row above already suggests', () => {
    // Regression: the folder's Shorts showed in the row and again in the list under it.
    const feed = [
      makeVideo({ id: 'short1' }),
      makeVideo({ id: 'wide1' }),
      makeVideo({ id: 'short2' }),
    ];
    const rowShorts = [makeVideo({ id: 'short1' }), makeVideo({ id: 'short2' })];

    expect(ids(relatedVideos(feed, 'playing', rowShorts))).toEqual(['wide1']);
  });

  it('should keep Shorts the row has no room for', () => {
    const feed = [makeVideo({ id: 'short1' }), makeVideo({ id: 'short13' })];

    expect(ids(relatedVideos(feed, 'playing', [makeVideo({ id: 'short1' })]))).toEqual(['short13']);
  });

  it('should leave out the playing video', () => {
    const feed = [makeVideo({ id: 'playing' }), makeVideo({ id: 'next' })];

    expect(ids(relatedVideos(feed, 'playing', []))).toEqual(['next']);
  });
});
