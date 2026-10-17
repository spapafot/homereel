import { describe, expect, it } from 'vitest';

import {
  fitWithin,
  initialSelection,
  isUsableSelection,
  MIN_SELECTION_PX,
  moveSelection,
  resizeSelection,
  selectionBetween,
} from './crop-geometry';

const FRAME = { width: 1280, height: 720 };

describe('crop geometry', () => {
  it('should start with a centred square covering most of the shorter side', () => {
    expect(initialSelection(FRAME)).toEqual({ x: 424, y: 144, width: 432, height: 432 });
  });

  it('should keep a moved selection inside the image', () => {
    const rect = { x: 100, y: 100, width: 200, height: 200 };

    expect(moveSelection(rect, 50, -20, FRAME)).toEqual({ ...rect, x: 150, y: 80 });
    expect(moveSelection(rect, -500, 900, FRAME)).toEqual({ ...rect, x: 0, y: 520 });
  });

  it('should resize from the bottom-right corner within the image and above the minimum', () => {
    const rect = { x: 1000, y: 600, width: 100, height: 100 };

    expect(resizeSelection(rect, 500, 500, FRAME)).toEqual({ ...rect, width: 280, height: 120 });
    expect(resizeSelection(rect, -500, -500, FRAME)).toEqual({
      ...rect,
      width: MIN_SELECTION_PX,
      height: MIN_SELECTION_PX,
    });
  });

  it('should span a drag in any direction and clip it to the image', () => {
    expect(selectionBetween({ x: 300, y: 400 }, { x: 100, y: 50 }, FRAME)).toEqual({
      x: 100,
      y: 50,
      width: 200,
      height: 350,
    });
    expect(selectionBetween({ x: 1200, y: 700 }, { x: 1500, y: 900 }, FRAME)).toEqual({
      x: 1200,
      y: 700,
      width: 80,
      height: 20,
    });
  });

  it('should reject selections smaller than the minimum', () => {
    expect(isUsableSelection({ x: 0, y: 0, width: 200, height: 10 })).toBe(false);
    expect(isUsableSelection({ x: 0, y: 0, width: 30, height: 30 })).toBe(true);
  });

  it('should scale large crops down to the long edge and leave small ones alone', () => {
    expect(fitWithin({ width: 1024, height: 768 }, 512)).toEqual({ width: 512, height: 384 });
    expect(fitWithin({ width: 300, height: 900 }, 512)).toEqual({ width: 171, height: 512 });
    expect(fitWithin({ width: 200, height: 100 }, 512)).toEqual({ width: 200, height: 100 });
  });
});
