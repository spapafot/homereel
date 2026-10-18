import { useState } from 'react';

import { Icon } from '@/components/common/Icon';
import { SaveToPlaylistDialog } from '@/components/video/SaveToPlaylistDialog';
import { VideoMenu } from '@/components/video/VideoMenu';
import { useToggleFavorite } from '@/hooks/use-video-actions';
import type { VideoSummary } from '@/lib/api';
import { revealVideo } from '@/lib/video-source';

type ShortActionProps = {
  icon: string;
  label: string;
  filled?: boolean;
  pressed?: boolean;
  onClick: () => void;
};

function ShortAction({ icon, label, filled, pressed, onClick }: ShortActionProps) {
  return (
    <button
      type="button"
      aria-pressed={pressed}
      onClick={onClick}
      className="group/action flex flex-col items-center gap-1.5 outline-none"
    >
      <span className="flex size-12 items-center justify-center rounded-full bg-chip group-hover/action:bg-chip-hover group-focus-visible/action:ring-2 group-focus-visible/action:ring-focus">
        <Icon name={icon} filled={filled} />
      </span>
      <span className="text-sm">{label}</span>
    </button>
  );
}

/** The round buttons beside a Short, where YouTube puts Like, Comments and Share. */
export function ShortActions({ video }: { video: VideoSummary }) {
  const toggleFavorite = useToggleFavorite();
  const [saveOpen, setSaveOpen] = useState(false);

  return (
    <div className="flex shrink-0 flex-col items-center gap-4">
      <ShortAction
        icon="favorite"
        label="Favorite"
        filled={video.isFavorite}
        pressed={video.isFavorite}
        onClick={() => toggleFavorite.mutate({ video, value: !video.isFavorite })}
      />
      <ShortAction icon="playlist_add" label="Save" onClick={() => setSaveOpen(true)} />
      {video.source === 'local' ? (
        <ShortAction icon="folder_open" label="Folder" onClick={() => revealVideo(video)} />
      ) : (
        <ShortAction icon="open_in_new" label="Drive" onClick={() => revealVideo(video)} />
      )}
      <div className="flex size-12 items-center justify-center rounded-full bg-chip">
        <VideoMenu video={video} />
      </div>
      <SaveToPlaylistDialog videoId={video.id} open={saveOpen} onOpenChange={setSaveOpen} />
    </div>
  );
}
