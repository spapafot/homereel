import type { VideoOrder } from '@/lib/api';

import type { Chip } from './ChipBar';

export const DEFAULT_VIDEO_ORDER: VideoOrder = 'createdNewest';

/** "Newest" and "Oldest" go by the day a video was recorded, the date cards show. */
export const ORDER_CHIPS: Array<Chip & { id: VideoOrder }> = [
  { id: 'createdNewest', label: 'Newest' },
  { id: 'createdOldest', label: 'Oldest' },
  { id: 'uploadedNewest', label: 'Newest uploads' },
  { id: 'uploadedOldest', label: 'Oldest uploads' },
  { id: 'name', label: 'A–Z' },
];

export function isVideoOrder(value: string): value is VideoOrder {
  return ORDER_CHIPS.some((chip) => chip.id === value);
}
