import { act, renderHook } from '@testing-library/react';
import { afterEach, describe, expect, it } from 'vitest';

import { useTheaterMode } from './use-theater-mode';

describe('useTheaterMode', () => {
  afterEach(() => {
    window.localStorage.clear();
  });

  it('should start in the default view', () => {
    const { result } = renderHook(() => useTheaterMode());

    expect(result.current[0]).toBe(false);
  });

  it('should remember theater mode for the next video', () => {
    const first = renderHook(() => useTheaterMode());
    act(() => first.result.current[1]());
    first.unmount();

    const { result } = renderHook(() => useTheaterMode());

    expect(result.current[0]).toBe(true);
  });

  it('should go back to the default view on a second toggle', () => {
    const { result } = renderHook(() => useTheaterMode());

    act(() => result.current[1]());
    act(() => result.current[1]());

    expect(result.current[0]).toBe(false);
  });
});
