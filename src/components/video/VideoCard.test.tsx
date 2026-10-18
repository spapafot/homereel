import { act, fireEvent, screen } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import { bumpThumbnailVersion } from '@/lib/thumbnail-versions';
import { makeVideo } from '@/test/fixtures';
import { renderWithProviders } from '@/test/render';

import { VideoCard } from './VideoCard';

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn().mockResolvedValue({ keyConfigured: false, model: 'gemini-3.8-flash' }),
  convertFileSrc: (path: string, protocol: string) => `http://${protocol}.localhost/${path}`,
}));

function renderCard(video = makeVideo()) {
  return renderWithProviders(<VideoCard video={video} />);
}

describe('VideoCard', () => {
  beforeEach(() => {
    vi.useFakeTimers({ toFake: ['Date'] });
    vi.setSystemTime(new Date('2026-10-17T12:00:00Z'));
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it('should link the title to the watch page and the folder in the channel slot', () => {
    renderCard();

    expect(screen.getByRole('link', { name: 'IMG_0817.m4v' })).toHaveAttribute(
      'href',
      '/watch?v=vid1&folder=fold1',
    );
    expect(screen.getByRole('link', { name: 'Holidays' })).toHaveAttribute('href', '/folder/fold1');
  });

  it('should show the folder and the date on one metadata line, as YouTube does', () => {
    renderCard();

    expect(screen.getByText('3 days ago')).toHaveTextContent('Holidays 3 days ago');
  });

  it('should show the duration only on the thumbnail badge', () => {
    renderCard();

    expect(screen.getAllByText('1:15')).toHaveLength(1);
  });

  it('should load the thumbnail from the thumb protocol', () => {
    const { container } = renderCard();

    const image = container.querySelector('img');
    expect(image).toHaveAttribute('src', 'http://thumb.localhost/vid1');
  });

  it('should show a red progress bar for partly watched videos', () => {
    renderCard(makeVideo({ lastPositionMs: 30_000, durationMs: 75_000 }));

    // The thumbnail link duplicates the title link, so it is hidden from assistive technology.
    expect(screen.getByRole('progressbar', { name: 'Watched', hidden: true })).toHaveAttribute(
      'aria-valuenow',
      '40',
    );
  });

  it('should still request a thumbnail when Drive has none, then fall back to a placeholder', () => {
    const { container } = renderCard(makeVideo({ hasThumbnail: false, width: null, height: null }));
    const images = container.querySelectorAll('img');

    expect(images.length).toBeGreaterThan(0);
    fireEvent.error(images[images.length - 1]);

    expect(container.querySelector('img')).toBeNull();
    expect(screen.getByText('movie')).toBeInTheDocument();
  });

  it('should load the new picture after the preview is regenerated, even after a failure', () => {
    const { container } = renderCard(makeVideo({ id: 'regen1', hasThumbnail: false }));
    const images = container.querySelectorAll('img');
    fireEvent.error(images[images.length - 1]);
    expect(container.querySelector('img')).toBeNull();

    act(() => bumpThumbnailVersion('regen1'));

    expect(container.querySelector('img')).toHaveAttribute(
      'src',
      'http://thumb.localhost/regen1?v=1',
    );
  });

  it('should show the folder as plain text for files shared with me', () => {
    renderCard(makeVideo({ folderId: null, folderName: 'Shared with me' }));

    expect(screen.getByText('Shared with me')).toBeInTheDocument();
    expect(screen.queryByRole('link', { name: 'Shared with me' })).toBeNull();
  });
});
