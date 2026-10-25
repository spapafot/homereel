import { useCallback, useState } from 'react';

import { readPreference, writePreference } from '@/lib/local-preference';

const STORAGE_KEY = 'homereel.playerSize';
const PLAYER_SIZES = ['default', 'theater'] as const;

/** Whether the watch page shows the player in theater mode; remembered, as YouTube does. */
export function useTheaterMode(): [boolean, () => void] {
  const [isTheater, setIsTheater] = useState(
    () => readPreference(STORAGE_KEY, PLAYER_SIZES, 'default') === 'theater',
  );
  const toggleTheater = useCallback(() => {
    writePreference(STORAGE_KEY, isTheater ? 'default' : 'theater');
    setIsTheater(!isTheater);
  }, [isTheater]);
  return [isTheater, toggleTheater];
}
