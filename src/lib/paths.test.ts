import { describe, expect, it } from 'vitest';

import { playlistPath, searchPath, shortsPath, watchPath } from './paths';

describe('paths', () => {
  it('should build YouTube-style watch URLs with folder or list context', () => {
    expect(watchPath('abc')).toBe('/watch?v=abc');
    expect(watchPath('abc', { folderId: 'f1' })).toBe('/watch?v=abc&folder=f1');
    expect(watchPath('abc', { folderId: 'f1', list: 'WL' })).toBe('/watch?v=abc&list=WL');
  });

  it('should build YouTube-style Shorts URLs with an optional folder', () => {
    expect(shortsPath('abc')).toBe('/shorts/abc');
    expect(shortsPath('abc', 'f1')).toBe('/shorts/abc?folder=f1');
    expect(shortsPath('abc', null)).toBe('/shorts/abc');
  });

  it('should encode playlist and search parameters', () => {
    expect(playlistPath(7)).toBe('/playlist?list=7');
    expect(searchPath('beach & sun')).toBe('/results?search_query=beach+%26+sun');
  });
});
