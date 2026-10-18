import { describe, expect, it, vi } from 'vitest';

import { thumbnailUrl } from './api';
import { bumpThumbnailVersion, getThumbnailVersion } from './thumbnail-versions';

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
  convertFileSrc: (path: string, protocol: string) => `http://${protocol}.localhost/${path}`,
}));

describe('thumbnail versions', () => {
  it('should give a video a new thumbnail URL each time its preview is regenerated', () => {
    expect(thumbnailUrl('a1', getThumbnailVersion('a1'))).toBe('http://thumb.localhost/a1');

    bumpThumbnailVersion('a1');
    expect(thumbnailUrl('a1', getThumbnailVersion('a1'))).toBe('http://thumb.localhost/a1?v=1');

    bumpThumbnailVersion('a1');
    expect(thumbnailUrl('a1', getThumbnailVersion('a1'))).toBe('http://thumb.localhost/a1?v=2');
  });

  it('should leave other videos on their original URL', () => {
    bumpThumbnailVersion('b1');

    expect(thumbnailUrl('b2', getThumbnailVersion('b2'))).toBe('http://thumb.localhost/b2');
  });
});
