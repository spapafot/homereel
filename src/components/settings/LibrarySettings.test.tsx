import { invoke } from '@tauri-apps/api/core';
import { screen } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';

import type { AuthStatus } from '@/lib/api';
import { renderWithProviders } from '@/test/render';

import { LibrarySettings } from './LibrarySettings';

vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn() }));

function serve(auth: Partial<AuthStatus>) {
  vi.mocked(invoke).mockImplementation(async (command) => {
    switch (command) {
      case 'auth_status':
        return { configured: true, connected: false, email: null, displayName: null, ...auth };
      case 'library_sources_get':
        return { driveFolder: null, localFolders: [] };
      default:
        throw new Error(`Unexpected command ${command}`);
    }
  });
}

describe('LibrarySettings', () => {
  afterEach(() => vi.mocked(invoke).mockReset());

  describe('when the build has no Google OAuth client', () => {
    it('should say Drive connect is not available instead of offering sign-in', async () => {
      serve({ configured: false });

      renderWithProviders(<LibrarySettings />);

      expect(
        await screen.findByText('Drive connect is not available. Contact Homereel.'),
      ).toBeInTheDocument();
      expect(screen.queryByRole('button', { name: /Sign in/ })).not.toBeInTheDocument();
      expect(screen.getByRole('button', { name: 'Add a folder' })).toBeInTheDocument();
    });
  });

  describe('when the build can sign in', () => {
    it('should offer Google sign-in without the unavailable message', async () => {
      serve({});

      renderWithProviders(<LibrarySettings />);

      expect(await screen.findByRole('button', { name: 'Sign in' })).toBeInTheDocument();
      expect(screen.queryByText(/Drive connect is not available/)).not.toBeInTheDocument();
    });
  });
});
