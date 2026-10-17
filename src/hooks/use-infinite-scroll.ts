import { useEffect, useRef } from 'react';

/** Start loading the next page well before the user reaches the end of the list. */
const PRELOAD_MARGIN = '800px 0px';

/** Returns a ref for a sentinel element; `onReachEnd` runs when it scrolls into range. */
export function useInfiniteScroll(onReachEnd: () => void, enabled: boolean) {
  const sentinelRef = useRef<HTMLDivElement>(null);
  const callbackRef = useRef(onReachEnd);

  useEffect(() => {
    callbackRef.current = onReachEnd;
  }, [onReachEnd]);

  useEffect(() => {
    const sentinel = sentinelRef.current;
    if (!sentinel || !enabled || typeof IntersectionObserver === 'undefined') return;
    const observer = new IntersectionObserver(
      (entries) => {
        if (entries.some((entry) => entry.isIntersecting)) callbackRef.current();
      },
      { rootMargin: PRELOAD_MARGIN },
    );
    observer.observe(sentinel);
    return () => observer.disconnect();
  }, [enabled]);

  return sentinelRef;
}
