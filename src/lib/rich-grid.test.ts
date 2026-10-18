import { describe, expect, it } from 'vitest';

import { richGridColumns } from './rich-grid';

describe('richGridColumns', () => {
  it('should show 3 cards per row in the grid YouTube draws at 1988px', () => {
    expect(richGridColumns(1684)).toBe(3);
  });

  it('should show 4 cards per row in a maximized Homereel window', () => {
    expect(richGridColumns(1897)).toBe(4);
  });

  it('should show 2 cards per row in a narrow window', () => {
    expect(richGridColumns(592)).toBe(2);
  });

  it('should always show at least one card', () => {
    expect(richGridColumns(200)).toBe(1);
    expect(richGridColumns(0)).toBe(1);
  });
});
