import { invoke } from '@tauri-apps/api/core';
import { screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it, vi } from 'vitest';

import type { AuthStatus, LibrarySources } from '@/lib/api';
import { renderWithProviders } from '@/test/render';

import { LibraryGate } from './LibraryGate';

vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn() }));

const NO_SOURCES: LibrarySources = { driveFolder: null, localFolders: [] };
const WITH_FOLDER: LibrarySources = {
  driveFolder: null,
  localFolders: [{ path: 'C:\\Users\\Sam\\Videos', name: 'Videos', available: true }],
};

function serve(auth: Partial<AuthStatus>, sources: LibrarySources) {
  vi.mocked(invoke).mockImplementation(async (command) => {
    switch (command) {
      case 'auth_status':
        return { configured: true, connected: false, email: null, displayName: null, ...auth };
      case 'library_sources_get':
        return sources;
      case 'library_local_folder_add':
        return WITH_FOLDER;
      default:
        throw new Error(`Unexpected command ${command}`);
    }
  });
}

function renderGate() {
  return renderWithProviders(
    <LibraryGate>
      <p>Library</p>
    </LibraryGate>,
  );
}

describe('LibraryGate', () => {
  afterEach(() => vi.mocked(invoke).mockReset());

  describe('when the library has no sources', () => {
    it('should offer a local folder and Google sign-in', async () => {
      serve({}, NO_SOURCES);

      renderGate();

      expect(await screen.findByRole('heading', { name: 'Add your videos' })).toBeInTheDocument();
      expect(screen.getByRole('button', { name: 'Add a folder' })).toBeInTheDocument();
      expect(screen.getByRole('button', { name: 'Sign in with Google' })).toBeInTheDocument();
      expect(screen.queryByText('Library')).not.toBeInTheDocument();
    });

    it('should offer only a local folder when Google sign-in isn’t set up', async () => {
      serve({ configured: false }, NO_SOURCES);

      renderGate();

      expect(await screen.findByRole('button', { name: 'Add a folder' })).toBeInTheDocument();
      expect(screen.queryByRole('button', { name: /Sign in/ })).not.toBeInTheDocument();
      expect(
        screen.getByText('Drive connect is not available. Contact Homereel.'),
      ).toBeInTheDocument();
    });

    it('should show the library once a folder is added', async () => {
      const user = userEvent.setup();
      serve({}, NO_SOURCES);
      renderGate();

      await user.click(await screen.findByRole('button', { name: 'Add a folder' }));

      expect(await screen.findByText('Library')).toBeInTheDocument();
      expect(invoke).toHaveBeenCalledWith('library_local_folder_add');
    });
  });

  it('should show the library from local folders without a Google account', async () => {
    serve({ configured: false }, WITH_FOLDER);

    renderGate();

    expect(await screen.findByText('Library')).toBeInTheDocument();
  });

  it('should show the library when Google Drive is connected', async () => {
    serve({ connected: true }, NO_SOURCES);

    renderGate();

    expect(await screen.findByText('Library')).toBeInTheDocument();
  });
});
