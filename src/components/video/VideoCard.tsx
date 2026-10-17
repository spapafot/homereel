import { Link } from 'react-router';

import { Avatar } from '@/components/common/Avatar';
import type { VideoSummary } from '@/lib/api';
import { folderPath, watchPath } from '@/lib/paths';
import { videoMetaLine } from '@/lib/video-meta';

import { Thumbnail } from './Thumbnail';
import { VideoMenu } from './VideoMenu';

/** Grid card laid out like YouTube's home feed: the folder takes the channel's place. */
export function VideoCard({ video }: { video: VideoSummary }) {
  const watchUrl = watchPath(video.id, { folderId: video.folderId });
  const folderUrl = video.folderId ? folderPath(video.folderId) : null;
  const avatar = (
    <Avatar name={video.folderName} colorKey={video.folderId ?? video.folderName} size={36} />
  );

  return (
    <article className="group flex min-w-0 flex-col gap-3">
      <Link to={watchUrl} tabIndex={-1} aria-hidden="true">
        <Thumbnail video={video} className="w-full rounded-xl" />
      </Link>
      <div className="flex gap-3">
        {folderUrl ? (
          <Link to={folderUrl} tabIndex={-1} aria-hidden="true" className="h-9">
            {avatar}
          </Link>
        ) : (
          avatar
        )}
        <div className="min-w-0 flex-1">
          <h3 className="line-clamp-2 text-base leading-[22px] font-medium break-words">
            <Link
              to={watchUrl}
              title={video.title}
              className="outline-none focus-visible:underline"
            >
              {video.title}
            </Link>
          </h3>
          <div className="mt-1 text-sm leading-5 text-text-secondary">
            {folderUrl ? (
              <Link to={folderUrl} className="hover:text-text">
                {video.folderName}
              </Link>
            ) : (
              <span>{video.folderName}</span>
            )}
            <p>{videoMetaLine(video)}</p>
          </div>
        </div>
        <VideoMenu
          video={video}
          className="-mt-2 -mr-2 opacity-0 group-hover:opacity-100 focus-visible:opacity-100"
        />
      </div>
    </article>
  );
}

export function VideoCardSkeleton() {
  return (
    <div className="flex flex-col gap-3" aria-hidden="true">
      <div className="aspect-video w-full rounded-xl bg-skeleton" />
      <div className="flex gap-3">
        <div className="size-9 shrink-0 rounded-full bg-skeleton" />
        <div className="flex flex-1 flex-col gap-2 pt-0.5">
          <div className="h-5 w-11/12 rounded bg-skeleton" />
          <div className="h-5 w-3/5 rounded bg-skeleton" />
        </div>
      </div>
    </div>
  );
}
