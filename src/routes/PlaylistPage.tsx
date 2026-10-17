import { useState } from 'react';
import { useNavigate, useSearchParams } from 'react-router';

import { CollectionPage } from '@/components/collection/CollectionPage';
import { EmptyState } from '@/components/common/EmptyState';
import { IconButton } from '@/components/common/IconButton';
import { ConfirmDialog, PromptDialog } from '@/components/common/Modal';
import type { VideoMenuAction } from '@/components/video/VideoMenu';
import {
  useDeletePlaylist,
  usePlaylist,
  useRenamePlaylist,
  useReorderPlaylist,
  useTogglePlaylistVideo,
} from '@/hooks/use-library';
import { parsePlaylistId } from '@/hooks/use-watch-list';
import { toAppError } from '@/lib/app-error';
import { moveItem } from '@/lib/move-item';
import { FAVORITES_LIST, WATCH_LATER_LIST } from '@/lib/paths';

import { FavoritesPage } from './FavoritesPage';
import { WatchLaterPage } from './WatchLaterPage';

/** `/playlist?list=…`, as on YouTube: Watch later, Favorites or one of the user's playlists. */
export function PlaylistPage() {
  const [params] = useSearchParams();
  const listId = params.get('list');
  if (listId === WATCH_LATER_LIST) return <WatchLaterPage />;
  if (listId === FAVORITES_LIST) return <FavoritesPage />;
  const playlistId = parsePlaylistId(listId);
  if (playlistId == null) return <MissingPlaylist />;
  return <UserPlaylist key={playlistId} playlistId={playlistId} />;
}

function MissingPlaylist() {
  return (
    <EmptyState icon="playlist_remove" title="This playlist doesn’t exist">
      It may have been deleted.
    </EmptyState>
  );
}

const HERO_BUTTON_CLASS = 'bg-white/15 text-white hover:bg-white/25';

function UserPlaylist({ playlistId }: { playlistId: number }) {
  const navigate = useNavigate();
  const playlist = usePlaylist(playlistId);
  const rename = useRenamePlaylist();
  const remove = useDeletePlaylist();
  const reorder = useReorderPlaylist();
  const toggle = useTogglePlaylistVideo();
  const [dialog, setDialog] = useState<'rename' | 'delete' | null>(null);

  if (playlist.isError && toAppError(playlist.error).code === 'not_found') {
    return <MissingPlaylist />;
  }
  const detail = playlist.data;
  const name = detail?.playlist.name ?? '';
  const videoIds = detail?.videos.map((video) => video.id) ?? [];

  const rowActions = (videoId: string, index: number): VideoMenuAction[] => {
    const move = (to: number) =>
      reorder.mutate({ playlistId, videoIds: moveItem(videoIds, index, to) });
    return [
      ...(index > 0
        ? [{ icon: 'vertical_align_top', label: 'Move to top', onSelect: () => move(0) }]
        : []),
      ...(index > 0
        ? [{ icon: 'arrow_upward', label: 'Move up', onSelect: () => move(index - 1) }]
        : []),
      ...(index < videoIds.length - 1
        ? [{ icon: 'arrow_downward', label: 'Move down', onSelect: () => move(index + 1) }]
        : []),
      {
        icon: 'delete',
        label: `Remove from ${name}`,
        onSelect: () => toggle.mutate({ playlistId, videoId, add: false, name }),
      },
    ];
  };

  return (
    <>
      <CollectionPage
        title={name}
        listId={String(playlistId)}
        videos={detail?.videos}
        isPending={playlist.isPending}
        error={playlist.error}
        onRetry={() => playlist.refetch()}
        empty={{
          icon: 'playlist_play',
          title: 'No videos in this playlist yet',
          text: 'Use “Save to playlist” in a video’s ⋮ menu to add videos.',
        }}
        headerActions={
          <>
            <IconButton
              icon="edit"
              label="Rename playlist"
              className={HERO_BUTTON_CLASS}
              onClick={() => setDialog('rename')}
            />
            <IconButton
              icon="delete"
              label="Delete playlist"
              className={HERO_BUTTON_CLASS}
              onClick={() => setDialog('delete')}
            />
          </>
        }
        rowActions={(video, index) => rowActions(video.id, index)}
      />
      <PromptDialog
        open={dialog === 'rename'}
        onOpenChange={(open) => setDialog(open ? 'rename' : null)}
        title="Rename playlist"
        label="Name"
        initialValue={name}
        confirmLabel="Save"
        onConfirm={(newName) => rename.mutateAsync({ playlistId, name: newName })}
      />
      <ConfirmDialog
        open={dialog === 'delete'}
        onOpenChange={(open) => setDialog(open ? 'delete' : null)}
        title={`Delete ${name}?`}
        description="The playlist is removed from DriveTube. The videos stay in your Google Drive."
        confirmLabel="Delete"
        onConfirm={() =>
          remove.mutate({ playlistId, name }, { onSuccess: () => navigate('/playlists') })
        }
      />
    </>
  );
}
