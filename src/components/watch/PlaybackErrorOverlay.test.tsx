import { render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';

import { PlaybackErrorOverlay } from './PlaybackErrorOverlay';

vi.mock('@/lib/platform', () => ({ IS_MAC: true }));

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn().mockResolvedValue(undefined),
}));

describe('PlaybackErrorOverlay', () => {
  describe('on a Mac', () => {
    it('should not offer the Microsoft Store when only the sound can be decoded', () => {
      render(
        <PlaybackErrorOverlay
          error="noPicture"
          source="local"
          onRetry={vi.fn()}
          onOpenElsewhere={vi.fn()}
        />,
      );

      expect(screen.queryByRole('button', { name: 'Get HEVC Video Extensions' })).toBeNull();
      expect(screen.queryByText(/Microsoft Store/)).toBeNull();
      expect(screen.getByRole('button', { name: 'Open in another player' })).toBeInTheDocument();
    });
  });
});
