import { useCallback, useEffect, useLayoutEffect, useRef, useState } from 'react';
import {
  Link,
  useLocation,
  useNavigate,
  useNavigationType,
  useParams,
  useSearchParams,
} from 'react-router';

import { Avatar } from '@/components/common/Avatar';
import { LibraryGate } from '@/components/common/LibraryGate';
import { EmptyState } from '@/components/common/EmptyState';
import { ErrorState } from '@/components/common/ErrorState';
import { Icon } from '@/components/common/Icon';
import { ShortActions } from '@/components/shorts/ShortActions';
import { ShortPlayer } from '@/components/shorts/ShortPlayer';
import { useThumbnailUrl } from '@/hooks/use-thumbnail-url';
import { useShortsFeed, useVideo } from '@/hooks/use-videos';
import type { VideoSummary } from '@/lib/api';
import { folderPath, shortsPath } from '@/lib/paths';
import { openVideoElsewhere } from '@/lib/video-source';
import { isTypingTarget } from '@/lib/typing-target';

/** Shorts this close to the current one get their picture, title and buttons; the rest are empty. */
const RENDER_AROUND = 2;
/** The next page loads when the current Short is this close to the last one loaded. */
const PRELOAD_BEFORE_END = 3;

type ShortsLocationState = { fromFeed?: boolean } | null;

/** YouTube's Shorts player: one Short per screen, scrolled with the wheel, arrows or buttons. */
export function ShortsPage() {
  const { videoId } = useParams();
  const [params] = useSearchParams();
  const location = useLocation();
  const navigationType = useNavigationType();
  const folderId = params.get('folder') ?? undefined;
  // Scrolling rewrites the URL to the current Short; any other navigation (the guide, Back)
  // starts the feed afresh at the Short it names.
  const [feedKey, setFeedKey] = useState(location.key);
  const fromFeed =
    navigationType === 'REPLACE' && (location.state as ShortsLocationState)?.fromFeed === true;
  if (!fromFeed && feedKey !== location.key) setFeedKey(location.key);

  return (
    <LibraryGate>
      <ShortsFeed key={feedKey} urlVideoId={videoId ?? null} folderId={folderId} />
    </LibraryGate>
  );
}

type ShortsFeedProps = {
  urlVideoId: string | null;
  folderId: string | undefined;
};

function ShortsFeed({ urlVideoId, folderId }: ShortsFeedProps) {
  const navigate = useNavigate();
  const feed = useShortsFeed(folderId);
  const { fetchNextPage, hasNextPage, isFetchingNextPage } = feed;
  const [startId] = useState(urlVideoId);
  const [activeId, setActiveId] = useState(urlVideoId);
  const [isMuted, setIsMuted] = useState(false);
  const scrollerRef = useRef<HTMLDivElement>(null);
  const positionedRef = useRef(false);

  const feedVideos = feed.data?.pages.flatMap((page) => page.videos) ?? [];
  const startInFeed = startId == null || feedVideos.some((video) => video.id === startId);
  // A link can name a Short past the loaded pages, or a video that isn't a Short at all; that
  // one plays first, ahead of the feed.
  const searchingStart = !startInFeed && hasNextPage;
  const startOutsideFeed = !startInFeed && feed.isSuccess && !hasNextPage;
  const startVideo = useVideo(startOutsideFeed && startId ? startId : '');
  const items = startOutsideFeed && startVideo.data ? [startVideo.data, ...feedVideos] : feedVideos;
  const ready = feed.isSuccess && !searchingStart && (!startOutsideFeed || !startVideo.isPending);
  const activeIndex = Math.max(
    0,
    items.findIndex((video) => video.id === activeId),
  );
  const currentId = ready ? items[activeIndex]?.id : undefined;
  const pageCount = feed.data?.pages.length ?? 0;

  useEffect(() => {
    if (searchingStart && !isFetchingNextPage) void fetchNextPage();
  }, [searchingStart, isFetchingNextPage, pageCount, fetchNextPage]);

  useEffect(() => {
    const nearEnd = activeIndex >= items.length - PRELOAD_BEFORE_END;
    if (ready && nearEnd && hasNextPage && !isFetchingNextPage) void fetchNextPage();
  }, [ready, activeIndex, items.length, hasNextPage, isFetchingNextPage, fetchNextPage]);

  // Opens at the linked Short; after that the scroll position decides which Short is current.
  useLayoutEffect(() => {
    const scroller = scrollerRef.current;
    if (!ready || positionedRef.current || !scroller) return;
    positionedRef.current = true;
    scroller.scrollTop = activeIndex * scroller.clientHeight;
  }, [ready, activeIndex]);

  useEffect(() => {
    if (!currentId || currentId === urlVideoId) return;
    const state: ShortsLocationState = { fromFeed: true };
    navigate(shortsPath(currentId, folderId), { replace: true, state });
  }, [currentId, urlVideoId, folderId, navigate]);

  const onScroll = () => {
    const scroller = scrollerRef.current;
    if (!scroller || !positionedRef.current || scroller.clientHeight === 0) return;
    const index = Math.round(scroller.scrollTop / scroller.clientHeight);
    const video = items[Math.min(index, items.length - 1)];
    if (video) setActiveId(video.id);
  };

  const goTo = useCallback((index: number) => {
    const scroller = scrollerRef.current;
    scroller?.scrollTo({ top: index * scroller.clientHeight, behavior: 'smooth' });
  }, []);

  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.defaultPrevented || event.ctrlKey || event.metaKey || event.altKey) return;
      if (isTypingTarget(event.target)) return;
      // Menus and dialogs use the arrow keys themselves.
      if (
        event.target instanceof Element &&
        event.target.closest('[role="menu"],[role="dialog"]')
      ) {
        return;
      }
      if (event.key === 'ArrowDown') goTo(activeIndex + 1);
      else if (event.key === 'ArrowUp') goTo(activeIndex - 1);
      else return;
      event.preventDefault();
    };
    window.addEventListener('keydown', onKeyDown);
    return () => window.removeEventListener('keydown', onKeyDown);
  }, [goTo, activeIndex]);

  const toggleMute = useCallback(() => setIsMuted((muted) => !muted), []);

  if (feed.isError) return <ErrorState error={feed.error} onRetry={() => feed.refetch()} />;
  if (ready && items.length === 0) {
    return (
      <EmptyState icon="smart_display" title="No Shorts here yet">
        Shorts are vertical or square videos up to 3 minutes long.
      </EmptyState>
    );
  }

  return (
    <div className="relative h-[calc(100dvh-3.5rem)]">
      {ready ? (
        <div
          ref={scrollerRef}
          onScroll={onScroll}
          className="scrollbar-none size-full snap-y snap-mandatory overflow-y-auto"
        >
          {items.map((video, index) => (
            <section
              key={video.id}
              aria-label={video.title}
              className="flex h-full snap-start snap-always items-center justify-center py-4"
            >
              {Math.abs(index - activeIndex) <= RENDER_AROUND && (
                <ShortItem
                  video={video}
                  isActive={index === activeIndex}
                  isMuted={isMuted}
                  onToggleMute={toggleMute}
                />
              )}
            </section>
          ))}
        </div>
      ) : (
        <div className="flex h-full items-center justify-center py-4" aria-hidden="true">
          <div className="aspect-[9/16] h-full rounded-xl bg-skeleton" />
        </div>
      )}
      {ready && (
        <nav
          aria-label="Shorts"
          className="absolute top-1/2 right-6 hidden -translate-y-1/2 flex-col gap-4 sm:flex"
        >
          <NavButton
            icon="arrow_upward"
            label="Previous Short"
            disabled={activeIndex === 0}
            onClick={() => goTo(activeIndex - 1)}
          />
          <NavButton
            icon="arrow_downward"
            label="Next Short"
            disabled={activeIndex >= items.length - 1}
            onClick={() => goTo(activeIndex + 1)}
          />
        </nav>
      )}
    </div>
  );
}

