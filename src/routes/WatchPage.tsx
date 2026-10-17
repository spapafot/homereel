import { useQueryClient } from '@tanstack/react-query';
import { useCallback, useEffect, useRef, useState } from 'react';
import { Link, useNavigate, useSearchParams } from 'react-router';

import { Avatar } from '@/components/common/Avatar';
import { Button } from '@/components/common/Button';
import { DriveGate } from '@/components/common/DriveGate';
import { EmptyState } from '@/components/common/EmptyState';
import { ErrorState } from '@/components/common/ErrorState';
import { Icon } from '@/components/common/Icon';
import { IconButton } from '@/components/common/IconButton';
import { TagFrameDialog } from '@/components/people/TagFrameDialog';
import { ChipBar, type Chip } from '@/components/video/ChipBar';
import { EditTitleDialog } from '@/components/video/EditTitleDialog';
import { DEFAULT_VIDEO_ORDER } from '@/components/video/order-chips';
import { SaveToPlaylistDialog } from '@/components/video/SaveToPlaylistDialog';
import { VideoMenu } from '@/components/video/VideoMenu';
import { RelatedVideoItem, RelatedVideoSkeleton } from '@/components/watch/RelatedVideoItem';
import { VideoPlayer, type PlayerHandle } from '@/components/watch/VideoPlayer';
import { useVideoPeople } from '@/hooks/use-people';
import { useToggleFavorite, useToggleWatchLater } from '@/hooks/use-video-actions';
import { useNextInFeed, usePlaybackSource, useVideo, useVideoFeed } from '@/hooks/use-videos';
import { useWatchList, type WatchList } from '@/hooks/use-watch-list';
import { api, type VideoSummary } from '@/lib/api';
import { toAppError } from '@/lib/app-error';
import { formatRelativeDate } from '@/lib/format-relative-date';
import { folderPath, playlistPath, watchPath, type WatchContext } from '@/lib/paths';
import { showToast } from '@/lib/toast';
import { pickNextVideo } from '@/lib/up-next';
import { patchVideoInCaches } from '@/lib/video-cache';
import { dateCreated, videoDetailsLine } from '@/lib/video-meta';

type RelatedMode = 'folder' | 'recent';

const RELATED_SKELETON_COUNT = 8;
const MIN_RESUME_MS = 5_000;
/** Near the end, start over instead of resuming, as YouTube does. */
const RESTART_FRACTION = 0.95;
const WATCHED_FRACTION = 0.9;

export function WatchPage() {
  const [params] = useSearchParams();
  const videoId = params.get('v') ?? '';

  return (
    <DriveGate>
      {videoId ? (
        <WatchContent
          key={videoId}
          videoId={videoId}
          contextFolderId={params.get('folder')}
          listId={params.get('list')}
        />
      ) : (
        <EmptyState icon="smart_display" title="No video selected" />
      )}
    </DriveGate>
  );
}

function resumePoint(video: VideoSummary | undefined): number | null {
  if (!video || video.watched || !video.lastPositionMs || !video.durationMs) return null;
  const { lastPositionMs, durationMs } = video;
  return lastPositionMs >= MIN_RESUME_MS && lastPositionMs < durationMs * RESTART_FRACTION
    ? lastPositionMs
    : null;
}

type WatchContentProps = {
  videoId: string;
  contextFolderId: string | null;
  listId: string | null;
};

