import { invoke } from '@tauri-apps/api/core';
import { cleanup, screen, within } from '@testing-library/react';
import { Route, Routes } from 'react-router';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import type { ShortsListRequest, VideoPage } from '@/lib/api';
import { makeVideo } from '@/test/fixtures';
import { renderWithProviders } from '@/test/render';

import { ShortsPage } from './ShortsPage';

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
  convertFileSrc: (path: string, protocol: string) => `http://${protocol}.localhost/${path}`,
}));

function short(id: string) {
  return makeVideo({ id, title: `Clip ${id}`, width: 1080, height: 1920, durationMs: 20_000 });
}

const PAGES: Record<string, VideoPage> = {
  first: { videos: [short('a'), short('b')], nextPageToken: 'p2' },
  p2: { videos: [short('c'), short('d')], nextPageToken: null },
};

function serveLibrary() {
  vi.mocked(invoke).mockImplementation(async (command, args) => {
    switch (command) {
      case 'auth_status':
        return { configured: true, connected: true, email: null, displayName: null };
      case 'shorts_list': {
        const { request } = args as { request: ShortsListRequest };
        return PAGES[request.pageToken ?? 'first'];
      }
      case 'video_get':
        return makeVideo({ id: 'x', title: 'Landscape clip' });
      case 'playback_source':
        return { url: 'http://127.0.0.1:5000/media/secret/clip' };
      default:
        return { keyConfigured: false, model: 'gemini-3.8-flash' };
    }
  });
}

function renderShorts(path: string) {
  return renderWithProviders(
    <Routes>
      <Route path="/shorts/:videoId?" element={<ShortsPage />} />
    </Routes>,
    { initialPath: path },
  );
}

describe('ShortsPage', () => {
  beforeEach(() => {
    vi.spyOn(HTMLMediaElement.prototype, 'pause').mockImplementation(() => undefined);
    vi.spyOn(HTMLMediaElement.prototype, 'load').mockImplementation(() => undefined);
    serveLibrary();
  });

  afterEach(() => {
    cleanup();
    vi.restoreAllMocks();
    vi.mocked(invoke).mockReset();
  });

  it('should open at the linked Short after loading the pages before it', async () => {
    renderShorts('/shorts/c');

    const current = await screen.findByRole('region', { name: 'Clip c' });

    expect(current.querySelector('video')).not.toBeNull();
    expect(screen.getByRole('button', { name: 'Previous Short' })).toBeEnabled();
    expect(screen.getByRole('button', { name: 'Next Short' })).toBeEnabled();
  });

  it('should play a linked video that is not a Short ahead of the feed', async () => {
    renderShorts('/shorts/x');

    const current = await screen.findByRole('region', { name: 'Landscape clip' });

    expect(current.querySelector('video')).not.toBeNull();
    expect(screen.getAllByRole('region')[0]).toBe(current);
    expect(screen.getByRole('button', { name: 'Previous Short' })).toBeDisabled();
  });

  it('should give each nearby Short its folder and buttons', async () => {
    renderShorts('/shorts/a');

    const current = await screen.findByRole('region', { name: 'Clip a' });

    expect(within(current).getByRole('link', { name: 'Holidays' })).toHaveAttribute(
      'href',
      '/folder/fold1',
    );
    expect(within(current).getByRole('button', { name: 'Favorite' })).toHaveAttribute(
      'aria-pressed',
      'false',
    );
  });
});
