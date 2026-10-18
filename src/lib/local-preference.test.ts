import { afterEach, describe, expect, it, vi } from 'vitest';

import { readPreference, writePreference } from './local-preference';

const SIDEBAR_STATES = ['expanded', 'collapsed'] as const;

describe('local preferences', () => {
  afterEach(() => {
    window.localStorage.clear();
    vi.restoreAllMocks();
  });

  it('should read back a stored allowed value', () => {
    writePreference('homereel.sidebar', 'collapsed');

    const value = readPreference('homereel.sidebar', SIDEBAR_STATES, 'expanded');

    expect(value).toBe('collapsed');
  });

  it('should fall back when the stored value is not allowed', () => {
    window.localStorage.setItem('homereel.sidebar', 'sideways');

    const value = readPreference('homereel.sidebar', SIDEBAR_STATES, 'expanded');

    expect(value).toBe('expanded');
  });

  it('should fall back instead of throwing when storage is unavailable', () => {
    vi.spyOn(Storage.prototype, 'getItem').mockImplementation(() => {
      throw new Error('blocked');
    });
    vi.spyOn(Storage.prototype, 'setItem').mockImplementation(() => {
      throw new Error('blocked');
    });

    expect(() => writePreference('homereel.sidebar', 'collapsed')).not.toThrow();
    expect(readPreference('homereel.sidebar', SIDEBAR_STATES, 'expanded')).toBe('expanded');
  });
});
