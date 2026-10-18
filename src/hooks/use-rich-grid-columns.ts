import { useCallback, useState } from 'react';

import { richGridColumns } from '@/lib/rich-grid';

/**
 * YouTube's column count for the element the returned ref is attached to, kept up to date as it
 * resizes. Apply it as `repeat(columns, minmax(0, 1fr))`.
 */
export function useRichGridColumns() {
  const [columns, setColumns] = useState(1);

  const ref = useCallback((node: HTMLElement | null) => {
    if (!node) return;
    const update = () => setColumns(richGridColumns(node.clientWidth));
    update();
    if (typeof ResizeObserver === 'undefined') return;
    const observer = new ResizeObserver(update);
    observer.observe(node);
    return () => observer.disconnect();
  }, []);

  return { ref, columns };
}
