import { useCallback, useEffect, useRef, useState } from 'react';

import { IconButton } from '@/components/common/IconButton';
import { cx } from '@/lib/class-names';

export type Chip = {
  id: string;
  label: string;
};

type ChipBarProps = {
  chips: Chip[];
  selectedId: string;
  onSelect: (id: string) => void;
  sticky?: boolean;
  /** Just the chips' height, as above the watch page's related videos; otherwise a 56px bar. */
  flush?: boolean;
};

const SCROLL_STEP_PX = 320;

/** YouTube's filter chip row, with arrow buttons when the chips overflow. */
export function ChipBar({
  chips,
  selectedId,
  onSelect,
  sticky = false,
  flush = false,
}: ChipBarProps) {
  const scrollerRef = useRef<HTMLDivElement>(null);
  const [canScrollBack, setCanScrollBack] = useState(false);
  const [canScrollForward, setCanScrollForward] = useState(false);

  const updateArrows = useCallback(() => {
    const scroller = scrollerRef.current;
    if (!scroller) return;
    setCanScrollBack(scroller.scrollLeft > 0);
    setCanScrollForward(scroller.scrollLeft + scroller.clientWidth < scroller.scrollWidth - 1);
  }, []);

  useEffect(() => {
    updateArrows();
    const scroller = scrollerRef.current;
    if (!scroller || typeof ResizeObserver === 'undefined') return;
    const observer = new ResizeObserver(updateArrows);
    observer.observe(scroller);
    return () => observer.disconnect();
  }, [chips, updateArrows]);

  const scrollBy = (distance: number) =>
    scrollerRef.current?.scrollBy({ left: distance, behavior: 'smooth' });

  return (
    <div className={cx('z-10 bg-page', sticky && 'sticky top-14')}>
      <div className={cx('relative flex items-center', flush ? 'h-8' : 'h-14')}>
        {canScrollBack && (
          <div className="absolute left-0 z-10 flex h-full items-center bg-gradient-to-r from-page from-60% to-transparent pr-8">
            <IconButton
              icon="chevron_left"
              label="Previous"
              onClick={() => scrollBy(-SCROLL_STEP_PX)}
            />
          </div>
        )}
        <div
          ref={scrollerRef}
          onScroll={updateArrows}
          className="scrollbar-none flex gap-3 overflow-x-auto"
          role="toolbar"
          aria-label="Filters"
        >
          {chips.map((chip) => {
            const isSelected = chip.id === selectedId;
            return (
              <button
                key={chip.id}
                type="button"
                aria-pressed={isSelected}
                onClick={() => onSelect(chip.id)}
                className={cx(
                  'h-8 shrink-0 rounded-lg px-3 text-sm font-medium whitespace-nowrap outline-none',
                  'focus-visible:ring-2 focus-visible:ring-focus',
                  isSelected
                    ? 'bg-chip-active text-chip-active-text'
                    : 'bg-chip hover:bg-chip-hover',
                )}
              >
                {chip.label}
              </button>
            );
          })}
        </div>
        {canScrollForward && (
          <div className="absolute right-0 z-10 flex h-full items-center bg-gradient-to-l from-page from-60% to-transparent pl-8">
            <IconButton
              icon="chevron_right"
              label="Next"
              onClick={() => scrollBy(SCROLL_STEP_PX)}
            />
          </div>
        )}
      </div>
    </div>
  );
}
