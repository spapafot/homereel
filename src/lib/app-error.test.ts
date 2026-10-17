import { describe, expect, it } from 'vitest';

import { toAppError } from './app-error';

describe('toAppError', () => {
  it('should pass through errors returned by Rust commands', () => {
    const rustError = { code: 'auth_revoked', message: 'Reconnect Google Drive.' };

    const result = toAppError(rustError);

    expect(result).toEqual(rustError);
  });

  it('should wrap JavaScript errors and strings as internal errors', () => {
    expect(toAppError(new Error('boom'))).toEqual({ code: 'internal', message: 'boom' });
    expect(toAppError('IPC failed')).toEqual({ code: 'internal', message: 'IPC failed' });
  });

  it('should fall back to a generic message for unknown values', () => {
    const result = toAppError(42);

    expect(result).toEqual({ code: 'internal', message: 'Something went wrong.' });
  });
});
