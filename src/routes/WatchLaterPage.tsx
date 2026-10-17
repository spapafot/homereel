import { CollectionPage } from '@/components/collection/CollectionPage';
import { useLibraryList } from '@/hooks/use-library';
import { useToggleWatchLater } from '@/hooks/use-video-actions';
import { WATCH_LATER_LIST } from '@/lib/paths';

export function WatchLaterPage() {
  const watchLater = useLibraryList('watchLater');
  const toggleWatchLater = useToggleWatchLater();

  return (
    <CollectionPage
      title="Watch later"
      listId={WATCH_LATER_LIST}
      videos={watchLater.data}
      isPending={watchLater.isPending}
      error={watchLater.error}
      onRetry={() => watchLater.refetch()}
      empty={{
        icon: 'schedule',
        title: 'Nothing saved for later',
        text: 'Use “Save to Watch later” in a video’s ⋮ menu to keep it here.',
      }}
      rowActions={(video) => [
        {
          icon: 'delete',
          label: 'Remove from Watch later',
          onSelect: () => toggleWatchLater.mutate({ video, value: false }),
        },
      ]}
    />
  );
}