type ShortItemProps = {
  video: VideoSummary;
  isActive: boolean;
  isMuted: boolean;
  onToggleMute: () => void;
};

function ShortItem({ video, isActive, isMuted, onToggleMute }: ShortItemProps) {
  const thumbnailSrc = useThumbnailUrl(video.id);
  const openElsewhere = useCallback(() => openVideoElsewhere(video), [video]);

  return (
    <div className="flex h-full items-end gap-4">
      <div className="relative aspect-[9/16] h-full overflow-hidden rounded-xl bg-black">
        {isActive ? (
          <ShortPlayer
            video={video}
            isMuted={isMuted}
            onToggleMute={onToggleMute}
            onOpenElsewhere={openElsewhere}
          />
        ) : (
          <img src={thumbnailSrc} alt="" className="size-full object-contain" />
        )}
        <ShortInfo video={video} />
      </div>
      <ShortActions video={video} />
    </div>
  );
}

/** The folder and title over the bottom of a Short, where YouTube shows channel and title. */
function ShortInfo({ video }: { video: VideoSummary }) {
  return (
    <div className="pointer-events-none absolute inset-x-0 bottom-0 bg-gradient-to-t from-black/60 to-transparent px-3 pt-20 pb-5 text-white">
      <div className="flex items-center gap-2">
        <Avatar name={video.folderName} colorKey={video.folderId ?? video.folderName} size={32} />
        {video.folderId ? (
          <Link
            to={folderPath(video.folderId)}
            className="pointer-events-auto truncate text-sm font-medium hover:underline"
          >
            {video.folderName}
          </Link>
        ) : (
          <span className="truncate text-sm font-medium">{video.folderName}</span>
        )}
      </div>
      <p className="mt-2 line-clamp-2 text-sm break-words">{video.title}</p>
    </div>
  );
}

function NavButton({
  icon,
  label,
  disabled,
  onClick,
}: {
  icon: string;
  label: string;
  disabled: boolean;
  onClick: () => void;
}) {
  return (
    <button
      type="button"
      aria-label={label}
      title={label}
      disabled={disabled}
      onClick={onClick}
      className="flex size-14 items-center justify-center rounded-full bg-chip outline-none hover:bg-chip-hover focus-visible:ring-2 focus-visible:ring-focus disabled:invisible"
    >
      <Icon name={icon} />
    </button>
  );
}
