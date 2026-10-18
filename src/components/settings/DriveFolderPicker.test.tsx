import { invoke } from '@tauri-apps/api/core';
import { screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import type { DriveFolder } from '@/lib/api';
import { renderWithProviders } from '@/test/render';

import { DriveFolderPicker } from './DriveFolderPicker';

vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn() }));

const CHILDREN: Record<string, DriveFolder[]> = {
  root: [
    { id: 'vids', name: 'VIDS' },
    { id: 'docs', name: 'Documents' },
  ],
  vids: [{ id: 'vids0', name: 'VIDS 0' }],
};

function renderPicker() {
  const onOpenChange = vi.fn();
  renderWithProviders(<DriveFolderPicker open onOpenChange={onOpenChange} />);
  return { onOpenChange };
}

describe('DriveFolderPicker', () => {
  beforeEach(() => {
    vi.mocked(invoke).mockImplementation(async (command, args) => {
      if (command === 'drive_folders_list') {
        const { parentId } = args as { parentId?: string };
        return CHILDREN[parentId ?? 'root'] ?? [];
      }
      if (command === 'library_drive_folder_set') {
        return { driveFolder: (args as { folder: DriveFolder | null }).folder, localFolders: [] };
      }
      throw new Error(`Unexpected command ${command}`);
    });
  });
  afterEach(() => vi.mocked(invoke).mockReset());

  it('should choose a folder opened from My Drive', async () => {
    const user = userEvent.setup();
    const { onOpenChange } = renderPicker();

    await user.click(await screen.findByRole('button', { name: 'VIDS' }));
    expect(await screen.findByRole('button', { name: 'VIDS 0' })).toBeInTheDocument();
    await user.click(screen.getByRole('button', { name: 'Use “VIDS”' }));

    expect(invoke).toHaveBeenCalledWith('library_drive_folder_set', {
      folder: { id: 'vids', name: 'VIDS' },
    });
    expect(onOpenChange).toHaveBeenCalledWith(false);
  });

  it('should go back up through the folder path', async () => {
    const user = userEvent.setup();
    renderPicker();

    await user.click(await screen.findByRole('button', { name: 'VIDS' }));
    await user.click(await screen.findByRole('button', { name: 'My Drive' }));

    expect(await screen.findByRole('button', { name: 'Documents' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Use all of Google Drive' })).toBeInTheDocument();
  });

  it('should use all of Google Drive from the top of My Drive', async () => {
    const user = userEvent.setup();
    renderPicker();

    await user.click(await screen.findByRole('button', { name: 'Use all of Google Drive' }));

    expect(invoke).toHaveBeenCalledWith('library_drive_folder_set', { folder: null });
  });
});
