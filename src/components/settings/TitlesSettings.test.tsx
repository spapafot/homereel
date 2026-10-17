import { invoke } from '@tauri-apps/api/core';
import { screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import type { TitleQueueStatus } from '@/lib/api';
import { renderWithProviders } from '@/test/render';

import { TitlesSettings } from './TitlesSettings';

vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn() }));

const IDLE: TitleQueueStatus = {
  running: false,
  scanning: false,
  stopping: false,
  total: 0,
  completed: 0,
  failed: 0,
  pauseReason: null,
};

function mockBackend(counts: { done: number; failed: number }) {
  vi.mocked(invoke).mockImplementation(async (command, args) => {
    switch (command) {
      case 'gemini_settings_get':
        return { keyConfigured: true, model: 'gemini-3.8-flash', instructions: '' };
      case 'gemini_instructions_set':
        return {
          keyConfigured: true,
          model: 'gemini-3.8-flash',
          instructions: (args as { instructions: string }).instructions.trim(),
        };
      case 'gemini_models_list':
        return ['gemini-3.8-flash'];
      case 'titles_status':
        return { queue: IDLE, counts };
      case 'titles_start':
        return { ...IDLE, running: true, scanning: true };
      default:
        return undefined;
    }
  });
}

describe('TitlesSettings', () => {
  beforeEach(() => {
    vi.mocked(invoke).mockReset();
  });

  it('should regenerate every title after confirming', async () => {
    mockBackend({ done: 12, failed: 0 });
    const user = userEvent.setup();
    renderWithProviders(<TitlesSettings />);

    await user.click(await screen.findByRole('button', { name: 'Regenerate all titles' }));
    await user.click(screen.getByRole('button', { name: 'Regenerate all' }));

    await waitFor(() => expect(invoke).toHaveBeenCalledWith('titles_start', { scope: 'all' }));
  });

  it('should only generate missing titles from the main button', async () => {
    mockBackend({ done: 0, failed: 0 });
    const user = userEvent.setup();
    renderWithProviders(<TitlesSettings />);

    await user.click(await screen.findByRole('button', { name: 'Generate missing titles' }));

    await waitFor(() => expect(invoke).toHaveBeenCalledWith('titles_start', { scope: 'missing' }));
    expect(screen.queryByRole('button', { name: 'Regenerate all titles' })).not.toBeInTheDocument();
  });

  it('should save instructions that every title request includes', async () => {
    mockBackend({ done: 0, failed: 0 });
    const user = userEvent.setup();
    renderWithProviders(<TitlesSettings />);
    const save = await screen.findByRole('button', { name: 'Save instructions' });
    const note = 'The baby after 24 August 2024 is almost always Anna.';

    expect(save).toBeDisabled();
    await user.type(screen.getByLabelText('Instructions for Gemini'), note);
    await user.click(save);

    await waitFor(() =>
      expect(invoke).toHaveBeenCalledWith('gemini_instructions_set', { instructions: note }),
    );
    await waitFor(() => expect(save).toBeDisabled());
  });
});
