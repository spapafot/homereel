import { describe, expect, it } from 'vitest';

import { makeVideo } from '@/test/fixtures';

import { pickNextVideo } from './up-next';

describe('pickNextVideo', () => {
  const list = [makeVideo({ id: 'a' }), makeVideo({ id: 'b' }), makeVideo({ id: 'c' })];

  it('should return the video after the current one', () => {
    expect(pickNextVideo(list, 'b')?.id).toBe('c');
  });

  it('should return null after the last video', () => {
    expect(pickNextVideo(list, 'c')).toBeNull();
  });

  it('should return null when the current video is not in the list', () => {
    expect(pickNextVideo(list, 'z')).toBeNull();
  });
});
