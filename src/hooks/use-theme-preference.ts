import { useSyncExternalStore } from 'react';

import {
  getThemePreference,
  setThemePreference,
  subscribeThemePreference,
  type ThemePreference,
} from '@/lib/theme';

export function useThemePreference(): [ThemePreference, (preference: ThemePreference) => void] {
  const preference = useSyncExternalStore(subscribeThemePreference, getThemePreference);
  return [preference, setThemePreference];
}
