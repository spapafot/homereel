import { useShortsFeed } from '@/hooks/use-videos';
import type { HomeMix } from '@/lib/api';

import { ShortCard } from './ShortCard';
import { ShortsIcon } from './ShortsIcon';

/** Enough cards for the widest row; the ones that don't fit fall into zero-height rows. */
const SHELF_ITEMS = 6;

/** Home's row of Shorts in Home's mix, as YouTube shows between rows of the feed. */
export function ShortsShelf({ folderId, mix }: { folderId?: string; mix: HomeMix }) {
  const feed = useShortsFeed(folderId, mix);
  const shorts = feed.data?.pages[0]?.videos.slice(0, SHELF_ITEMS) ?? [];
  // Nothing shows while loading, so a folder without Shorts never flashes an empty shelf.
  if (shorts.length === 0) return null;

  return (
    <section aria-labelledby="shorts-shelf-title">
      <h2 id="shorts-shelf-title" className="mb-4 flex items-center gap-2 text-xl font-bold">
        <span className="text-progress">
          <ShortsIcon filled size={28} />
        </span>
        Shorts
      </h2>
      <div className="@container">
        <div className="grid grid-cols-2 grid-rows-[auto] gap-x-4 overflow-hidden [grid-auto-rows:0] @[600px]:grid-cols-3 @[800px]:grid-cols-4 @[1000px]:grid-cols-5 @[1200px]:grid-cols-6">
          {shorts.map((video) => (
            <ShortCard key={video.id} video={video} folderId={folderId} />
          ))}
        </div>
      </div>
    </section>
  );
}
