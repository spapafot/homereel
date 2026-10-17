import { toAppError } from '@/lib/app-error';

import { Button } from './Button';
import { EmptyState } from './EmptyState';
import { SignInButton } from './SignInButton';

type ErrorStateProps = {
  error: unknown;
  onRetry?: () => void;
};

export function ErrorState({ error, onRetry }: ErrorStateProps) {
  const appError = toAppError(error);

  if (appError.code === 'auth_revoked' || appError.code === 'not_connected') {
    return (
      <EmptyState icon="lock" title="Reconnect Google Drive" action={<SignInButton />}>
        {appError.message}
      </EmptyState>
    );
  }

  const isOffline = appError.code === 'offline';
  return (
    <EmptyState
      icon={isOffline ? 'cloud_off' : 'error'}
      title={isOffline ? 'You’re offline' : 'Something went wrong'}
      action={onRetry && <Button onClick={onRetry}>Retry</Button>}
    >
      {appError.message}
    </EmptyState>
  );
}
