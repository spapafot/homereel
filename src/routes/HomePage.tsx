import { useSearchParams } from 'react-router';

import { DriveGate } from '@/components/common/DriveGate';
import { ChipBar, type Chip } from '@/components/video/ChipBar';
import { DEFAULT_VIDEO_ORDER } from '@/components/video/order-chips';
import { VideoCard } from '@/components/video/VideoCard';
import { VideoFeed } from '@/components/video/VideoFeed';
import { useLibraryList } from '@/hooks/use-library';
import { useFolders } from '@/hooks/use-videos';

const ALL_CHIP_ID = 'all';
const MAX_FOLDER_CHIPS = 20;
const SHELF_ITEMS = 5;

/** Home: newest videos, with folder chips where YouTube shows topic chips. */
export function HomePage() {
  return (
    <DriveGate>
      <HomeFeed />
    </DriveGate>
  );
}

function HomeFeed() {
  const [params, setParams] = useSearchParams();
  const folders = useFolders();
  const folderId = params.get('folder') ?? undefined;
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
      <div className="pt-6 pb-12">
        <VideoFeed request={{ folderId, order: DEFAULT_VIDEO_ORDER }} />
      </div>
    </div>
  );
}

/** One row of partly watched videos above the feed, as the spec's "Continue watching". */
function ContinueWatchingShelf() {
  const continueWatching = useLibraryList('continueWatching');
  const videos = continueWatching.data ?? [];
  if (videos.length === 0) return null;

  return (
    <section className="border-b border-divider pt-6 pb-8">
      <h2 className="mb-4 text-xl font-bold">Continue watching</h2>
      <div className="@container">
        {/* Only the first row is shown; extra cards fall into zero-height rows. */}
        <div className="grid grid-cols-1 grid-rows-[auto] gap-x-4 overflow-hidden [grid-auto-rows:0] @[560px]:grid-cols-2 @[900px]:grid-cols-3 @[2000px]:grid-cols-4 @[2600px]:grid-cols-5">
          {videos.slice(0, SHELF_ITEMS).map((video) => (
            <VideoCard key={video.id} video={video} />
          ))}
        </div>
      </div>
    </section>
  );
}
