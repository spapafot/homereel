import { Link } from 'react-router';

import { Avatar } from '@/components/common/Avatar';
import { LibraryGate } from '@/components/common/LibraryGate';
import { EmptyState } from '@/components/common/EmptyState';
import { ErrorState } from '@/components/common/ErrorState';
import { useFolders } from '@/hooks/use-videos';
import { folderPath } from '@/lib/paths';

const SKELETON_COUNT = 8;

export function FoldersPage() {
  return (
    <LibraryGate>
      <div className="px-6 py-6">
        <h1 className="text-4xl font-bold">Folders</h1>
        <p className="mt-2 text-sm text-text-secondary">
          Folders that contain videos, with the most videos first.
        </p>
        <FolderGrid />
      </div>
    </LibraryGate>
  );
}

function FolderGrid() {
  const folders = useFolders();

  if (folders.isError) {
    return <ErrorState error={folders.error} onRetry={() => folders.refetch()} />;
  }
  if (folders.isSuccess && folders.data.length === 0) {
    return <EmptyState icon="folder_off" title="No folders with videos yet" />;
  }

  return (
    <div className="mt-6 grid grid-cols-[repeat(auto-fill,minmax(200px,1fr))] gap-4">
      {folders.isPending &&
        Array.from({ length: SKELETON_COUNT }, (_, index) => (
          <div key={index} className="flex flex-col items-center gap-3 p-4" aria-hidden="true">
            <div className="size-[136px] rounded-full bg-skeleton" />
            <div className="h-5 w-3/4 rounded bg-skeleton" />
          </div>
        ))}
      {folders.data?.map((folder) => (
        <Link
          key={folder.id}
          to={folderPath(folder.id)}
          className="flex flex-col items-center gap-3 rounded-xl p-4 text-center outline-none hover:bg-hover focus-visible:ring-2 focus-visible:ring-focus"
        >
          <Avatar name={folder.name} colorKey={folder.id} size={136} />
          <div className="min-w-0">
            <p className="line-clamp-2 text-base font-medium break-words">{folder.name}</p>
            <p className="text-sm text-text-secondary">
              {folder.videoCount} {folder.videoCount === 1 ? 'video' : 'videos'}
            </p>
          </div>
        </Link>
      ))}
    </div>
  );
}
