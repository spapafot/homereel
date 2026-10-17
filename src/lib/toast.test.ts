import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import { dismissToast, getToasts, showToast } from './toast';

describe('showToast', () => {
  beforeEach(() => {
    vi.useFakeTimers();
  });

  afterEach(() => {
    getToasts().forEach((toast) => dismissToast(toast.id));
    vi.useRealTimers();
  });

  it('should hide a message after a few seconds', () => {
    showToast('Saved to Watch later');

    vi.advanceTimersByTime(4_000);

    expect(getToasts()).toEqual([]);
  });

  it('should keep a persistent message until the result replaces it', () => {
    showToast('Generating a new title…', { persistent: true });

    vi.advanceTimersByTime(60_000);
    const pending = getToasts().map((toast) => toast.message);
    showToast('New title: Beach day');

    expect(pending).toEqual(['Generating a new title…']);
    expect(getToasts().map((toast) => toast.message)).toEqual(['New title: Beach day']);
  });
});
