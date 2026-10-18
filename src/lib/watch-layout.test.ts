import { describe, expect, it } from 'vitest';

import { watchColumns } from './watch-layout';

describe('watchColumns', () => {
  it('should come within a few pixels of the column YouTube shows in a 1972px window', () => {
    // Measured on youtube.com: a 538px related column beside a 1386px player.
    const columns = watchColumns(1972, 1000);

    expect(columns.twoColumns).toBe(true);
    expect(columns.secondaryWidth).toBe(542);
    expect(1972 - 3 * 16 - columns.secondaryWidth).toBe(1382);
  });

  it('should give the related column 27.5% of a maximized window', () => {
    const columns = watchColumns(2187, 1164);

    expect(columns.secondaryWidth).toBe(601);
    expect(columns.maxPrimaryWidth).toBe(1840);
  });

  it('should keep the related column at least 320px wide', () => {
    expect(watchColumns(1100, 1400).secondaryWidth).toBe(320);
  });

  it('should cap the related column at 35% of a wide, short window', () => {
    const columns = watchColumns(1600, 600);

    expect(columns.secondaryWidth).toBe(560);
    expect(columns.maxPrimaryWidth).toBe(860);
  });

  it('should never let the player grow past 1840px', () => {
    expect(watchColumns(4000, 2400).maxPrimaryWidth).toBe(1840);
  });

  it('should move the related videos under the player below 1000px', () => {
    const columns = watchColumns(999, 900);

    expect(columns.twoColumns).toBe(false);
    expect(columns.secondaryWidth).toBe(0);
  });
});
