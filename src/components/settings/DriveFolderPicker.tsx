import { Fragment, useState } from 'react';

import { Button } from '@/components/common/Button';
import { Icon } from '@/components/common/Icon';
import { Modal } from '@/components/common/Modal';
import { useDriveFolders, useSetDriveFolder } from '@/hooks/use-library-sources';
import type { DriveFolder } from '@/lib/api';
import { toAppError } from '@/lib/app-error';

const SKELETON_ROWS = 5;

type DriveFolderPickerProps = {
  open: boolean;
  onOpenChange: (open: boolean) => void;
};

/** Browses My Drive to choose the folder the library shows, subfolders included. */
export function DriveFolderPicker({ open, onOpenChange }: DriveFolderPickerProps) {
  return (
    <Modal
      open={open}
      onOpenChange={onOpenChange}
      title="Choose a Google Drive folder"
      description="Homereel shows the videos in this folder and in every folder inside it."
      className="w-[min(92vw,520px)]"
    >
      {/* Mounting on open starts every visit at the top of My Drive. */}
      {open && <FolderBrowser onDone={() => onOpenChange(false)} />}
    </Modal>
  );
}

function FolderBrowser({ onDone }: { onDone: () => void }) {
  // The folders opened below My Drive, outermost first.
  const [trail, setTrail] = useState<DriveFolder[]>([]);
  const current = trail.at(-1);
  const folders = useDriveFolders(current?.id, true);
  const setDriveFolder = useSetDriveFolder();

  const choose = (folder: DriveFolder | null) => {
    setDriveFolder.mutate(folder, { onSuccess: onDone });
  };

  return (
    <div className="mt-4">
      <nav aria-label="Folder path" className="flex min-h-9 flex-wrap items-center gap-0.5 text-sm">
        <CrumbButton label="My Drive" isCurrent={!current} onClick={() => setTrail([])} />
        {trail.map((folder, index) => (
          <Fragment key={folder.id}>
            <Icon name="chevron_right" size={20} className="text-text-secondary" />
            <CrumbButton
              label={folder.name}
              isCurrent={index === trail.length - 1}
              onClick={() => setTrail(trail.slice(0, index + 1))}
            />
          </Fragment>
        ))}
      </nav>
      <div className="mt-2 h-72 overflow-y-auto rounded-lg border border-divider p-1">
        {folders.isPending &&
          Array.from({ length: SKELETON_ROWS }, (_, index) => (
            <div key={index} className="m-2 h-6 rounded bg-skeleton" aria-hidden="true" />
          ))}
        {folders.isError && (
          <p className="p-3 text-sm text-red-600 dark:text-red-400">
            {toAppError(folders.error).message}
          </p>
        )}
        {folders.isSuccess && folders.data.length === 0 && (
          <p className="p-3 text-sm text-text-secondary">No folders in here.</p>
        )}
        {folders.data?.map((folder) => (
          <button
            key={folder.id}
            type="button"
            onClick={() => setTrail([...trail, folder])}
            className="flex h-10 w-full items-center gap-3 rounded-lg px-2 text-left text-sm outline-none hover:bg-hover focus-visible:ring-2 focus-visible:ring-focus"
          >
            <Icon name="folder" className="text-text-secondary" />
            <span className="min-w-0 flex-1 truncate">{folder.name}</span>
            <Icon name="chevron_right" className="text-text-secondary" />
          </button>
        ))}
      </div>
      <div className="mt-6 flex flex-wrap items-center justify-end gap-2">
        <Button onClick={onDone} disabled={setDriveFolder.isPending}>
          Cancel
        </Button>
        {/* At the top of My Drive there is no folder to pick, so the choice is all of Drive. */}
        <Button
          variant="primary"
          disabled={setDriveFolder.isPending}
          onClick={() => choose(current ?? null)}
        >
          {current ? `Use “${current.name}”` : 'Use all of Google Drive'}
        </Button>
      </div>
    </div>
  );
}

function CrumbButton({
  label,
  isCurrent,
  onClick,
}: {
  label: string;
  isCurrent: boolean;
  onClick: () => void;
}) {
  return (
    <button
      type="button"
      onClick={onClick}
      aria-current={isCurrent ? 'location' : undefined}
      className="max-w-48 truncate rounded px-1.5 py-1 outline-none hover:bg-hover focus-visible:ring-2 focus-visible:ring-focus aria-[current]:font-medium"
    >
      {label}
    </button>
  );
}
