import { useCallback, useLayoutEffect, useRef, useState } from 'react';

import { Icon } from '@/components/common/Icon';
import { useShortsRow } from '@/hooks/use-videos';
import { cx } from '@/lib/class-names';

import { ShortCard } from './ShortCard';

type ShortsRowProps = {
  /** Shorts from this folder, or from all of Drive. */
  folderId?: string;
  /** The video playing on the watch page, left out of the row. */
  currentId: string;
};

type ScrollState = { back: boolean; forward: boolean };

/** The row of Shorts YouTube shows above related videos: three at a time, scrolled sideways. */
export function ShortsRow({ folderId, currentId }: ShortsRowProps) {
  const shorts = useShortsRow(folderId, currentId);
  const rowRef = useRef<HTMLDivElement>(null);
  const [canScroll, setCanScroll] = useState<ScrollState>({ back: false, forward: false });

  const updateArrows = useCallback(() => {
    const row = rowRef.current;
    if (!row) return;
    setCanScroll({
      back: row.scrollLeft > 0,
      forward: row.scrollLeft + row.clientWidth < row.scrollWidth - 1,
    });
  }, []);

  useLayoutEffect(updateArrows, [updateArrows, shorts.length]);

  const page = (direction: 1 | -1) => {
    const row = rowRef.current;
    row?.scrollBy({ left: direction * row.clientWidth, behavior: 'smooth' });
  };

  if (shorts.length === 0) return null;

  return (
    <section aria-label="Shorts" className="relative">
      <div
        ref={rowRef}
        onScroll={updateArrows}
        className="scrollbar-none flex snap-x snap-mandatory gap-1 overflow-x-auto"
      >
        {shorts.map((video) => (
          <div key={video.id} className="w-[calc((100%-8px)/3)] max-w-[231px] shrink-0 snap-start">
            <ShortCard video={video} folderId={folderId} compact />
          </div>
        ))}
      </div>
      {/* Three 2:3 thumbnails side by side are 2:1, so the arrows sit at their middle. Cards stop
          growing at a third of YouTube's widest related column (700px), as does this height. */}
      <div className="pointer-events-none absolute inset-x-0 top-0 flex aspect-[2/1] max-h-[347px] items-center justify-between">
        <ScrollButton
          icon="chevron_left"
          label="Previous Shorts"
          hidden={!canScroll.back}
          onClick={() => page(-1)}
        />
        <ScrollButton
          icon="chevron_right"
          label="More Shorts"
          hidden={!canScroll.forward}
          onClick={() => page(1)}
        />
      </div>
    </section>
  );
}

function ScrollButton({
  icon,
  label,
  hidden,
  onClick,
}: {
  icon: string;
  label: string;
  hidden: boolean;
  onClick: () => void;
}) {
  return (
    <button
      type="button"
      aria-label={label}
      title={label}
      onClick={onClick}
      tabIndex={hidden ? -1 : undefined}
      className={cx(
        'pointer-events-auto flex size-10 items-center justify-center rounded-full bg-page shadow-md outline-none hover:bg-chip focus-visible:ring-2 focus-visible:ring-focus',
        hidden && 'invisible',
      )}
    >
      <Icon name={icon} />
    </button>
  );
}
