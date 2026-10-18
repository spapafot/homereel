import { useState } from 'react';
import { Link } from 'react-router';

import { Icon } from '@/components/common/Icon';
import { useThumbnailUrl } from '@/hooks/use-thumbnail-url';

type PlaylistCardProps = {
  title: string;
  videoCount: number;
  href: string;
  /** First video, whose thumbnail stands for the playlist. */
  coverVideoId: string | null;
};

/** YouTube's playlist tile: a stacked thumbnail with the video count. */
export function PlaylistCard({ title, videoCount, href, coverVideoId }: PlaylistCardProps) {
  const coverSrc = useThumbnailUrl(coverVideoId);
  const [failedCoverSrc, setFailedCoverSrc] = useState<string>();

  return (
    <Link to={href} className="group flex flex-col gap-3 outline-none focus-visible:underline">
      <div className="relative pt-2">
        <div className="absolute inset-x-4 top-0 h-2 rounded-t-lg bg-skeleton" aria-hidden="true" />
        <div
          className="absolute inset-x-2 top-1 h-2 rounded-t-lg bg-chip-hover"
          aria-hidden="true"
        />
        <div className="relative aspect-video overflow-hidden rounded-xl bg-skeleton">
          {coverSrc && coverSrc !== failedCoverSrc ? (
            <img
              src={coverSrc}
              alt=""
              loading="lazy"
              onError={() => setFailedCoverSrc(coverSrc)}
              className="size-full object-cover"
            />
          ) : (
            <div className="flex size-full items-center justify-center text-text-secondary">
              <Icon name="playlist_play" size={48} />
            </div>
          )}
          <span className="absolute right-1 bottom-1 flex items-center gap-1 rounded bg-black/70 px-1.5 py-0.5 text-xs font-medium text-white">
            <Icon name="playlist_play" size={16} />
            {videoCount} {videoCount === 1 ? 'video' : 'videos'}
          </span>
          <div className="absolute inset-0 flex items-center justify-center gap-1 bg-black/60 text-sm font-medium text-white opacity-0 transition-opacity group-hover:opacity-100">
            <Icon name="play_arrow" filled />
            Play all
          </div>
        </div>
      </div>
      <div>
        <h3 className="line-clamp-2 text-sm font-medium break-words">{title}</h3>
        <p className="mt-1 text-xs text-text-secondary">View full playlist</p>
      </div>
    </Link>
  );
}
