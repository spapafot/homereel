import { useSearchParams } from 'react-router';

import { LibraryGate } from '@/components/common/LibraryGate';
import { ShortsShelf } from '@/components/shorts/ShortsShelf';
import { ChipBar, type Chip } from '@/components/video/ChipBar';
import { VideoCard } from '@/components/video/VideoCard';
import { VideoFeed } from '@/components/video/VideoFeed';
import { useHomeMix } from '@/hooks/use-home-mix';
import { useLibraryList } from '@/hooks/use-library';
import { useRichGridColumns } from '@/hooks/use-rich-grid-columns';
import { useFolders, useVideoFeed } from '@/hooks/use-videos';
import type { VideoListRequest, VideoSummary } from '@/lib/api';

const ALL_CHIP_ID = 'all';
const MAX_FOLDER_CHIPS = 20;

/**
 * Home: the library in a mix that changes when the app starts and whenever Home is clicked, with
 * folder chips where YouTube shows topic chips.
 */
export function HomePage() {
  return (
    <LibraryGate>
      <HomeFeed />
    </LibraryGate>
  );
}

function HomeFeed() {
  const [params, setParams] = useSearchParams();
  const folders = useFolders();
  const folderId = params.get('folder') ?? undefined;
  const mix = useHomeMix();
  const request = { folderId, mix };
  const chips: Chip[] = [
    { id: ALL_CHIP_ID, label: 'All' },
    ...(folders.data ?? [])
      .slice(0, MAX_FOLDER_CHIPS)
      .map((folder) => ({ id: folder.id, label: folder.name })),
  ];

  return (
    <div className="px-6">
      <ChipBar
        sticky
        chips={chips}
        selectedId={folderId ?? ALL_CHIP_ID}
        onSelect={(id) => setParams(id === ALL_CHIP_ID ? {} : { folder: id })}
      />
      {!folderId && <ContinueWatchingShelf />}
      <OnThisDayShelf request={request} />
      <div className="pt-6 pb-12">
        <VideoFeed
          request={request}
          shelf={<ShortsShelf folderId={folderId} mix={mix} />}
          rowGap="home"
        />
      </div>
    </div>
  );
}

/** One row of partly watched videos above the feed, as the spec's "Continue watching". */
function ContinueWatchingShelf() {
  const continueWatching = useLibraryList('continueWatching');
  return <VideoShelf title="Continue watching" videos={continueWatching.data ?? []} />;
}

/**
 * Videos recorded on this day in earlier years, which come with the first page of the mix. Their
 * cards' dates say how many years ago.
 */
function OnThisDayShelf({ request }: { request: Omit<VideoListRequest, 'pageToken'> }) {
  const feed = useVideoFeed(request);
  return <VideoShelf title="On this day" videos={feed.data?.pages[0]?.onThisDay ?? []} />;
}

/** One row of videos above the feed, hidden when there are none. */
function VideoShelf({ title, videos }: { title: string; videos: VideoSummary[] }) {
  const { ref: gridRef, columns } = useRichGridColumns();
  if (videos.length === 0) return null;

  return (
    <section className="border-b border-divider pt-6 pb-8">
      <h2 className="mb-4 text-xl font-bold">{title}</h2>
      {/* One row, as wide as the feed's rows below it. */}
      <div
        ref={gridRef}
        className="grid gap-x-4"
        style={{ gridTemplateColumns: `repeat(${columns}, minmax(0, 1fr))` }}
      >
        {videos.slice(0, columns).map((video) => (
          <VideoCard key={video.id} video={video} />
        ))}
      </div>
    </section>
  );
}
