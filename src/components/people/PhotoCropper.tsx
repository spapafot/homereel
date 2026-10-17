import {
  useRef,
  useState,
  type KeyboardEvent,
  type PointerEvent as ReactPointerEvent,
} from 'react';

import { Button } from '@/components/common/Button';
import {
  fitWithin,
  initialSelection,
  isUsableSelection,
  moveSelection,
  resizeSelection,
  selectionBetween,
  type Point,
  type Rect,
  type Size,
} from '@/lib/crop-geometry';

/** Long edge of saved photos. Gemini downsamples images anyway; this keeps faces sharp. */
const PHOTO_MAX_EDGE = 512;
const JPEG_QUALITY = 0.9;
const MAX_DISPLAY_HEIGHT_VH = 55;
const KEY_STEP_PX = 10;
const KEY_STEP_LARGE_PX = 50;

type DragMode = 'draw' | 'move' | 'resize';

type Drag = {
  mode: DragMode;
  start: Point;
  startRect: Rect;
  /** Source pixels per screen pixel. */
  scale: number;
  originX: number;
  originY: number;
};

function cropToJpeg(image: HTMLImageElement, rect: Rect): string {
  const size = fitWithin(rect, PHOTO_MAX_EDGE);
  const canvas = document.createElement('canvas');
  canvas.width = size.width;
  canvas.height = size.height;
  const context = canvas.getContext('2d');
  if (!context) throw new Error('Could not crop the photo.');
  context.drawImage(
    image,
    Math.round(rect.x),
    Math.round(rect.y),
    Math.round(rect.width),
    Math.round(rect.height),
    0,
    0,
    size.width,
    size.height,
  );
  return canvas.toDataURL('image/jpeg', JPEG_QUALITY);
}

function percent(value: number, total: number): string {
  return `${(value / total) * 100}%`;
}

type PhotoCropperProps = {
  /** An object URL or data URL; same-origin, so the canvas can read it. */
  src: string;
  onCancel: () => void;
  /** Receives the crop as a JPEG data URL. */
  onCrop: (dataUrl: string) => void;
  cancelLabel?: string;
};

/** Drag a box around one person or pet. Drag inside the box to move it, or its corner to resize. */
export function PhotoCropper({ src, onCancel, onCrop, cancelLabel = 'Cancel' }: PhotoCropperProps) {
  const frameRef = useRef<HTMLDivElement>(null);
  const imageRef = useRef<HTMLImageElement>(null);
  const dragRef = useRef<Drag | null>(null);
  const [natural, setNatural] = useState<Size | null>(null);
  const [selection, setSelection] = useState<Rect | null>(null);
  const [error, setError] = useState<string | null>(null);

  const startDrag = (event: ReactPointerEvent, mode: DragMode) => {
    const frame = frameRef.current;
    if (!frame || !natural || !selection || event.button !== 0) return;
    event.preventDefault();
    event.stopPropagation();
    const bounds = frame.getBoundingClientRect();
    const scale = natural.width / bounds.width;
    const start = {
      x: (event.clientX - bounds.left) * scale,
      y: (event.clientY - bounds.top) * scale,
    };
    dragRef.current = {
      mode,
      start,
      startRect: selection,
      scale,
      originX: bounds.left,
      originY: bounds.top,
    };
    frame.setPointerCapture?.(event.pointerId);
  };

  const continueDrag = (event: ReactPointerEvent) => {
    const drag = dragRef.current;
    if (!drag || !natural) return;
    const point = {
      x: (event.clientX - drag.originX) * drag.scale,
      y: (event.clientY - drag.originY) * drag.scale,
    };
    const dx = point.x - drag.start.x;
    const dy = point.y - drag.start.y;
    if (drag.mode === 'move') setSelection(moveSelection(drag.startRect, dx, dy, natural));
    else if (drag.mode === 'resize') setSelection(resizeSelection(drag.startRect, dx, dy, natural));
    else setSelection(selectionBetween(drag.start, point, natural));
  };

  const endDrag = () => {
    const drag = dragRef.current;
    dragRef.current = null;
    // A click without a drag would leave a sliver; keep the previous box instead.
    if (drag?.mode === 'draw' && selection && !isUsableSelection(selection)) {
      setSelection(drag.startRect);
    }
  };

  const nudge = (event: KeyboardEvent) => {
    if (!natural || !selection) return;
    const step = event.shiftKey ? KEY_STEP_LARGE_PX : KEY_STEP_PX;
    const offsets: Record<string, [number, number]> = {
      ArrowLeft: [-step, 0],
      ArrowRight: [step, 0],
      ArrowUp: [0, -step],
      ArrowDown: [0, step],
    };
    const offset = offsets[event.key];
    if (!offset) return;
    event.preventDefault();
    setSelection(moveSelection(selection, offset[0], offset[1], natural));
  };

  const confirm = () => {
    const image = imageRef.current;
    if (!image || !selection) return;
    try {
      onCrop(cropToJpeg(image, selection));
    } catch {
      setError('Could not crop the photo. Try another one.');
    }
  };

  const aspect = natural ? natural.width / natural.height : 1;

  return (
    <div className="mt-4">
      <p className="text-sm text-text-secondary">
        Drag a box around one person or pet. A close crop of the face works best.
      </p>
      <div className="mt-3 overflow-hidden rounded-lg bg-black">
        <div
          ref={frameRef}
          className="relative mx-auto touch-none select-none"
          style={
            natural
              ? {
                  aspectRatio: `${natural.width} / ${natural.height}`,
                  width: `min(100%, ${MAX_DISPLAY_HEIGHT_VH * aspect}vh)`,
                }
              : undefined
          }
          onPointerDown={(event) => startDrag(event, 'draw')}
          onPointerMove={continueDrag}
          onPointerUp={endDrag}
          onPointerCancel={endDrag}
        >
          <img
            ref={imageRef}
            src={src}
            alt="Photo to crop"
            draggable={false}
            onLoad={(event) => {
              const size = {
                width: event.currentTarget.naturalWidth,
                height: event.currentTarget.naturalHeight,
              };
              setNatural(size);
              setSelection(initialSelection(size));
            }}
            onError={() =>
              setError('DriveTube can’t open this image. Use a JPEG, PNG or WebP photo.')
            }
            className={natural ? 'absolute inset-0 size-full' : 'invisible max-h-[55vh] max-w-full'}
          />
          {natural && selection && (
            <div
              role="group"
              aria-label="Crop area. Use the arrow keys to move it."
              tabIndex={0}
              onKeyDown={nudge}
              onPointerDown={(event) => startDrag(event, 'move')}
              className="absolute cursor-move border-2 border-white shadow-[0_0_0_9999px_rgba(0,0,0,0.55)] outline-none focus-visible:border-focus"
              style={{
                left: percent(selection.x, natural.width),
                top: percent(selection.y, natural.height),
                width: percent(selection.width, natural.width),
                height: percent(selection.height, natural.height),
              }}
            >
              <div
                aria-hidden="true"
                onPointerDown={(event) => startDrag(event, 'resize')}
                className="absolute -right-2 -bottom-2 size-4 cursor-nwse-resize rounded-full border-2 border-black/40 bg-white"
              />
            </div>
          )}
        </div>
      </div>
      {error && <p className="mt-2 text-sm text-red-600 dark:text-red-400">{error}</p>}
      <div className="mt-6 flex justify-end gap-2">
        <Button onClick={onCancel}>{cancelLabel}</Button>
        <Button variant="primary" disabled={!selection} onClick={confirm}>
          Use photo
        </Button>
      </div>
    </div>
  );
}
