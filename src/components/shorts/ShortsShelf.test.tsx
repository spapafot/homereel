import { invoke } from '@tauri-apps/api/core';
import { screen, waitFor } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';

import type { HomeMix, VideoPage } from '@/lib/api';
import { makeVideo } from '@/test/fixtures';
import { renderWithProviders } from '@/test/render';

import { ShortsShelf } from './ShortsShelf';

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
  convertFileSrc: (path: string, protocol: string) => `http://${protocol}.localhost/${path}`,
}));

const MIX: HomeMix = { seed: 7, today: '2026-10-18' };

function serveShorts(page: VideoPage) {
  vi.mocked(invoke).mockImplementation(async (command) =>
    command === 'shorts_list' ? page : { keyConfigured: false, model: 'gemini-3.8-flash' },
  );
}

describe('ShortsShelf', () => {
  afterEach(() => vi.mocked(invoke).mockReset());

  it('should link each Short to the Shorts player within the shelf’s folder', async () => {
    serveShorts({
      videos: [makeVideo({ id: 's1', title: 'Beach day', width: 1080, height: 1920 })],
      nextPageToken: null,
    });

    renderWithProviders(<ShortsShelf folderId="fold1" mix={MIX} />);

    expect(await screen.findByRole('heading', { name: 'Shorts' })).toBeInTheDocument();
    expect(screen.getByRole('link', { name: 'Beach day' })).toHaveAttribute(
      'href',
      '/shorts/s1?folder=fold1',
    );
  });

  it('should list the folder’s Shorts in Home’s mix', async () => {
    serveShorts({ videos: [], nextPageToken: null });

    renderWithProviders(<ShortsShelf folderId="fold1" mix={MIX} />);

    await waitFor(() =>
      expect(invoke).toHaveBeenCalledWith('shorts_list', {
        request: { folderId: 'fold1', mix: MIX, pageToken: undefined },
      }),
    );
  });

  it('should show nothing when the folder has no Shorts', async () => {
    serveShorts({ videos: [], nextPageToken: null });

    const { container } = renderWithProviders(<ShortsShelf folderId="fold1" mix={MIX} />);

    await waitFor(() => expect(invoke).toHaveBeenCalledWith('shorts_list', expect.anything()));
    expect(container).toBeEmptyDOMElement();
  });
});