function WatchContent({ videoId, contextFolderId, listId }: WatchContentProps) {
  const navigate = useNavigate();
  const queryClient = useQueryClient();
  const video = useVideo(videoId);
  const source = usePlaybackSource(videoId, video.isSuccess);
  const { list } = useWatchList(listId);
  const folderId = contextFolderId ?? video.data?.folderId ?? undefined;
  const [relatedMode, setRelatedMode] = useState<RelatedMode>('folder');
  const playerRef = useRef<PlayerHandle>(null);
  const [tagPositionMs, setTagPositionMs] = useState<number | null>(null);
  // The player only applies this when the video first loads, so later progress updates to the
  // cached video do not make it jump.
  const startAtMs = resumePoint(video.data);
  const showFolder = relatedMode === 'folder' && folderId != null;

  // Up next follows the folder page's default order.
  const folderFeed = useVideoFeed(
    { folderId, order: DEFAULT_VIDEO_ORDER },
    { enabled: folderId != null },
  );
  const recentFeed = useVideoFeed({ order: 'uploadedNewest' }, { enabled: !showFolder });
  const nextInFolder = useNextInFeed(folderFeed, videoId, { enabled: !list });
  const nextVideo = list ? pickNextVideo(list.videos, videoId) : nextInFolder;
  const listContextId = list?.id ?? null;
  const relatedFeed = showFolder ? folderFeed : recentFeed;
  const related = (relatedFeed.data?.pages.flatMap((page) => page.videos) ?? []).filter(
    (candidate) => candidate.id !== videoId,
  );

  // History and Continue watching change while watching; refresh them when leaving.
  useEffect(
    () => () => {
      void queryClient.invalidateQueries({ queryKey: ['history'] });
      void queryClient.invalidateQueries({ queryKey: ['library', 'continueWatching'] });
    },
    [queryClient],
  );

  const goToNext = useCallback(() => {
    if (!nextVideo) return;
    const context: WatchContext = listContextId ? { list: listContextId } : { folderId };
    navigate(watchPath(nextVideo.id, context));
  }, [navigate, nextVideo, listContextId, folderId]);
  const openInDrive = useCallback(() => {
    api.playbackOpenInBrowser(videoId).catch(() => showToast('Could not open Google Drive'));
  }, [videoId]);
  const saveProgress = useCallback(
    (positionMs: number, durationMs: number) => {
      api
        .recordProgress(videoId, positionMs, durationMs)
        .then(() =>
          patchVideoInCaches(queryClient, videoId, (current) => ({
            ...current,
            lastPositionMs: positionMs,
            watched: current.watched || positionMs >= durationMs * WATCHED_FRACTION,
          })),
        )
        .catch((error: unknown) => console.error('Could not save the resume point', error));
    },
    [queryClient, videoId],
  );

  if (video.isError) {
    return toAppError(video.error).code === 'not_found' ? (
      <EmptyState icon="videocam_off" title="This video isn’t available anymore">
        It may have been moved to the trash, deleted, or unshared in Google Drive.
      </EmptyState>
    ) : (
      <ErrorState error={video.error} onRetry={() => video.refetch()} />
    );
  }

  const relatedChips: Chip[] = [
    ...(folderId && video.data ? [{ id: 'folder', label: `From ${video.data.folderName}` }] : []),
    { id: 'recent', label: 'Recently added' },
  ];

  return (
    <div className="mx-auto flex max-w-[1754px] flex-col gap-6 px-6 pt-6 pb-12 lg:flex-row">
      <div className="min-w-0 flex-1">
        {source.isError ? (
          <div className="aspect-video w-full bg-skeleton">
            <ErrorState error={source.error} onRetry={() => source.refetch()} />
          </div>
        ) : (
          <VideoPlayer
            ref={playerRef}
            src={source.data?.url}
            title={video.data?.title ?? ''}
            hasNext={nextVideo != null}
            onNext={goToNext}
            onOpenInDrive={openInDrive}
            startAtMs={startAtMs}
            onProgress={saveProgress}
          />
        )}
        {video.data ? (
          <VideoDetails
            video={video.data}
            onOpenInDrive={openInDrive}
            onTagFrame={() => setTagPositionMs(playerRef.current?.pauseAt() ?? 0)}
          />
        ) : (
          <div className="mt-3 flex flex-col gap-3" aria-hidden="true">
            <div className="h-7 w-3/4 rounded bg-skeleton" />
            <div className="h-10 w-1/3 rounded bg-skeleton" />
          </div>
        )}
      </div>

      <aside className="flex w-full shrink-0 flex-col gap-3 lg:w-[344px]">
        {list && <PlaylistPanel list={list} currentId={videoId} />}
        <div>
          <ChipBar
            chips={relatedChips}
            selectedId={showFolder ? 'folder' : 'recent'}
            onSelect={(id) => setRelatedMode(id as RelatedMode)}
          />
          <div className="mt-3 flex flex-col gap-4">
            {relatedFeed.isError && (
              <ErrorState error={relatedFeed.error} onRetry={() => relatedFeed.refetch()} />
            )}
            {related.map((candidate) => (
              <RelatedVideoItem
                key={candidate.id}
                video={candidate}
                isNext={!list && showFolder && candidate.id === nextVideo?.id}
              />
            ))}
            {relatedFeed.isPending &&
              Array.from({ length: RELATED_SKELETON_COUNT }, (_, index) => (
                <RelatedVideoSkeleton key={index} />
              ))}
            {relatedFeed.hasNextPage && (
              <Button
                variant="outline"
                className="mt-2 justify-center"
                disabled={relatedFeed.isFetchingNextPage}
                onClick={() => relatedFeed.fetchNextPage()}
              >
                Show more
              </Button>
            )}
          </div>
        </div>
      </aside>
      <TagFrameDialog
        videoId={videoId}
        positionMs={tagPositionMs}
        onClose={() => setTagPositionMs(null)}
      />
    </div>
  );
}

