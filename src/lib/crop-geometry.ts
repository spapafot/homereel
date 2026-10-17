/** Crop selection maths for the photo cropper. All values are pixels of the source image. */

export type Point = { x: number; y: number };
export type Size = { width: number; height: number };
export type Rect = Point & Size;

/** Smallest selection, so a stray click cannot produce an empty crop. */
export const MIN_SELECTION_PX = 24;
/** Share of the shorter side covered by the starting selection. */
const INITIAL_FRACTION = 0.6;

function clamp(value: number, min: number, max: number): number {
  return Math.min(Math.max(value, min), max);
}

/** A centred square, so the cropper starts with something usable. */
export function initialSelection(bounds: Size): Rect {
  const side = Math.round(Math.min(bounds.width, bounds.height) * INITIAL_FRACTION);
  return {
    x: Math.round((bounds.width - side) / 2),
    y: Math.round((bounds.height - side) / 2),
    width: side,
    height: side,
  };
}

/** Moves the selection, stopping at the image edges. */
export function moveSelection(rect: Rect, dx: number, dy: number, bounds: Size): Rect {
  return {
    ...rect,
    x: clamp(rect.x + dx, 0, bounds.width - rect.width),
    y: clamp(rect.y + dy, 0, bounds.height - rect.height),
  };
}

/** Drags the bottom-right corner. */
export function resizeSelection(rect: Rect, dx: number, dy: number, bounds: Size): Rect {
  return {
    ...rect,
    width: clamp(
      rect.width + dx,
      Math.min(MIN_SELECTION_PX, bounds.width - rect.x),
      bounds.width - rect.x,
    ),
    height: clamp(
      rect.height + dy,
      Math.min(MIN_SELECTION_PX, bounds.height - rect.y),
      bounds.height - rect.y,
    ),
  };
}

/** The box spanned by dragging from `start` to `end`, in any direction, inside the image. */
export function selectionBetween(start: Point, end: Point, bounds: Size): Rect {
  const left = clamp(Math.min(start.x, end.x), 0, bounds.width);
  const top = clamp(Math.min(start.y, end.y), 0, bounds.height);
  const right = clamp(Math.max(start.x, end.x), 0, bounds.width);
  const bottom = clamp(Math.max(start.y, end.y), 0, bounds.height);
  return { x: left, y: top, width: right - left, height: bottom - top };
}

export function isUsableSelection(rect: Rect): boolean {
  return rect.width >= MIN_SELECTION_PX && rect.height >= MIN_SELECTION_PX;
}

/** Scales `size` down so its long edge is at most `maxEdge`; never scales up. */
export function fitWithin(size: Size, maxEdge: number): Size {
  const scale = Math.min(1, maxEdge / Math.max(size.width, size.height));
  return {
    width: Math.max(1, Math.round(size.width * scale)),
    height: Math.max(1, Math.round(size.height * scale)),
  };
}
