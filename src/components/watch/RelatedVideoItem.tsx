import { Link } from 'react-router';

import { Thumbnail } from '@/components/video/Thumbnail';
import type { VideoSummary } from '@/lib/api';
import { cx } from '@/lib/class-names';
import { watchPath, type WatchContext } from '@/lib/paths';
import { videoMetaLine } from '@/lib/video-meta';

type RelatedVideoItemProps = {
  video: VideoSummary;
  context?: WatchContext;
  isNext?: boolean;
  isCurrent?: boolean;
  position?: number;
};

/**
 * A suggestion card in the watch page's right-hand column, or, with `position`, a compact
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
    return (
      <Link
        to={to}
        className="flex flex-col gap-2 rounded-xl outline-none focus-visible:ring-2 focus-visible:ring-focus"
      >
        <Thumbnail video={video} className="w-full rounded-xl" />
        <div className="min-w-0 pr-1">
          <h3
            className="line-clamp-2 text-sm leading-5 font-medium break-words"
            title={video.title}
          >
            {video.title}
          </h3>
          <p className="mt-1 truncate text-xs leading-[18px] text-text-secondary">
            {video.folderName}
          </p>
          <p className="text-xs leading-[18px] text-text-secondary">
            {isNext ? `Up next • ${videoMetaLine(video)}` : videoMetaLine(video)}
          </p>
        </div>
      </Link>
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
      <Thumbnail video={video} className="w-[100px] shrink-0 rounded-lg" />
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
    <div className="flex flex-col gap-2" aria-hidden="true">
      <div className="aspect-video w-full rounded-xl bg-skeleton" />
      <div className="h-4 w-full rounded bg-skeleton" />
      <div className="h-4 w-2/3 rounded bg-skeleton" />
    </div>
  );
}
