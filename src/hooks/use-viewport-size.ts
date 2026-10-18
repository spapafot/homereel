import { useSyncExternalStore } from 'react';

function subscribe(onChange: () => void) {
  window.addEventListener('resize', onChange);
  // The root's width also changes when the page scrollbar appears, without a resize event.
  const observer = typeof ResizeObserver === 'undefined' ? null : new ResizeObserver(onChange);
  observer?.observe(document.documentElement);
  return () => {
    window.removeEventListener('resize', onChange);
    observer?.disconnect();
  };
}

/** The window's width without its scrollbar, which is what YouTube's layout rules measure. */
export function useViewportWidth(): number {
  return useSyncExternalStore(subscribe, () => document.documentElement.clientWidth);
}

export function useViewportHeight(): number {
  return useSyncExternalStore(subscribe, () => window.innerHeight);
}
