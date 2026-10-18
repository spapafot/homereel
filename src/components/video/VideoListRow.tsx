import { Link } from 'react-router';

import { Avatar } from '@/components/common/Avatar';
import { IconButton } from '@/components/common/IconButton';
import type { VideoSummary } from '@/lib/api';
import { folderPath, watchPath } from '@/lib/paths';
import { relativeDateCreated, videoDetailsParts } from '@/lib/video-meta';

import { MetadataParts } from './MetadataParts';
import { Thumbnail } from './Thumbnail';
import { VideoMenu } from './VideoMenu';

type VideoListRowProps = {
  video: VideoSummary;
  /** Shows YouTube's ✕ button, e.g. "Remove from watch history". */
  onRemove?: { label: string; action: () => void };
};

/** Search-results and history row: large thumbnail on the left, details on the right. */
export function VideoListRow({ video, onRemove }: VideoListRowProps) {
  const watchUrl = watchPath(video.id, { folderId: video.folderId });

  return (
    <article className="group flex gap-4">
      <Link
        to={watchUrl}
        tabIndex={-1}
        aria-hidden="true"
        className="w-[360px] max-w-[45%] shrink-0"
      >
        <Thumbnail video={video} className="w-full rounded-xl" />
      </Link>
      <div className="min-w-0 flex-1 py-0.5">
        <h3 className="line-clamp-2 text-lg leading-[26px] break-words">
          <Link to={watchUrl} title={video.title} className="outline-none focus-visible:underline">
            {video.title}
          </Link>
        </h3>
        <p className="text-xs leading-[18px] text-text-secondary">{relativeDateCreated(video)}</p>
        <Link
          to={video.folderId ? folderPath(video.folderId) : watchUrl}
          className="my-3 flex w-fit items-center gap-2 text-xs text-text-secondary hover:text-text"
        >
          <Avatar name={video.folderName} colorKey={video.folderId ?? video.folderName} size={24} />
          {video.folderName}
        </Link>
        <p className="line-clamp-1 text-xs text-text-secondary">
          <MetadataParts parts={[video.fileName, ...videoDetailsParts(video)]} />
        </p>
      </div>
      <div className="flex flex-col items-center">
        {onRemove && (
          <IconButton
            icon="close"
            label={onRemove.label}
            onClick={onRemove.action}
            className="opacity-0 group-hover:opacity-100 focus-visible:opacity-100"
          />
        )}
        <VideoMenu
          video={video}
          className="opacity-0 group-hover:opacity-100 focus-visible:opacity-100"
        />
      </div>
    </article>
  );
}

export function VideoListRowSkeleton() {
  return (
    <div className="flex gap-4" aria-hidden="true">
      <div className="aspect-video w-[360px] max-w-[45%] shrink-0 rounded-xl bg-skeleton" />
      <div className="flex flex-1 flex-col gap-3 pt-1">
        <div className="h-6 w-4/5 rounded bg-skeleton" />
        <div className="h-4 w-2/5 rounded bg-skeleton" />
        <div className="h-4 w-1/3 rounded bg-skeleton" />
      </div>
    </div>
  );
}
