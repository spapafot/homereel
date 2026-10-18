import { invoke } from '@tauri-apps/api/core';
import { screen } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';

import type { ShortsListRequest } from '@/lib/api';
import { makeVideo } from '@/test/fixtures';
import { renderWithProviders } from '@/test/render';

import { ShortsRow } from './ShortsRow';

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
  convertFileSrc: (path: string, protocol: string) => `http://${protocol}.localhost/${path}`,
}));

function serveShorts() {
  vi.mocked(invoke).mockImplementation(async (command, args) => {
    if (command !== 'shorts_list') return { keyConfigured: false, model: 'gemini-3.8-flash' };
    const { request } = args as { request: ShortsListRequest };
    return {
      videos: [
        makeVideo({ id: 'playing', title: 'Playing now', folderId: request.folderId }),
        makeVideo({ id: 's2', title: 'Sandcastle', folderId: request.folderId }),
      ],
      nextPageToken: null,
    };
  });
}

describe('ShortsRow', () => {
  afterEach(() => vi.mocked(invoke).mockReset());

  it('should suggest the folder’s other Shorts and open them in the Shorts player', async () => {
    serveShorts();

    renderWithProviders(<ShortsRow folderId="fold1" currentId="playing" />);

    expect(await screen.findByRole('link', { name: 'Sandcastle' })).toHaveAttribute(
      'href',
      '/shorts/s2?folder=fold1',
    );
    expect(screen.queryByRole('link', { name: 'Playing now' })).toBeNull();
    expect(invoke).toHaveBeenCalledWith('shorts_list', {
      request: { folderId: 'fold1', pageToken: undefined },
    });
  });

  it('should suggest Shorts from all of Drive without a folder', async () => {
    serveShorts();

    renderWithProviders(<ShortsRow currentId="playing" />);

    expect(await screen.findByRole('link', { name: 'Sandcastle' })).toHaveAttribute(
      'href',
      '/shorts/s2',
    );
  });
});
