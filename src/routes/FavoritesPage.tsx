import { CollectionPage } from '@/components/collection/CollectionPage';
import { useLibraryList } from '@/hooks/use-library';
import { useToggleFavorite } from '@/hooks/use-video-actions';
import { FAVORITES_LIST } from '@/lib/paths';

export function FavoritesPage() {
  const favorites = useLibraryList('favorites');
  const toggleFavorite = useToggleFavorite();

  return (
    <CollectionPage
      title="Favorites"
      listId={FAVORITES_LIST}
      videos={favorites.data}
      isPending={favorites.isPending}
      error={favorites.error}
      onRetry={() => favorites.refetch()}
      empty={{
        icon: 'favorite',
        title: 'No favorites yet',
        text: 'Use “Add to Favorites” in a video’s ⋮ menu or the heart on the watch page.',
      }}
      rowActions={(video) => [
        {
          icon: 'delete',
          label: 'Remove from Favorites',
          onSelect: () => toggleFavorite.mutate({ video, value: false }),
        },
      ]}
    />
  );
}
