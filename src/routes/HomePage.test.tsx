import { invoke } from '@tauri-apps/api/core';
import { screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it, vi } from 'vitest';

import { Logo } from '@/components/common/Logo';
import type { VideoListRequest, VideoPage } from '@/lib/api';
import { localDay } from '@/lib/home-mix';
import { makeVideo } from '@/test/fixtures';
import { renderWithProviders } from '@/test/render';

import { HomePage } from './HomePage';

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
  convertFileSrc: (path: string, protocol: string) => `http://${protocol}.localhost/${path}`,
}));

function serveHome(page: VideoPage) {
  vi.mocked(invoke).mockImplementation(async (command) => {
    switch (command) {
      case 'auth_status':
        return { configured: true, connected: true, email: null, displayName: null };
      case 'folders_list':
      case 'library_list':
        return [];
      case 'videos_list':
        return page;
      case 'shorts_list':
        return { videos: [], nextPageToken: null };
      default:
        return { keyConfigured: false, model: 'gemini-3.8-flash' };
    }
  });
}

function requestedSeeds(): number[] {
  return vi
    .mocked(invoke)
    .mock.calls.filter(([command]) => command === 'videos_list')
    .map(([, args]) => (args as { request: VideoListRequest }).request.mix?.seed ?? -1);
}

const FEED_VIDEO = makeVideo({ id: 'feed', title: 'Beach day' });

describe('HomePage', () => {
  afterEach(() => vi.mocked(invoke).mockReset());

  it('should list a mix of the library with its On this day shelf', async () => {
    serveHome({
      videos: [FEED_VIDEO],
      nextPageToken: null,
      onThisDay: [makeVideo({ id: 'snow', title: 'Snow day', recordedDate: '2019-10-18' })],
    });

    renderWithProviders(<HomePage />);

    const heading = await screen.findByRole('heading', { name: 'On this day' });
    expect(within(heading.closest('section')!).getByText('Snow day')).toBeInTheDocument();
    expect(invoke).toHaveBeenCalledWith('videos_list', {
      request: {
        folderId: undefined,
        mix: { seed: expect.any(Number), today: localDay(new Date()) },
        pageToken: undefined,
      },
    });
  });

  it('should leave out the On this day shelf when nothing was recorded on this day', async () => {
    serveHome({ videos: [FEED_VIDEO], nextPageToken: null });

    renderWithProviders(<HomePage />);

    expect(await screen.findByText('Beach day')).toBeInTheDocument();
    expect(screen.queryByRole('heading', { name: 'On this day' })).not.toBeInTheDocument();
  });

  it('should list a new mix when the logo is clicked', async () => {
    serveHome({ videos: [FEED_VIDEO], nextPageToken: null });
    renderWithProviders(
      <>
        <Logo />
        <HomePage />
      </>,
    );
    await screen.findByText('Beach day');

    await userEvent.click(screen.getByRole('link', { name: 'Homereel home' }));

    await waitFor(() => expect(new Set(requestedSeeds()).size).toBe(2));
  });
});
