import { describe, expect, it } from 'vitest';

import { formatDuration } from './format-duration';

describe('formatDuration', () => {
  it('should format short videos as minutes and padded seconds', () => {
    expect(formatDuration(245_000)).toBe('4:05');
    expect(formatDuration(754_000)).toBe('12:34');
  });

  it('should include hours with padded minutes for long videos', () => {
    expect(formatDuration(3_723_000)).toBe('1:02:03');
  });

  it('should truncate partial seconds and clamp negative values', () => {
    expect(formatDuration(59_999)).toBe('0:59');
    expect(formatDuration(-5)).toBe('0:00');
  });
});
