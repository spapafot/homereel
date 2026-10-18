import { screen } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import { makeVideo } from '@/test/fixtures';
import { renderWithProviders } from '@/test/render';

import { RelatedVideoItem } from './RelatedVideoItem';

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
  convertFileSrc: (path: string, protocol: string) => `http://${protocol}.localhost/${path}`,
}));

function widthClasses(element: Element): string[] {
  return [...element.classList].filter((name) => /^w-/.test(name));
}

describe('RelatedVideoItem', () => {
  beforeEach(() => {
    vi.useFakeTimers({ toFake: ['Date'] });
    vi.setSystemTime(new Date('2026-10-17T12:00:00Z'));
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  describe('as a suggestion row', () => {
    it('should show the title, folder and created date beside the thumbnail', () => {
      renderWithProviders(
        <RelatedVideoItem
          video={makeVideo({ title: 'Friends dancing at a party', durationMs: 17_000 })}
          isNext
        />,
      );

      expect(screen.getByRole('heading', { name: 'Friends dancing at a party' })).toBeVisible();
      expect(screen.getByText('Holidays')).toBeInTheDocument();
      expect(
        screen.getByText(
          (_, element) => element?.tagName === 'P' && element.textContent === 'Up next 3 days ago',
        ),
      ).toBeInTheDocument();
      expect(screen.getByRole('link')).toHaveAttribute('href', '/watch?v=vid1&folder=fold1');
    });

    it('should offer the action menu next to the link, not inside it', () => {
      renderWithProviders(<RelatedVideoItem video={makeVideo()} />);

      const menu = screen.getByRole('button', { name: 'Action menu' });
      expect(screen.getByRole('link')).not.toContainElement(menu);
    });

    it('should give the thumbnail exactly one width', () => {
      const { container } = renderWithProviders(<RelatedVideoItem video={makeVideo()} />);

      const thumbnail = container.querySelector('.aspect-video');
      expect(thumbnail).not.toBeNull();
      expect(widthClasses(thumbnail!)).toEqual(['w-[62.5%]']);
    });
  });

  describe('as a playlist panel row', () => {
    // Regression: a default `w-full` on the thumbnail beat the row's width and squeezed the text
    // column to nothing.
    it('should give the thumbnail exactly one width', () => {
      const { container } = renderWithProviders(
        <RelatedVideoItem video={makeVideo()} context={{ list: 'WL' }} position={2} />,
      );

      const thumbnail = container.querySelector('.aspect-video');
      expect(thumbnail).not.toBeNull();
      expect(widthClasses(thumbnail!)).toEqual(['w-[100px]']);
      expect(screen.getByText('2')).toBeInTheDocument();
    });
  });
});
