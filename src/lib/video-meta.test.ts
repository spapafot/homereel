import { describe, expect, it } from 'vitest';

import { makeVideo } from '@/test/fixtures';

import { dateCreated, relativeDateCreated, videoDetailsParts } from './video-meta';

const NOW = new Date('2026-10-17T12:00:00Z');

describe('relativeDateCreated', () => {
  it('should show the recording date, not the upload date', () => {
    const video = makeVideo({
      recordedDate: '2017-07-09',
      modifiedTime: '2026-06-16T10:00:00Z',
      createdTime: '2026-06-17T10:00:00Z',
    });

    expect(relativeDateCreated(video, NOW)).toBe('9 years ago');
  });

  it('should fall back to the modified date until the recording date is read', () => {
    const video = makeVideo({
      recordedDate: null,
      modifiedTime: '2025-03-06T10:00:00Z',
      createdTime: '2026-06-17T10:00:00Z',
    });

    expect(relativeDateCreated(video, NOW)).toBe('1 year ago');
  });

  it('should return null when no date is known', () => {
    const video = makeVideo({ recordedDate: null, modifiedTime: null });

    expect(relativeDateCreated(video, NOW)).toBeNull();
  });
});

describe('videoDetailsParts', () => {
  it('should list the resolution and the file size', () => {
    const video = makeVideo({ width: 1920, height: 1080, sizeBytes: 100_034_150 });

    expect(videoDetailsParts(video)).toEqual(['1920x1080', '95.4 MB']);
  });

  it('should leave out what Drive does not know', () => {
    const video = makeVideo({ width: null, height: null, sizeBytes: null });

    expect(videoDetailsParts(video)).toEqual([]);
  });
});

describe('dateCreated', () => {
  it('should prefer the recording date over Drive dates', () => {
    expect(dateCreated(makeVideo({ recordedDate: '2019-08-13' }))).toBe('2019-08-13');
  });
});
