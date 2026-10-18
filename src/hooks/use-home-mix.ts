import { useSyncExternalStore } from 'react';

import type { HomeMix } from '@/lib/api';
import { getHomeMix, subscribeHomeMix } from '@/lib/home-mix';

export function useHomeMix(): HomeMix {
  return useSyncExternalStore(subscribeHomeMix, getHomeMix);
}
