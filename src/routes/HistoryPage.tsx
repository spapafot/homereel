import { useState } from 'react';

import { Button } from '@/components/common/Button';
import { EmptyState } from '@/components/common/EmptyState';
import { ErrorState } from '@/components/common/ErrorState';
import { ConfirmDialog } from '@/components/common/Modal';
import { VideoListRow, VideoListRowSkeleton } from '@/components/video/VideoListRow';
import { useInfiniteScroll } from '@/hooks/use-infinite-scroll';
import { useClearHistory, useHistoryFeed, useRemoveFromHistory } from '@/hooks/use-library';
import type { VideoSummary } from '@/lib/api';
import { cx } from '@/lib/class-names';
import { groupHistoryByDay } from '@/lib/history-groups';

const SKELETON_ROWS = 4;

export function HistoryPage() {
  const history = useHistoryFeed();
  const remove = useRemoveFromHistory();
  const clear = useClearHistory();
  const [confirmOpen, setConfirmOpen] = useState(false);
  const sentinelRef = useInfiniteScroll(
    () => history.fetchNextPage(),
    history.hasNextPage && !history.isFetchingNextPage,
  );
  const videos = uniqueVideos(history.data?.pages.flat() ?? []);
  const hasHistory = videos.length > 0;

  const clearButton = (
    <Button icon="delete" disabled={!hasHistory} onClick={() => setConfirmOpen(true)}>
      Clear all watch history
    </Button>
  );

  return (
    <div className="mx-auto flex max-w-[1284px] gap-10 px-6 py-6">
      <div className="max-w-[852px] min-w-0 flex-1">
        <h1 className="text-4xl font-bold">Watch history</h1>
        <div className="mt-4 xl:hidden">{clearButton}</div>
        {history.isError && <ErrorState error={history.error} onRetry={() => history.refetch()} />}
        {history.isPending && <RowSkeletons className="mt-8" />}
        {history.isSuccess && !hasHistory && (
          <EmptyState icon="history" title="This list has no videos">
            Videos you watch in Homereel will show up here.
          </EmptyState>
        )}
        {groupHistoryByDay(videos).map((group) => (
          <section key={group.label}>
            <h2 className="mt-8 mb-4 text-xl font-bold">{group.label}</h2>
            <div className="flex flex-col gap-4">
              {group.videos.map((video) => (
                <VideoListRow
                  key={video.id}
                  video={video}
                  onRemove={{
                    label: 'Remove from watch history',
                    action: () => remove.mutate(video.id),
                  }}
                />
              ))}
            </div>
          </section>
        ))}
        {history.isFetchingNextPage && <RowSkeletons className="mt-4" />}
        <div ref={sentinelRef} className="h-px" />
      </div>
      <aside className="hidden w-[300px] shrink-0 pt-20 xl:block">
        {clearButton}
        <p className="mt-3 text-xs leading-[18px] text-text-secondary">
          Watch history and resume points are stored only on this computer.
        </p>
      </aside>
      <ConfirmDialog
        open={confirmOpen}
        onOpenChange={setConfirmOpen}
        title="Clear watch history?"
        description="Your Homereel watch history and resume points will be cleared. Your videos are not affected."
        confirmLabel="Clear watch history"
        onConfirm={() => clear.mutate()}
      />
    </div>
  );
}

/** A video watched while scrolling moves to the top and could appear on two pages. */
function uniqueVideos(videos: VideoSummary[]): VideoSummary[] {
  const seen = new Set<string>();
  return videos.filter((video) => !seen.has(video.id) && seen.add(video.id));
}

function RowSkeletons({ className }: { className: string }) {
  return (
    <div className={cx('flex flex-col gap-4', className)}>
      {Array.from({ length: SKELETON_ROWS }, (_, index) => (
        <VideoListRowSkeleton key={index} />
      ))}
    </div>
  );
}