/** The bordered box YouTube shows above related videos when playing from a playlist. */
function PlaylistPanel({ list, currentId }: { list: WatchList; currentId: string }) {
  const index = list.videos.findIndex((candidate) => candidate.id === currentId);
  return (
    <section className="overflow-hidden rounded-xl border border-divider">
      <header className="bg-chip px-4 py-3">
        <Link
          to={playlistPath(list.id)}
          className="block truncate text-xl font-bold hover:underline"
        >
          {list.title}
        </Link>
        <p className="mt-1 text-xs text-text-secondary">
          {index >= 0 ? `${index + 1} / ${list.videos.length}` : `${list.videos.length} videos`}
        </p>
      </header>
      <ol className="flex max-h-[400px] flex-col gap-1 overflow-y-auto py-2 pr-2">
        {list.videos.map((candidate, position) => (
          <li key={candidate.id}>
            <RelatedVideoItem
              video={candidate}
              context={{ list: list.id }}
              position={position + 1}
              isCurrent={candidate.id === currentId}
            />
          </li>
        ))}
      </ol>
    </section>
  );
}

const DAY_ONLY = /^\d{4}-\d{2}-\d{2}$/;

function formatDate(date: string | null): string {
  if (!date) return '—';
  // A bare `YYYY-MM-DD` is a calendar day; read as UTC midnight it can show the day before.
  return new Date(DAY_ONLY.test(date) ? `${date}T00:00` : date).toLocaleDateString(undefined, {
    year: 'numeric',
    month: 'short',
    day: 'numeric',
  });
}

