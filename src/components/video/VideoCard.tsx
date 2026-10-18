import { Link } from 'react-router';

import { Avatar } from '@/components/common/Avatar';
import type { VideoSummary } from '@/lib/api';
import { folderPath, watchPath } from '@/lib/paths';
import { relativeDateCreated } from '@/lib/video-meta';

import { MetadataParts } from './MetadataParts';
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
      <div className="relative flex gap-2">
        {folderUrl ? (
          <Link to={folderUrl} tabIndex={-1} aria-hidden="true" className="h-9">
            {avatar}
          </Link>
        ) : (
          avatar
        )}
        <div className="min-w-0 flex-1">
          {/* The padding keeps the title clear of the ⋮ button floating over its corner. */}
          <h3 className="line-clamp-2 pr-6 text-base leading-[22px] font-medium break-words">
            <Link
              to={watchUrl}
              title={video.title}
              className="outline-none focus-visible:underline"
            >
              {video.title}
            </Link>
          </h3>
          <p className="mt-0.5 text-sm leading-5 text-text-secondary">
            <MetadataParts
              parts={[
                folderUrl ? (
                  <Link key="folder" to={folderUrl} className="hover:text-text">
                    {video.folderName}
                  </Link>
                ) : (
                  <span key="folder">{video.folderName}</span>
                ),
                relativeDateCreated(video),
              ]}
            />
          </p>
        </div>
        <VideoMenu video={video} className="absolute -top-1.5 -right-2.5" />
      </div>
    </article>
  );
}

export function VideoCardSkeleton() {
  return (
    <div className="flex flex-col gap-3" aria-hidden="true">
      <div className="aspect-video w-full rounded-xl bg-skeleton" />
      <div className="flex gap-2">
        <div className="size-9 shrink-0 rounded-full bg-skeleton" />
        <div className="flex flex-1 flex-col gap-2 pt-0.5">
          <div className="h-5 w-11/12 rounded bg-skeleton" />
          <div className="h-5 w-3/5 rounded bg-skeleton" />
        </div>
      </div>
    </div>
  );
}
