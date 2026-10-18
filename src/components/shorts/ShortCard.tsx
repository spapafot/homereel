import { useState } from 'react';
import { Link } from 'react-router';

import { Icon } from '@/components/common/Icon';
import { VideoMenu } from '@/components/video/VideoMenu';
import { useThumbnailUrl } from '@/hooks/use-thumbnail-url';
import { type VideoSummary } from '@/lib/api';
import { cx } from '@/lib/class-names';
import { shortsPath } from '@/lib/paths';
import { relativeDateCreated } from '@/lib/video-meta';

type ShortCardProps = {
  video: VideoSummary;
  /** The folder the cards come from, so the Shorts player keeps to it. */
  folderId?: string;
  /** The watch page's row, where YouTube shows a third of the column as a 2:3 card. */
  compact?: boolean;
};

/** A Shorts card: a tall thumbnail with the title under it, as on YouTube. */
export function ShortCard({ video, folderId, compact = false }: ShortCardProps) {
  const thumbnailSrc = useThumbnailUrl(video.id);
  const [failedSrc, setFailedSrc] = useState<string>();
  const url = shortsPath(video.id, folderId);

  return (
    <article className="group flex min-w-0 flex-col gap-2">
      <Link
        to={url}
        tabIndex={-1}
        aria-hidden="true"
        className={cx(
          'relative block overflow-hidden bg-skeleton',
          compact ? 'aspect-[2/3] rounded-lg' : 'aspect-[9/16] rounded-xl',
        )}
      >
        {failedSrc === thumbnailSrc ? (
          <div className="flex size-full items-center justify-center text-text-secondary">
            <Icon name="movie" size={40} />
          </div>
        ) : (
          <img
            src={thumbnailSrc}
            alt=""
            loading="lazy"
            onError={() => setFailedSrc(thumbnailSrc)}
            className="size-full object-cover"
          />
        )}
      </Link>
      <div className="flex items-start gap-1">
        <div className="min-w-0 flex-1">
          <h3 className="line-clamp-2 text-base leading-[22px] font-medium break-words">
            <Link to={url} title={video.title} className="outline-none focus-visible:underline">
              {video.title}
            </Link>
          </h3>
          <p className="mt-1 truncate text-sm leading-5 text-text-secondary">
            {relativeDateCreated(video)}
          </p>
        </div>
        {/* YouTube keeps the watch page row's ⋮ in view; the shelf's shows on hover. */}
        <VideoMenu
          video={video}
          className={cx(
            '-mt-2',
            !compact && '-mr-3 opacity-0 group-hover:opacity-100 focus-visible:opacity-100',
          )}
        />
      </div>
    </article>
  );
}