function VideoDetails({
  video,
  onOpenInDrive,
  onTagFrame,
}: {
  video: VideoSummary;
  onOpenInDrive: () => void;
  onTagFrame: () => void;
}) {
  const [dialog, setDialog] = useState<'playlist' | 'title' | null>(null);
  const recognizedPeople = useVideoPeople(video.id).data ?? [];
  const toggleFavorite = useToggleFavorite();
  const toggleWatchLater = useToggleWatchLater();
  const folderUrl = video.folderId ? folderPath(video.folderId) : null;
  const created = dateCreated(video);
  const headline = [
    created ? `Created ${formatRelativeDate(created)}` : null,
    videoDetailsLine(video),
  ]
    .filter(Boolean)
    .join(' • ');

  return (
    <>
      <div className="mt-3 flex items-start gap-2">
        <h1 className="line-clamp-2 min-w-0 flex-1 text-xl leading-7 font-bold break-words">
          {video.title}
          {video.titleSource === 'ai' && (
            <span
              title="Title generated by Gemini from the video’s frames"
              className="ml-2 inline-flex translate-y-[-2px] items-center gap-0.5 rounded bg-chip px-1.5 py-0.5 align-middle text-xs font-medium text-text-secondary"
            >
              <Icon name="auto_awesome" size={14} /> AI
            </span>
          )}
        </h1>
        <IconButton icon="edit" label="Edit title" onClick={() => setDialog('title')} />
      </div>
      <div className="mt-3 flex flex-wrap items-center justify-between gap-3">
        <div className="flex min-w-0 items-center gap-3">
          <Avatar name={video.folderName} colorKey={video.folderId ?? video.folderName} size={40} />
          <div className="min-w-0">
            {folderUrl ? (
              <Link to={folderUrl} className="block truncate text-base font-medium hover:underline">
                {video.folderName}
              </Link>
            ) : (
              <p className="truncate text-base font-medium">{video.folderName}</p>
            )}
            <p className="text-xs text-text-secondary">Google Drive folder</p>
          </div>
        </div>
        <div className="flex flex-wrap items-center gap-2">
          <Button
            icon="favorite"
            iconFilled={video.isFavorite}
            aria-pressed={video.isFavorite}
            onClick={() => toggleFavorite.mutate({ video, value: !video.isFavorite })}
          >
            {video.isFavorite ? 'Favorited' : 'Favorite'}
          </Button>
          <Button
            icon={video.isWatchLater ? 'check' : 'schedule'}
            aria-pressed={video.isWatchLater}
            onClick={() => toggleWatchLater.mutate({ video, value: !video.isWatchLater })}
          >
            Watch later
          </Button>
          <Button icon="playlist_add" onClick={() => setDialog('playlist')}>
            Save
          </Button>
          <Button icon="open_in_new" onClick={onOpenInDrive}>
            Open in Drive
          </Button>
          <VideoMenu
            video={video}
            extraActions={[
              { icon: 'person_add', label: 'Tag a person or pet', onSelect: onTagFrame },
            ]}
            className="bg-chip hover:bg-chip-hover"
          />
        </div>
      </div>
      <section className="mt-3 rounded-xl bg-chip p-3 text-sm">
        <p className="font-medium">{headline}</p>
        <dl className="mt-2 grid grid-cols-[auto_1fr] gap-x-6 gap-y-1">
          <dt className="text-text-secondary">File name</dt>
          <dd className="break-all">{video.fileName}</dd>
          {video.aiTitle && video.titleSource !== 'ai' && (
            <>
              <dt className="text-text-secondary">AI title</dt>
              <dd>{video.aiTitle}</dd>
            </>
          )}
          {recognizedPeople.length > 0 && (
            <>
              <dt className="text-text-secondary">People</dt>
              <dd title="Recognized by Gemini from your people and pets">
                {recognizedPeople.join(', ')}
              </dd>
            </>
          )}
          <dt className="text-text-secondary">Type</dt>
          <dd>{video.mimeType}</dd>
          <dt className="text-text-secondary">Date created</dt>
          <dd>
            {formatDate(created)}
            {!video.recordedDate && created && (
              <span
                className="text-text-secondary"
                title="DriveTube reads the recording date from the video file in the background"
              >
                {' '}
                · Drive’s modified date
              </span>
            )}
          </dd>
          <dt className="text-text-secondary">Date uploaded</dt>
          <dd>{formatDate(video.createdTime)}</dd>
        </dl>
      </section>
      <SaveToPlaylistDialog
        videoId={video.id}
        open={dialog === 'playlist'}
        onOpenChange={(open) => setDialog(open ? 'playlist' : null)}
      />
      <EditTitleDialog
        video={video}
        open={dialog === 'title'}
        onOpenChange={(open) => setDialog(open ? 'title' : null)}
      />
    </>
  );
}
