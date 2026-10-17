import { describe, expect, it } from 'vitest';

import { makeVideo } from '@/test/fixtures';

import { dateCreated, videoMetaLine } from './video-meta';

const NOW = new Date('2026-10-17T12:00:00Z');

describe('videoMetaLine', () => {
  it('should show the duration and the recording date, not the upload date', () => {
    const video = makeVideo({
      durationMs: 13_000,
      recordedDate: '2017-07-09',
      modifiedTime: '2026-06-16T10:00:00Z',
      createdTime: '2026-06-17T10:00:00Z',
    });

    expect(videoMetaLine(video, NOW)).toBe('0:13 • 9 years ago');
  });

  it('should fall back to the modified date until the recording date is read', () => {
    const video = makeVideo({
      durationMs: 13_000,
      recordedDate: null,
      modifiedTime: '2025-03-06T10:00:00Z',
      createdTime: '2026-06-17T10:00:00Z',
    });

    expect(videoMetaLine(video, NOW)).toBe('0:13 • 1 year ago');
  });

  it('should leave out the date when no date is known', () => {
    const video = makeVideo({ durationMs: 13_000, recordedDate: null, modifiedTime: null });

    expect(videoMetaLine(video, NOW)).toBe('0:13');
  });
});

describe('dateCreated', () => {
  it('should prefer the recording date over Drive dates', () => {
    expect(dateCreated(makeVideo({ recordedDate: '2019-08-13' }))).toBe('2019-08-13');
  });
});
