import { describe, expect, it, vi } from 'vitest';

import { getHomeMix, localDay, reshuffleHome, subscribeHomeMix } from './home-mix';

describe('localDay', () => {
  it('should format the local date with zero-padded month and day', () => {
    const day = localDay(new Date(2026, 0, 5, 23, 30));

    expect(day).toBe('2026-01-05');
  });
});

describe('reshuffleHome', () => {
  it('should make a new seed for today and notify subscribers', () => {
    const before = getHomeMix();
    const listener = vi.fn();
    const unsubscribe = subscribeHomeMix(listener);
    vi.spyOn(Math, 'random').mockReturnValue(before.seed === 0 ? 0.5 : 0);

    reshuffleHome();
    unsubscribe();
    vi.restoreAllMocks();

    expect(getHomeMix().seed).not.toBe(before.seed);
    expect(getHomeMix().today).toBe(localDay(new Date()));
    expect(listener).toHaveBeenCalledTimes(1);
  });
});
