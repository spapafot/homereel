import { Link } from 'react-router';

import { MetadataParts } from '@/components/video/MetadataParts';
import { Thumbnail } from '@/components/video/Thumbnail';
import { VideoMenu } from '@/components/video/VideoMenu';
import type { VideoSummary } from '@/lib/api';
import { cx } from '@/lib/class-names';
import { watchPath, type WatchContext } from '@/lib/paths';
import { relativeDateCreated } from '@/lib/video-meta';

type RelatedVideoItemProps = {
  video: VideoSummary;
  context?: WatchContext;
  isNext?: boolean;
  isCurrent?: boolean;
  position?: number;
};

/**
 * A suggestion row in the watch page's right-hand column, or, with `position`, a compact
 * numbered row in the playlist panel.
 */
export function RelatedVideoItem({
  video,
  context = { folderId: video.folderId },
  isNext = false,
  isCurrent = false,
  position,
}: RelatedVideoItemProps) {
  const to = watchPath(video.id, context);

  if (position == null) {
    // YouTube's compact row: the thumbnail takes most of the column, the text the rest.
    return (
      <div className="group relative">
        <Link
          to={to}
          className="flex rounded-lg outline-none focus-visible:ring-2 focus-visible:ring-focus"
        >
          <Thumbnail video={video} className="w-[62.5%] shrink-0 rounded-lg" />
          <div className="min-w-0 flex-1 pr-1 pl-3">
            <h3
              className="line-clamp-3 text-sm leading-5 font-medium break-words"
              title={video.title}
            >
              {video.title}
            </h3>
            <p className="mt-0.5 truncate text-xs leading-[18px] text-text-secondary">
              {video.folderName}
            </p>
            <p className="mt-0.5 text-xs leading-[18px] text-text-secondary">
              <MetadataParts parts={[isNext ? 'Up next' : null, relativeDateCreated(video)]} />
            </p>
          </div>
        </Link>
        <VideoMenu
          video={video}
          className="absolute right-0 bottom-0 opacity-0 group-hover:opacity-100 focus-visible:opacity-100"
        />
      </div>
    );
  }

  return (
    <Link
      to={to}
      aria-current={isCurrent ? 'true' : undefined}
      className={cx(
        'flex items-center gap-2 rounded-lg outline-none focus-visible:ring-2 focus-visible:ring-focus',
        isCurrent && 'bg-chip',
      )}
    >
      <span className="w-6 shrink-0 text-center text-xs text-text-secondary">
        {isCurrent ? '▶' : position}
      </span>
      <Thumbnail video={video} small className="w-[100px] shrink-0 rounded-lg" />
      <div className="min-w-0 flex-1 self-start py-0.5 pr-1">
        <h3 className="line-clamp-2 text-sm leading-5 font-medium break-words" title={video.title}>
          {video.title}
        </h3>
        <p className="mt-1 truncate text-xs leading-[18px] text-text-secondary">
          {video.folderName}
        </p>
      </div>
    </Link>
  );
}

export function RelatedVideoSkeleton() {
  return (
    <div className="flex" aria-hidden="true">
      <div className="aspect-video w-[62.5%] shrink-0 rounded-lg bg-skeleton" />
      <div className="flex flex-1 flex-col gap-2 pl-3">
        <div className="h-4 w-full rounded bg-skeleton" />
        <div className="h-4 w-2/3 rounded bg-skeleton" />
      </div>
    </div>
  );
}
