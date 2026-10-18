import { invoke } from '@tauri-apps/api/core';
import { cleanup, fireEvent, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import { makeVideo } from '@/test/fixtures';
import { renderWithProviders } from '@/test/render';

import { ShortPlayer } from './ShortPlayer';

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
  convertFileSrc: (path: string, protocol: string) => `http://${protocol}.localhost/${path}`,
}));

const SOURCE = 'http://127.0.0.1:5000/media/secret/short1';
const DURATION_SECONDS = 10;

async function renderShort() {
  const props = {
    video: makeVideo({ id: 'short1', width: 1080, height: 1920, durationMs: 10_000 }),
    isMuted: false,
    onToggleMute: vi.fn(),
    onOpenElsewhere: vi.fn(),
  };
  const view = renderWithProviders(<ShortPlayer {...props} />);
  const video = view.container.querySelector('video')!;
  await waitFor(() => expect(video).toHaveAttribute('src', SOURCE));
  return { ...view, video, props };
}

/** jsdom has no media pipeline, so time and duration are set by hand. */
function playTo(video: HTMLVideoElement, seconds: number) {
  Object.defineProperty(video, 'duration', { value: DURATION_SECONDS, configurable: true });
  Object.defineProperty(video, 'currentTime', { value: seconds, configurable: true });
  fireEvent.timeUpdate(video);
}

function recordedProgress() {
  return vi.mocked(invoke).mock.calls.filter(([command]) => command === 'video_record_progress');
}

describe('ShortPlayer', () => {
  beforeEach(() => {
    vi.spyOn(HTMLMediaElement.prototype, 'play').mockResolvedValue(undefined);
    vi.spyOn(HTMLMediaElement.prototype, 'pause').mockImplementation(() => undefined);
    vi.spyOn(HTMLMediaElement.prototype, 'load').mockImplementation(() => undefined);
    vi.mocked(invoke).mockImplementation(async (command) =>
      command === 'playback_source' ? { url: SOURCE } : undefined,
    );
  });

  afterEach(() => {
    cleanup();
    vi.restoreAllMocks();
    vi.mocked(invoke).mockReset();
  });

  it('should add a Short played to the end to History as watched', async () => {
    const { video, unmount } = await renderShort();

    playTo(video, 9.5);
    unmount();

    expect(invoke).toHaveBeenCalledWith('video_record_progress', {
      driveId: 'short1',
      positionMs: 10_000,
      durationMs: 10_000,
    });
  });

  it('should count a loop back to the start as watched', async () => {
    const { video, unmount } = await renderShort();

    playTo(video, 8);
    playTo(video, 0.2);
    unmount();

    expect(recordedProgress()[0][1]).toMatchObject({ positionMs: 10_000 });
  });

  it('should add a partly watched Short to History without a resume point', async () => {
    const { video, unmount } = await renderShort();

    playTo(video, 3);
    unmount();

    expect(recordedProgress()[0][1]).toMatchObject({ positionMs: 0, durationMs: 10_000 });
  });

  it('should leave a Short scrolled straight past out of History', async () => {
    const { video, unmount } = await renderShort();

    playTo(video, 0.4);
    unmount();

    expect(recordedProgress()).toHaveLength(0);
  });

  it('should mute with the m key', async () => {
    const user = userEvent.setup();
    const { props } = await renderShort();

    await user.keyboard('m');

    expect(props.onToggleMute).toHaveBeenCalledOnce();
  });

  it('should explain HEVC when the picture cannot be decoded', async () => {
    const { video } = await renderShort();

    Object.defineProperty(video, 'videoWidth', { value: 0, configurable: true });
    fireEvent.loadedMetadata(video);

    expect(
      screen.getByText('Homereel can play this video’s sound but not its picture'),
    ).toBeInTheDocument();
  });
});
