import { useState } from 'react';
import { useNavigate } from 'react-router';

import { PlaylistCard } from '@/components/collection/PlaylistCard';
import { Button } from '@/components/common/Button';
import { ErrorState } from '@/components/common/ErrorState';
import { PromptDialog } from '@/components/common/Modal';
import { useCreatePlaylist, useLibraryList, usePlaylists } from '@/hooks/use-library';
import { FAVORITES_LIST, playlistPath, WATCH_LATER_LIST } from '@/lib/paths';

export function PlaylistsPage() {
  const navigate = useNavigate();
  const playlists = usePlaylists();
  const watchLater = useLibraryList('watchLater');
  const favorites = useLibraryList('favorites');
  const create = useCreatePlaylist();
  const [creating, setCreating] = useState(false);

  return (
    <div className="px-6 py-6">
      <div className="flex flex-wrap items-center justify-between gap-4">
        <h1 className="text-4xl font-bold">Playlists</h1>
        <Button icon="add" onClick={() => setCreating(true)}>
          New playlist
        </Button>
      </div>
      {playlists.isError && (
        <ErrorState error={playlists.error} onRetry={() => playlists.refetch()} />
      )}
      <div className="mt-6 grid grid-cols-[repeat(auto-fill,minmax(240px,1fr))] gap-x-4 gap-y-8">
        <PlaylistCard
          title="Watch later"
          videoCount={watchLater.data?.length ?? 0}
          href={playlistPath(WATCH_LATER_LIST)}
          coverVideoId={watchLater.data?.[0]?.id ?? null}
        />
        <PlaylistCard
          title="Favorites"
          videoCount={favorites.data?.length ?? 0}
          href={playlistPath(FAVORITES_LIST)}
          coverVideoId={favorites.data?.[0]?.id ?? null}
        />
        {playlists.data?.map((playlist) => (
          <PlaylistCard
            key={playlist.id}
            title={playlist.name}
            videoCount={playlist.videoCount}
            href={playlistPath(playlist.id)}
            coverVideoId={playlist.firstVideoId}
          />
        ))}
      </div>
      <PromptDialog
        open={creating}
        onOpenChange={setCreating}
        title="New playlist"
        label="Name"
        confirmLabel="Create"
        description="Playlists are stored only on this computer."
        onConfirm={async (name) => {
          const playlist = await create.mutateAsync({ name });
          navigate(playlistPath(playlist.id));
        }}
      />
    </div>
  );
}
