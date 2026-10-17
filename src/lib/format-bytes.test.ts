import { describe, expect, it } from 'vitest';

import { formatBytes } from './format-bytes';

describe('formatBytes', () => {
  it('should keep small values in bytes', () => {
    expect(formatBytes(512)).toBe('512 B');
  });

  it('should use one decimal below 100 units and none above', () => {
    expect(formatBytes(1536)).toBe('1.5 KB');
    expect(formatBytes(150 * 1024 * 1024)).toBe('150 MB');
    expect(formatBytes(2.5 * 1024 ** 3)).toBe('2.5 GB');
  });
});
