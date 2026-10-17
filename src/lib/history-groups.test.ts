import { describe, expect, it } from 'vitest';

import { makeVideo } from '@/test/fixtures';

import { dayLabel, groupHistoryByDay } from './history-groups';

// Local times, so the tests do not depend on the machine's time zone.
const NOW = new Date(2026, 9, 17, 15, 0);

describe('dayLabel', () => {
  it('should use Today, Yesterday and weekday names for the last week', () => {
    expect(dayLabel(new Date(2026, 9, 17, 9, 0).toISOString(), NOW)).toBe('Today');
    expect(dayLabel(new Date(2026, 9, 16, 23, 30).toISOString(), NOW)).toBe('Yesterday');
    expect(dayLabel(new Date(2026, 9, 13, 12, 0).toISOString(), NOW)).toBe('Tuesday');
  });

  it('should use the date for older days, with the year only when it differs', () => {
    expect(dayLabel(new Date(2026, 8, 3, 12, 0).toISOString(), NOW)).toBe('Sep 3');
    expect(dayLabel(new Date(2025, 11, 24, 12, 0).toISOString(), NOW)).toBe('Dec 24, 2025');
  });
});

describe('groupHistoryByDay', () => {
  it('should group consecutive videos under one heading in order', () => {
    const videos = [
      makeVideo({ id: 'a', lastWatchedAt: new Date(2026, 9, 17, 14, 0).toISOString() }),
      makeVideo({ id: 'b', lastWatchedAt: new Date(2026, 9, 17, 8, 0).toISOString() }),
      makeVideo({ id: 'c', lastWatchedAt: new Date(2026, 9, 16, 20, 0).toISOString() }),
      makeVideo({ id: 'd', lastWatchedAt: null }),
    ];

    const groups = groupHistoryByDay(videos, NOW);

    expect(groups.map((group) => [group.label, group.videos.map((video) => video.id)])).toEqual([
      ['Today', ['a', 'b']],
      ['Yesterday', ['c']],
    ]);
  });
});
