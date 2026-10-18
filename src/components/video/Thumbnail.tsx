import { useState } from 'react';

import { Icon } from '@/components/common/Icon';
import { useThumbnailUrl } from '@/hooks/use-thumbnail-url';
import { type VideoSummary } from '@/lib/api';
import { cx } from '@/lib/class-names';
import { formatDuration } from '@/lib/format-duration';
import { watchProgress } from '@/lib/video-meta';

/** Videos at least this wide relative to their height fill a 16:9 frame without bars. */
const WIDESCREEN_RATIO = 1.7;

type ThumbnailProps = {
  video: VideoSummary;
  /** A playlist-row thumbnail, where YouTube tucks the duration badge closer to the corner. */
  small?: boolean;
  className?: string;
};

/**
 * 16:9 thumbnail with YouTube's duration badge. Portrait and square videos are letterboxed
 * over a blurred copy of the image instead of being cropped.
 *
 * Callers set the width through `className`. A default width here would clash with theirs,
 * because the generated CSS, not the class order, decides which width wins.
 */
export function Thumbnail({ video, small = false, className }: ThumbnailProps) {
  const src = useThumbnailUrl(video.id);
  // Keyed by URL, so a regenerated thumbnail is tried even after the old one failed.
  const [failedSrc, setFailedSrc] = useState<string>();
  const isWidescreen =
    video.width != null && video.height != null && video.width / video.height >= WIDESCREEN_RATIO;
  // Rust extracts a frame for videos Drive has no thumbnail for, so always try the image.
  const showImage = failedSrc !== src;
  const progress = watchProgress(video);

  return (
    <div className={cx('relative aspect-video overflow-hidden bg-skeleton', className)}>
      {showImage && !isWidescreen && (
        <img
          src={src}
          alt=""
          aria-hidden="true"
          loading="lazy"
          className="absolute inset-0 size-full scale-125 object-cover opacity-70 blur-2xl"
        />
      )}
      {showImage && (
        <img
          src={src}
          alt=""
          loading="lazy"
          onError={() => setFailedSrc(src)}
          className={cx(
            'absolute inset-0 size-full',
            isWidescreen ? 'object-cover' : 'object-contain',
          )}
        />
      )}
      {!showImage && (
        <div className="flex size-full items-center justify-center text-text-secondary">
          <Icon name="movie" size={40} />
        </div>
      )}
      {video.durationMs != null && (
        <span
          className={cx(
            'absolute rounded bg-black/60 px-1 text-xs leading-[18px] font-medium text-white',
            small ? 'right-1' : 'right-2',
            // Clears the 4px watched bar.
            small && progress == null ? 'bottom-1' : 'bottom-2',
          )}
        >
          {formatDuration(video.durationMs)}
        </span>
      )}
      {progress != null && (
        <div
          className="absolute inset-x-0 bottom-0 h-1 bg-white/40"
          role="progressbar"
          aria-label="Watched"
          aria-valuenow={Math.round(progress * 100)}
        >
          <div className="h-full bg-progress" style={{ width: `${progress * 100}%` }} />
        </div>
      )}
    </div>
  );
}
