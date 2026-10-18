import { useSearchParams } from 'react-router';

import { LibraryGate } from '@/components/common/LibraryGate';
import { EmptyState } from '@/components/common/EmptyState';
import { ErrorState } from '@/components/common/ErrorState';
import { VideoListRow, VideoListRowSkeleton } from '@/components/video/VideoListRow';
import { useInfiniteScroll } from '@/hooks/use-infinite-scroll';
import { useVideoFeed } from '@/hooks/use-videos';

const SKELETON_COUNT = 6;

export function SearchPage() {
  const [params] = useSearchParams();
  const query = params.get('search_query')?.trim() ?? '';

  return (
    <LibraryGate>
      {query ? (
        <SearchResults key={query} query={query} />
      ) : (
        <EmptyState icon="search" title="Search your Drive videos">
          Type in the search box at the top to find videos by title or file name.
        </EmptyState>
      )}
    </LibraryGate>
  );
}

function SearchResults({ query }: { query: string }) {
  const feed = useVideoFeed({ search: query });
  const sentinelRef = useInfiniteScroll(
    () => feed.fetchNextPage(),
    feed.hasNextPage && !feed.isFetchingNextPage,
  );

  if (feed.isError) return <ErrorState error={feed.error} onRetry={() => feed.refetch()} />;
  const videos = feed.data?.pages.flatMap((page) => page.videos) ?? [];
  if (feed.isSuccess && videos.length === 0) {
    return (
      <EmptyState icon="search_off" title="No results found">
        Try different keywords. Search matches video titles and file names.
      </EmptyState>
    );
  }
  const skeletons = feed.isPending || feed.isFetchingNextPage ? SKELETON_COUNT : 0;

  return (
    <div className="mx-auto flex max-w-[1096px] flex-col gap-4 px-6 py-4">
      {videos.map((video) => (
        <VideoListRow key={video.id} video={video} />
      ))}
      {Array.from({ length: skeletons }, (_, index) => (
        <VideoListRowSkeleton key={`skeleton-${index}`} />
      ))}
      <div ref={sentinelRef} className="h-px" />
    </div>
  );
}
