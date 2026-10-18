import { useParams, useSearchParams } from 'react-router';

import { Avatar } from '@/components/common/Avatar';
import { LibraryGate } from '@/components/common/LibraryGate';
import { ChipBar } from '@/components/video/ChipBar';
import { DEFAULT_VIDEO_ORDER, isVideoOrder, ORDER_CHIPS } from '@/components/video/order-chips';
import { VideoFeed } from '@/components/video/VideoFeed';
import { useFolders, useVideoFeed } from '@/hooks/use-videos';
import { FOLDER_KIND_LABELS } from '@/lib/video-source';

/** A folder shown the way YouTube shows a channel's Videos tab. */
export function FolderPage() {
  return (
    <LibraryGate>
      <FolderContent />
    </LibraryGate>
  );
}

function FolderContent() {
  const { folderId = '' } = useParams();
  const [params, setParams] = useSearchParams();
  const sort = params.get('sort') ?? DEFAULT_VIDEO_ORDER;
  const order = isVideoOrder(sort) ? sort : DEFAULT_VIDEO_ORDER;
  const folders = useFolders();
  // Shares its cache with the grid below, so this costs no extra request.
  const feed = useVideoFeed({ folderId, order });

  const folder = folders.data?.find((candidate) => candidate.id === folderId);
  const firstVideo = feed.data?.pages[0]?.videos[0];
  const name = folder?.name ?? firstVideo?.folderName ?? '';
  const kind = FOLDER_KIND_LABELS[folder?.source ?? firstVideo?.source ?? 'drive'];

  return (
    <div className="px-6">
      <header className="flex items-center gap-6 py-8">
        <Avatar name={name || 'Folder'} colorKey={folderId} size={120} />
        <div className="min-w-0">
          {name ? (
            <h1 className="truncate text-4xl font-bold">{name}</h1>
          ) : (
            <div className="h-10 w-64 rounded bg-skeleton" />
          )}
          <p className="mt-2 text-sm text-text-secondary">
            {folder
              ? `${kind} • ${folder.videoCount} ${folder.videoCount === 1 ? 'video' : 'videos'}`
              : kind}
          </p>
        </div>
      </header>
      <ChipBar
        sticky
        chips={ORDER_CHIPS}
        selectedId={order}
        onSelect={(id) => setParams(id === DEFAULT_VIDEO_ORDER ? {} : { sort: id })}
      />
      <div className="pt-4 pb-12">
        <VideoFeed request={{ folderId, order }} emptyTitle="This folder has no videos" />
      </div>
    </div>
  );
}
