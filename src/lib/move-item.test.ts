import { describe, expect, it } from 'vitest';

import { moveItem } from './move-item';

describe('moveItem', () => {
  it('should move an item up or down without changing the input', () => {
    const items = ['a', 'b', 'c', 'd'];

    expect(moveItem(items, 2, 1)).toEqual(['a', 'c', 'b', 'd']);
    expect(moveItem(items, 0, 3)).toEqual(['b', 'c', 'd', 'a']);
    expect(items).toEqual(['a', 'b', 'c', 'd']);
  });

  it('should ignore positions outside the list', () => {
    expect(moveItem(['a', 'b'], 1, 2)).toEqual(['a', 'b']);
  });
});
