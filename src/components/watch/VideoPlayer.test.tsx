import { invoke } from '@tauri-apps/api/core';
import { cleanup, fireEvent, render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import { VideoPlayer } from './VideoPlayer';

vi.mock('@tauri-apps/api/window', () => ({
  getCurrentWindow: () => ({ setFullscreen: vi.fn().mockResolvedValue(undefined) }),
}));

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn().mockResolvedValue(undefined),
}));

const SOURCE = 'http://127.0.0.1:5000/media/secret/vid1';

function renderPlayer(overrides: Partial<Parameters<typeof VideoPlayer>[0]> = {}) {
  const props = {
    src: SOURCE,
    title: 'Holiday clip',
    hasNext: false,
    autoAdvance: false,
    onNext: vi.fn(),
    videoSource: 'drive' as const,
    onOpenElsewhere: vi.fn(),
    ...overrides,
  };
  const view = render(<VideoPlayer {...props} />);
  const video = view.container.querySelector('video')!;
  return { ...view, props, video };
}

function failWith(video: HTMLVideoElement, code: number) {
  Object.defineProperty(video, 'error', { value: { code }, configurable: true });
  fireEvent.error(video);
}

/** Chromium reports a width of 0 when it dropped a video track it can't decode. */
function loadMetadata(video: HTMLVideoElement, videoWidth: number) {
  Object.defineProperty(video, 'videoWidth', { value: videoWidth, configurable: true });
  fireEvent.loadedMetadata(video);
}

describe('VideoPlayer', () => {
  beforeEach(() => {
    vi.spyOn(HTMLMediaElement.prototype, 'play').mockResolvedValue(undefined);
    vi.spyOn(HTMLMediaElement.prototype, 'pause').mockImplementation(() => undefined);
    vi.spyOn(HTMLMediaElement.prototype, 'load').mockImplementation(() => undefined);
  });

  afterEach(() => {
    // Unmount while the media mocks are still in place; the player detaches its source then.
    cleanup();
    vi.restoreAllMocks();
  });

  it('should play the proxy stream', () => {
    const { video } = renderPlayer();

    expect(video).toHaveAttribute('src', SOURCE);
  });

  it('should offer Open in Google Drive when the format cannot be played', async () => {
    const user = userEvent.setup();
    const { video, props } = renderPlayer();

    failWith(video, 4);
    await user.click(screen.getByRole('button', { name: 'Open in Google Drive' }));

    expect(screen.getByText('Homereel can’t play this video’s format')).toBeInTheDocument();
    expect(props.onOpenElsewhere).toHaveBeenCalledOnce();
  });

  it('should offer another player when a local video cannot be played', async () => {
    const user = userEvent.setup();
    const { video, props } = renderPlayer({ videoSource: 'local' });

    failWith(video, 4);
    await user.click(screen.getByRole('button', { name: 'Open in another player' }));

    expect(screen.getByText('Open it in your default video player.')).toBeInTheDocument();
    expect(props.onOpenElsewhere).toHaveBeenCalledOnce();
  });

  it('should offer a retry after a network error', () => {
    const { video } = renderPlayer();

    failWith(video, 2);

    expect(screen.getByText('The video stopped loading')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Retry' })).toBeInTheDocument();
  });

  it('should toggle playback with the k shortcut like YouTube', async () => {
    const user = userEvent.setup();
    renderPlayer();

    await user.keyboard('k');

    expect(HTMLMediaElement.prototype.play).toHaveBeenCalled();
  });

  it('should go to the next video when playback ends with auto-advance on', () => {
    const { video, props } = renderPlayer({ hasNext: true, autoAdvance: true });

    fireEvent.ended(video);

    expect(props.onNext).toHaveBeenCalledOnce();
  });

  it('should stay on the video when playback ends with auto-advance off', () => {
    const { video, props } = renderPlayer({ hasNext: true, autoAdvance: false });

    fireEvent.ended(video);

    expect(props.onNext).not.toHaveBeenCalled();
    expect(screen.getByRole('button', { name: 'Next (Shift+N)' })).toBeInTheDocument();
  });

  it('should hide the Next button when there is no next video', () => {
    renderPlayer({ hasNext: false });

    expect(screen.queryByRole('button', { name: 'Next (Shift+N)' })).toBeNull();
  });

  describe('when only the sound can be decoded', () => {
    it('should stop and point to the HEVC codec instead of playing sound over black', async () => {
      const user = userEvent.setup();
      const { video } = renderPlayer();

      loadMetadata(video, 0);
      await user.click(screen.getByRole('button', { name: 'Get HEVC Video Extensions' }));

      expect(
        screen.getByText('Homereel can play this video’s sound but not its picture'),
      ).toBeInTheDocument();
      expect(HTMLMediaElement.prototype.pause).toHaveBeenCalled();
      expect(invoke).toHaveBeenCalledWith('playback_open_hevc_store');
    });

    it('should ask for a restart instead of a retry, since WebView2 finds codecs at startup', () => {
      const { video } = renderPlayer();

      loadMetadata(video, 0);

      expect(screen.getByText(/close and reopen Homereel/)).toBeInTheDocument();
      expect(screen.queryByRole('button', { name: /retry|try again/i })).toBeNull();
    });
  });

  it('should keep playing when the picture can be decoded', () => {
    const { video } = renderPlayer();

    loadMetadata(video, 1920);

    expect(screen.queryByText(/sound but not its picture/)).toBeNull();
    expect(HTMLMediaElement.prototype.pause).not.toHaveBeenCalled();
  });
});
