import type { ReactNode } from 'react';

import { useAuthStatus } from '@/hooks/use-auth';
import { useLibrarySources } from '@/hooks/use-library-sources';

import { AddFolderButton } from './AddFolderButton';
import { EmptyState } from './EmptyState';
import { ErrorState } from './ErrorState';
import { DRIVE_UNAVAILABLE_MESSAGE, SignInButton } from './SignInButton';

/**
 * Renders its children once the library has somewhere to list videos from: Google Drive or a
 * folder on this computer.
 */
export function LibraryGate({ children }: { children: ReactNode }) {
  const auth = useAuthStatus();
  const sources = useLibrarySources();

  if (auth.isPending || sources.isPending) return null;
  if (auth.isError) return <ErrorState error={auth.error} onRetry={() => auth.refetch()} />;
  if (sources.isError) {
    return <ErrorState error={sources.error} onRetry={() => sources.refetch()} />;
  }
  if (auth.data.connected || sources.data.localFolders.length > 0) return children;

  const canSignIn = auth.data.configured;
  return (
    <EmptyState
      icon="video_library"
      title="Add your videos"
      action={
        <div className="flex flex-col items-center gap-3">
          <div className="flex flex-wrap items-start justify-center gap-3">
            <AddFolderButton variant="outline" />
            {canSignIn && <SignInButton label="Sign in with Google" />}
          </div>
          {!canSignIn && <p className="text-sm text-text-secondary">{DRIVE_UNAVAILABLE_MESSAGE}</p>}
        </div>
      }
    >
      {canSignIn
        ? 'Choose a folder of videos on this computer, or sign in with Google to watch the videos in your Drive.'
        : 'Choose a folder of videos on this computer to watch them here.'}{' '}
      Homereel only reads your files; it never changes or deletes them.
    </EmptyState>
  );
}
