import { useInfiniteScroll } from '@/hooks/use-infinite-scroll';
import type { VideoSummary } from '@/lib/api';

import { VideoCard, VideoCardSkeleton } from './VideoCard';

const SKELETON_COUNT = 12;

type VideoGridProps = {
  videos: VideoSummary[];
  isLoading: boolean;
  hasMore: boolean;
  isLoadingMore: boolean;
  onLoadMore: () => void;
};

/** Responsive grid with YouTube's column counts: 3 per row on a typical desktop window. */
export function VideoGrid({
  videos,
  isLoading,
  hasMore,
  isLoadingMore,
  onLoadMore,
}: VideoGridProps) {
  const sentinelRef = useInfiniteScroll(onLoadMore, hasMore && !isLoadingMore);
  const skeletons = isLoading || isLoadingMore ? SKELETON_COUNT : 0;

  return (
    <div className="@container">
      <div className="grid grid-cols-1 gap-x-4 gap-y-10 @[560px]:grid-cols-2 @[900px]:grid-cols-3 @[2000px]:grid-cols-4 @[2600px]:grid-cols-5">
        {videos.map((video) => (
          <VideoCard key={video.id} video={video} />
        ))}
        {Array.from({ length: skeletons }, (_, index) => (
          <VideoCardSkeleton key={`skeleton-${index}`} />
        ))}
      </div>
      <div ref={sentinelRef} className="h-px" />
    </div>
  );
}
