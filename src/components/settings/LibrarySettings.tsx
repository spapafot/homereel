import { useState, type ReactNode } from 'react';

import { AddFolderButton } from '@/components/common/AddFolderButton';
import { Avatar } from '@/components/common/Avatar';
import { Button } from '@/components/common/Button';
import { Icon } from '@/components/common/Icon';
import { IconButton } from '@/components/common/IconButton';
import { SignInButton } from '@/components/common/SignInButton';
import { useAuthStatus, useDisconnectDrive } from '@/hooks/use-auth';
import { useLibrarySources, useRemoveLocalFolder } from '@/hooks/use-library-sources';

import { DriveFolderPicker } from './DriveFolderPicker';

function SourceHeading({ children }: { children: ReactNode }) {
  return <h3 className="mb-3 text-base font-medium">{children}</h3>;
}

/** Where the library's videos come from: Google Drive and folders on this computer. */
export function LibrarySettings() {
  return (
    <div className="flex flex-col gap-8">
      <DriveSource />
      <LocalSources />
      <p className="text-xs text-text-secondary">
        Homereel only reads your files. It never renames, moves or deletes anything.
      </p>
    </div>
  );
}

function DriveSource() {
  const auth = useAuthStatus();
  const disconnect = useDisconnectDrive();
  const sources = useLibrarySources();
  const [isPickerOpen, setIsPickerOpen] = useState(false);

  if (!auth.data?.configured) {
    return (
      <div>
        <SourceHeading>Google Drive</SourceHeading>
        <p className="text-sm text-text-secondary">
          To watch videos from Google Drive, add <code>src-tauri/google-oauth-client.json</code> as
          described in <code>docs/setup.md</code>, then rebuild Homereel.
        </p>
      </div>
    );
  }
  if (!auth.data.connected) {
    return (
      <div>
        <SourceHeading>Google Drive</SourceHeading>
        <div className="flex flex-col items-start gap-3">
          <p className="text-sm text-text-secondary">
            Sign in with Google to add the videos in your Drive to Homereel.
          </p>
          <SignInButton />
        </div>
      </div>
    );
  }

  const email = auth.data.email ?? '';
  const name = auth.data.displayName ?? (email || 'Google account');
  const driveFolder = sources.data?.driveFolder ?? null;
  return (
    <div>
      <SourceHeading>Google Drive</SourceHeading>
      <div className="flex flex-wrap items-center justify-between gap-4">
        <div className="flex min-w-0 items-center gap-4">
          <Avatar name={name} colorKey={email || name} size={48} />
          <div className="min-w-0">
            <p className="truncate text-base">{name}</p>
            {email && <p className="truncate text-sm text-text-secondary">{email}</p>}
          </div>
        </div>
        <Button onClick={() => disconnect.mutate()} disabled={disconnect.isPending}>
          Disconnect
        </Button>
      </div>
      <div className="mt-4 flex flex-wrap items-center justify-between gap-4">
        <div className="flex min-w-0 items-center gap-4">
          <div className="flex w-12 shrink-0 justify-center text-text-secondary">
            <Icon name="folder" />
          </div>
          <div className="min-w-0">
            <p className="truncate text-sm">{driveFolder?.name ?? 'All of Google Drive'}</p>
            <p className="text-xs text-text-secondary">
              {driveFolder
                ? 'Videos in this folder and every folder inside it'
                : 'Every video in your Drive'}
            </p>
          </div>
        </div>
        <Button onClick={() => setIsPickerOpen(true)}>
          {driveFolder ? 'Change folder' : 'Choose a folder'}
        </Button>
      </div>
      <DriveFolderPicker open={isPickerOpen} onOpenChange={setIsPickerOpen} />
    </div>
  );
}

function LocalSources() {
  const sources = useLibrarySources();
  const removeFolder = useRemoveLocalFolder();
  const folders = sources.data?.localFolders ?? [];

  return (
    <div>
      <SourceHeading>Folders on this computer</SourceHeading>
      {folders.length === 0 ? (
        <p className="mb-3 text-sm text-text-secondary">
          Add folders of videos, such as your Videos folder or an external drive. Their subfolders
          are included.
        </p>
      ) : (
        <ul className="mb-3 flex flex-col gap-1">
          {folders.map((folder) => (
            <li key={folder.path} className="flex items-center justify-between gap-4 py-1">
              <div className="flex min-w-0 items-center gap-4">
                <div className="flex w-12 shrink-0 justify-center text-text-secondary">
                  <Icon name={folder.available ? 'folder' : 'folder_off'} />
                </div>
                <div className="min-w-0">
                  <p className="truncate text-sm">{folder.name}</p>
                  <p className="truncate text-xs text-text-secondary" title={folder.path}>
                    {folder.available ? folder.path : `Not found: ${folder.path}`}
                  </p>
                </div>
              </div>
              <IconButton
                icon="close"
                label={`Remove ${folder.name}`}
                disabled={removeFolder.isPending}
                onClick={() => removeFolder.mutate(folder.path)}
              />
            </li>
          ))}
        </ul>
      )}
      <AddFolderButton />
    </div>
  );
}
