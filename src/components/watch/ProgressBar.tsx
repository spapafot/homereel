import { useRef, useState, type PointerEvent } from 'react';

import { formatDuration } from '@/lib/format-duration';

type ProgressBarProps = {
  currentTime: number;
  duration: number;
  bufferedEnd: number;
  onSeek: (time: number) => void;
};

type HoverInfo = { time: number; offsetPx: number };

function percent(value: number, total: number): string {
  return total > 0 ? `${Math.min(100, (value / total) * 100)}%` : '0%';
}

/** YouTube's red scrubber: thickens on hover, shows the time under the pointer, drag to seek. */
export function ProgressBar({ currentTime, duration, bufferedEnd, onSeek }: ProgressBarProps) {
  const barRef = useRef<HTMLDivElement>(null);
  const [dragTime, setDragTime] = useState<number | null>(null);
  const [hover, setHover] = useState<HoverInfo | null>(null);

  const pointerInfo = (event: PointerEvent): HoverInfo | null => {
    const bar = barRef.current;
    if (!bar || duration <= 0) return null;
    const rect = bar.getBoundingClientRect();
    const offsetPx = Math.min(Math.max(event.clientX - rect.left, 0), rect.width);
    return { time: (offsetPx / rect.width) * duration, offsetPx };
  };

  const shownTime = dragTime ?? currentTime;

  return (
    <div
      ref={barRef}
      role="slider"
      aria-label="Seek slider"
      aria-valuemin={0}
      aria-valuemax={Math.round(duration)}
      aria-valuenow={Math.round(shownTime)}
      aria-valuetext={`${formatDuration(shownTime * 1000)} of ${formatDuration(duration * 1000)}`}
      className="group/progress relative flex h-4 cursor-pointer touch-none items-center"
      onPointerDown={(event) => {
        const info = pointerInfo(event);
        if (!info) return;
        event.currentTarget.setPointerCapture(event.pointerId);
        setDragTime(info.time);
      }}
      onPointerMove={(event) => {
        const info = pointerInfo(event);
        setHover(info);
        if (info && dragTime != null) setDragTime(info.time);
      }}
      onPointerUp={() => {
        if (dragTime != null) onSeek(dragTime);
        setDragTime(null);
      }}
      onPointerLeave={() => setHover(null)}
    >
      <div className="relative h-[3px] w-full bg-white/20 transition-[height] duration-100 group-hover/progress:h-[5px]">
        <div
          className="absolute inset-y-0 left-0 bg-white/40"
          style={{ width: percent(bufferedEnd, duration) }}
        />
        <div
          className="absolute inset-y-0 left-0 bg-progress"
          style={{ width: percent(shownTime, duration) }}
        />
      </div>
      <div
        className="absolute size-3.5 -translate-x-1/2 scale-0 rounded-full bg-progress transition-transform duration-100 group-hover/progress:scale-100"
        style={{ left: percent(shownTime, duration) }}
      />
      {hover && (
        <div
          className="pointer-events-none absolute bottom-6 -translate-x-1/2 rounded bg-black/80 px-2 py-0.5 text-xs font-medium text-white"
          style={{ left: hover.offsetPx }}
        >
          {formatDuration(hover.time * 1000)}
        </div>
      )}
    </div>
  );
}
