import type { ReactNode } from 'react';
import { Link } from 'react-router';

import { EmptyState } from '@/components/common/EmptyState';
import { ErrorState } from '@/components/common/ErrorState';
import { Icon } from '@/components/common/Icon';
import { Thumbnail } from '@/components/video/Thumbnail';
import { VideoMenu, type VideoMenuAction } from '@/components/video/VideoMenu';
import { thumbnailUrl, type VideoSummary } from '@/lib/api';
import { watchPath } from '@/lib/paths';
import { videoMetaLine } from '@/lib/video-meta';

type CollectionPageProps = {
  title: string;
  /** List ID used for `?list=` so Next follows this list. */
  listId: string;
  videos: VideoSummary[] | undefined;
  isPending: boolean;
  error: unknown;
  onRetry: () => void;
  empty: { icon: string; title: string; text: string };
  headerActions?: ReactNode;
  rowActions?: (video: VideoSummary, index: number) => VideoMenuAction[];
};

const SKELETON_ROWS = 6;

/** YouTube's playlist page: a hero panel on the left and the numbered videos on the right. */
export function CollectionPage({
  title,
  listId,
  videos,
  isPending,
  error,
  onRetry,
  empty,
  headerActions,
  rowActions,
}: CollectionPageProps) {
  if (error) return <ErrorState error={error} onRetry={onRetry} />;
  const first = videos?.[0];

  return (
    <div className="flex flex-col gap-6 px-6 py-6 lg:flex-row lg:items-start">
      <aside className="relative shrink-0 overflow-hidden rounded-2xl p-6 text-white lg:sticky lg:top-20 lg:w-[360px]">
        <div className="absolute inset-0 bg-neutral-700" aria-hidden="true">
          {first?.hasThumbnail && (
            <img
              src={thumbnailUrl(first.id)}
              alt=""
              className="size-full scale-150 object-cover opacity-80 blur-3xl"
            />
          )}
          <div className="absolute inset-0 bg-gradient-to-b from-black/10 via-black/40 to-black/80" />
        </div>
        <div className="relative">
          {first ? (
            <Link to={watchPath(first.id, { list: listId })} aria-label={`Play ${title}`}>
              <Thumbnail video={first} className="w-full rounded-xl" />
            </Link>
          ) : (
            <div className="flex aspect-video items-center justify-center rounded-xl bg-white/10">
              <Icon name={empty.icon} size={48} />
            </div>
          )}
          <h1 className="mt-6 text-[28px] leading-9 font-bold break-words">{title}</h1>
          <p className="mt-3 text-sm text-white/70">
            {videos ? `${videos.length} ${videos.length === 1 ? 'video' : 'videos'}` : '…'} • Saved
            on this computer
          </p>
          <div className="mt-5 flex items-center gap-2">
            {first && (
              <Link
                to={watchPath(first.id, { list: listId })}
                className="flex h-9 flex-1 items-center justify-center gap-1.5 rounded-full bg-white px-4 text-sm font-medium text-black hover:bg-white/85"
              >
                <Icon name="play_arrow" filled />
                Play all
              </Link>
            )}
            {headerActions}
          </div>
        </div>
      </aside>

      <section className="min-w-0 flex-1">
        {isPending && (
          <div className="flex flex-col gap-2" aria-hidden="true">
            {Array.from({ length: SKELETON_ROWS }, (_, index) => (
              <div key={index} className="flex gap-2 py-2 pl-8">
                <div className="aspect-video w-40 rounded-lg bg-skeleton" />
                <div className="flex flex-1 flex-col gap-2 pt-1">
                  <div className="h-4 w-3/4 rounded bg-skeleton" />
                  <div className="h-3 w-1/3 rounded bg-skeleton" />
                </div>
              </div>
            ))}
          </div>
        )}
        {videos?.length === 0 && (
          <EmptyState icon={empty.icon} title={empty.title}>
            {empty.text}
          </EmptyState>
        )}
        <ol>
          {videos?.map((video, index) => (
            <li
              key={video.id}
              className="group flex items-center gap-2 rounded-xl py-2 pr-1 hover:bg-hover"
            >
              <span className="w-6 shrink-0 text-center text-sm text-text-secondary">
                {index + 1}
              </span>
              <Link
                to={watchPath(video.id, { list: listId })}
                className="flex min-w-0 flex-1 gap-2 outline-none focus-visible:underline"
              >
                <Thumbnail video={video} className="w-40 shrink-0 rounded-lg" />
                <div className="min-w-0 py-0.5">
                  <h3
                    className="line-clamp-2 text-base leading-[22px] font-medium break-words"
                    title={video.title}
                  >
                    {video.title}
                  </h3>
                  <p className="mt-1 text-xs leading-[18px] text-text-secondary">
                    {video.folderName} • {videoMetaLine(video)}
                  </p>
                </div>
              </Link>
              <VideoMenu
                video={video}
                extraActions={rowActions?.(video, index)}
                className="opacity-0 group-hover:opacity-100 focus-visible:opacity-100"
              />
            </li>
          ))}
        </ol>
      </section>
    </div>
  );
}
