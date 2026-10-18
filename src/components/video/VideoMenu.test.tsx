import { invoke } from '@tauri-apps/api/core';
import { screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import { makeVideo } from '@/test/fixtures';
import { renderWithProviders } from '@/test/render';

import { VideoMenu } from './VideoMenu';

vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn() }));

async function openMenu() {
  const user = userEvent.setup();
  await user.click(screen.getByRole('button', { name: 'Action menu' }));
  return user;
}

describe('VideoMenu', () => {
  beforeEach(() => {
    vi.mocked(invoke).mockImplementation(async (command) =>
      command === 'gemini_settings_get' ? { keyConfigured: false } : undefined,
    );
  });
  afterEach(() => vi.mocked(invoke).mockReset());

  it('should open a Drive video in Google Drive', async () => {
    renderWithProviders(<VideoMenu video={makeVideo({ source: 'drive' })} />);
    const user = await openMenu();

    await user.click(await screen.findByRole('menuitem', { name: 'Open in Google Drive' }));

    expect(invoke).toHaveBeenCalledWith('playback_open_external', { driveId: 'vid1' });
  });

  it('should show a local video in its folder', async () => {
    renderWithProviders(<VideoMenu video={makeVideo({ id: 'local-1', source: 'local' })} />);
    const user = await openMenu();

    await user.click(await screen.findByRole('menuitem', { name: 'Show in folder' }));

    expect(invoke).toHaveBeenCalledWith('video_show_in_folder', { driveId: 'local-1' });
    expect(screen.queryByRole('menuitem', { name: 'Open in Google Drive' })).toBeNull();
  });
});
