import { EmptyState } from '@/components/common/EmptyState';
import { ErrorState } from '@/components/common/ErrorState';
import { useVideoFeed } from '@/hooks/use-videos';
import type { ReactNode } from 'react';

import type { VideoListRequest } from '@/lib/api';

import { VideoGrid, type GridRowGap } from './VideoGrid';

type VideoFeedProps = {
  request: Omit<VideoListRequest, 'pageToken'>;
  emptyTitle?: string;
  /** A full-width row shown after the first rows, such as the Shorts shelf. */
  shelf?: ReactNode;
  rowGap?: GridRowGap;
};

export function VideoFeed({
  request,
  emptyTitle = 'No videos here yet',
  shelf,
  rowGap,
}: VideoFeedProps) {
  const feed = useVideoFeed(request);

  if (feed.isError) return <ErrorState error={feed.error} onRetry={() => feed.refetch()} />;
  const videos = feed.data?.pages.flatMap((page) => page.videos) ?? [];
  if (feed.isSuccess && videos.length === 0) {
    return <EmptyState icon="video_library" title={emptyTitle} />;
  }
  return (
    <VideoGrid
      videos={videos}
      isLoading={feed.isPending}
      hasMore={feed.hasNextPage}
      isLoadingMore={feed.isFetchingNextPage}
      onLoadMore={() => feed.fetchNextPage()}
      shelf={shelf}
      rowGap={rowGap}
    />
  );
}
