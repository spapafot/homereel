import { useConnectDrive } from '@/hooks/use-auth';
import { api } from '@/lib/api';
import { toAppError } from '@/lib/app-error';

import { Button } from './Button';

/** Shown where sign-in would be, in builds made without a Google OAuth client. */
export const DRIVE_UNAVAILABLE_MESSAGE = 'Drive connect is not available. Contact Homereel.';

type SignInButtonProps = {
  compact?: boolean;
  label?: string;
};

/** YouTube's outlined "Sign in" button; starts the Google sign-in in the system browser. */
export function SignInButton({ compact = false, label = 'Sign in' }: SignInButtonProps) {
  const connect = useConnectDrive();
  const error = connect.error ? toAppError(connect.error) : null;

  return (
    <div className="flex flex-col items-center gap-2">
      <div className="flex items-center gap-2">
        <Button
          variant="outline"
          icon="account_circle"
          onClick={() => connect.mutate()}
          disabled={connect.isPending}
        >
          {connect.isPending ? 'Waiting for browser…' : label}
        </Button>
        {connect.isPending && !compact && <Button onClick={() => api.authCancel()}>Cancel</Button>}
      </div>
      {error && error.code !== 'cancelled' && !compact && (
        <p className="text-sm text-red-600 dark:text-red-400">{error.message}</p>
      )}
    </div>
  );
}
