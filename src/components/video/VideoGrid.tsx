import type { ReactNode } from 'react';

import { useInfiniteScroll } from '@/hooks/use-infinite-scroll';
import { useRichGridColumns } from '@/hooks/use-rich-grid-columns';
import type { VideoSummary } from '@/lib/api';
import { cx } from '@/lib/class-names';

import { VideoCard, VideoCardSkeleton } from './VideoCard';

const SKELETON_COUNT = 12;
/**
 * The shelf follows this many cards. Dense packing moves later cards into the gap the full-width
 * shelf leaves, so it lands after two full rows at 3, 4 and 5 columns.
 */
const SHELF_AFTER = 6;

/** YouTube spaces Home's rows 36px apart and every other grid 40px. */
export type GridRowGap = 'home' | 'default';

const ROW_GAP_CLASSES: Record<GridRowGap, string> = {
  home: 'gap-y-9',
  default: 'gap-y-10',
};

type VideoGridProps = {
  videos: VideoSummary[];
  isLoading: boolean;
  hasMore: boolean;
  isLoadingMore: boolean;
  onLoadMore: () => void;
  /** A full-width row between cards, such as YouTube's Shorts shelf. */
  shelf?: ReactNode;
  rowGap?: GridRowGap;
};

/** Responsive grid with YouTube's column counts, which follow the grid's width. */
export function VideoGrid({
  videos,
  isLoading,
  hasMore,
  isLoadingMore,
  onLoadMore,
  shelf,
  rowGap = 'default',
}: VideoGridProps) {
  const sentinelRef = useInfiniteScroll(onLoadMore, hasMore && !isLoadingMore);
  const { ref: gridRef, columns } = useRichGridColumns();
  const skeletons = isLoading || isLoadingMore ? SKELETON_COUNT : 0;

  return (
    <>
      <div
        ref={gridRef}
        className={cx(
          'grid gap-x-4',
          ROW_GAP_CLASSES[rowGap],
          shelf != null && 'grid-flow-row-dense',
        )}
        style={{ gridTemplateColumns: `repeat(${columns}, minmax(0, 1fr))` }}
      >
        {videos.slice(0, SHELF_AFTER).map((video) => (
          <VideoCard key={video.id} video={video} />
        ))}
        {/* An empty shelf (nothing to show yet) takes no row. */}
        {shelf != null && <div className="col-span-full empty:hidden">{shelf}</div>}
        {videos.slice(SHELF_AFTER).map((video) => (
          <VideoCard key={video.id} video={video} />
        ))}
        {Array.from({ length: skeletons }, (_, index) => (
          <VideoCardSkeleton key={`skeleton-${index}`} />
        ))}
      </div>
      <div ref={sentinelRef} className="h-px" />
    </>
  );
}
