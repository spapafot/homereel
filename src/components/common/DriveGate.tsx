import type { ReactNode } from 'react';

import { useAuthStatus } from '@/hooks/use-auth';

import { EmptyState } from './EmptyState';
import { ErrorState } from './ErrorState';
import { SignInButton } from './SignInButton';

/** Renders its children only once Google Drive is connected. */
export function DriveGate({ children }: { children: ReactNode }) {
  const auth = useAuthStatus();

  if (auth.isPending) return null;
  if (auth.isError) return <ErrorState error={auth.error} onRetry={() => auth.refetch()} />;
  if (!auth.data.configured) {
    return (
      <EmptyState icon="key" title="Google sign-in isn’t set up yet">
        Add <code>src-tauri/google-oauth-client.json</code> as described in{' '}
        <code>docs/setup.md</code>, then restart DriveTube.
      </EmptyState>
    );
  }
  if (!auth.data.connected) {
    return (
      <EmptyState icon="video_library" title="Enjoy your Drive videos" action={<SignInButton />}>
        Sign in with Google to browse and play the videos in your Drive. DriveTube only reads your
        files; it never changes or deletes them.
      </EmptyState>
    );
  }
  return children;
}
