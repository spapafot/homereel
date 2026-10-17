import { useState } from 'react';

import { Icon } from '@/components/common/Icon';
import { thumbnailUrl, type VideoSummary } from '@/lib/api';
import { cx } from '@/lib/class-names';
import { formatDuration } from '@/lib/format-duration';
import { watchProgress } from '@/lib/video-meta';

/** Videos at least this wide relative to their height fill a 16:9 frame without bars. */
const WIDESCREEN_RATIO = 1.7;

type ThumbnailProps = {
  video: VideoSummary;
  className?: string;
};

/**
 * 16:9 thumbnail with YouTube's duration badge. Portrait and square videos are letterboxed
 * over a blurred copy of the image instead of being cropped.
 *
 * Callers set the width through `className`. A default width here would clash with theirs,
 * because the generated CSS, not the class order, decides which width wins.
 */
export function Thumbnail({ video, className }: ThumbnailProps) {
  const [failed, setFailed] = useState(false);
  const src = thumbnailUrl(video.id);
  const isWidescreen =
    video.width != null && video.height != null && video.width / video.height >= WIDESCREEN_RATIO;
  // Rust extracts a frame for videos Drive has no thumbnail for, so always try the image.
  const showImage = !failed;
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
          onError={() => setFailed(true)}
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
            'absolute right-1 rounded bg-black/60 px-1 py-px text-xs leading-4 font-medium text-white',
            progress != null ? 'bottom-2' : 'bottom-1',
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
