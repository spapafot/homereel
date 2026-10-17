import { useState, type FormEvent } from 'react';

import { Button } from '@/components/common/Button';
import { Icon } from '@/components/common/Icon';
import { Modal } from '@/components/common/Modal';
import {
  useCreatePlaylist,
  usePlaylistMemberships,
  useTogglePlaylistVideo,
} from '@/hooks/use-library';
import { toAppError } from '@/lib/app-error';

type SaveToPlaylistDialogProps = {
  videoId: string;
  open: boolean;
  onOpenChange: (open: boolean) => void;
};

/** YouTube's "Save video to…" dialog: tick playlists, or create a new one. */
export function SaveToPlaylistDialog({ videoId, open, onOpenChange }: SaveToPlaylistDialogProps) {
  const memberships = usePlaylistMemberships(videoId, open);
  const toggle = useTogglePlaylistVideo();

  return (
    <Modal
      open={open}
      onOpenChange={onOpenChange}
      title="Save video to…"
      className="w-[min(92vw,360px)]"
    >
      <div className="mt-4 flex max-h-72 flex-col overflow-y-auto">
        {memberships.isPending && <div className="h-10 animate-pulse rounded bg-skeleton" />}
        {memberships.data?.length === 0 && (
          <p className="py-2 text-sm text-text-secondary">You don’t have any playlists yet.</p>
        )}
        {memberships.data?.map((playlist) => (
          <label key={playlist.id} className="flex h-10 cursor-pointer items-center gap-4 text-sm">
            <input
              type="checkbox"
              checked={playlist.contains}
              disabled={toggle.isPending}
              onChange={(event) =>
                toggle.mutate({
                  playlistId: playlist.id,
                  videoId,
                  add: event.target.checked,
                  name: playlist.name,
                })
              }
              className="size-5 accent-text"
            />
            <span className="truncate">{playlist.name}</span>
          </label>
        ))}
      </div>
      {open && <NewPlaylistForm videoId={videoId} onCreated={() => onOpenChange(false)} />}
    </Modal>
  );
}

function NewPlaylistForm({ videoId, onCreated }: { videoId: string; onCreated: () => void }) {
  const create = useCreatePlaylist();
  const [isOpen, setIsOpen] = useState(false);
  const [name, setName] = useState('');

  if (!isOpen) {
    return (
      <Button icon="add" className="mt-4 w-full justify-center" onClick={() => setIsOpen(true)}>
        New playlist
      </Button>
    );
  }

  const submit = (event: FormEvent) => {
    event.preventDefault();
    if (!name.trim()) return;
    create.mutate({ name: name.trim(), videoId }, { onSuccess: onCreated });
  };

  return (
    <form onSubmit={submit} className="mt-4 border-t border-divider pt-4">
      <label className="block">
        <span className="text-xs text-text-secondary">Name</span>
        <input
          autoFocus
          value={name}
          maxLength={150}
          onChange={(event) => setName(event.target.value)}
          placeholder="Choose a name"
          className="mt-1 w-full border-b border-divider bg-transparent pb-1 text-base outline-none placeholder:text-text-secondary focus:border-b-2 focus:border-text"
        />
      </label>
      {create.error && (
        <p className="mt-2 text-sm text-red-600 dark:text-red-400">
          {toAppError(create.error).message}
        </p>
      )}
      <div className="mt-4 flex justify-end gap-2">
        <Button onClick={() => setIsOpen(false)}>Cancel</Button>
        <Button type="submit" variant="primary" disabled={!name.trim() || create.isPending}>
          <Icon name="playlist_add" />
          Create
        </Button>
      </div>
    </form>
  );
}
