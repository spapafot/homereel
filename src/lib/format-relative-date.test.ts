import { describe, expect, it } from 'vitest';

import { formatRelativeDate } from './format-relative-date';

const NOW = new Date('2026-10-17T12:00:00Z');

describe('formatRelativeDate', () => {
  it('should use the largest whole unit with singular and plural forms', () => {
    expect(formatRelativeDate('2026-10-14T12:00:00Z', NOW)).toBe('3 days ago');
    expect(formatRelativeDate('2026-10-16T11:00:00Z', NOW)).toBe('1 day ago');
    expect(formatRelativeDate('2026-10-03T12:00:00Z', NOW)).toBe('2 weeks ago');
    expect(formatRelativeDate('2026-08-10T12:00:00Z', NOW)).toBe('2 months ago');
    expect(formatRelativeDate('2024-06-12T18:14:55Z', NOW)).toBe('2 years ago');
  });

  it('should say just now for times under a minute', () => {
    expect(formatRelativeDate('2026-10-17T11:59:30Z', NOW)).toBe('just now');
  });

  it('should return an empty string for invalid dates', () => {
    expect(formatRelativeDate('not a date', NOW)).toBe('');
  });
});
