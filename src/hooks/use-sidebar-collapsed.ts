import { useCallback, useState } from 'react';

import { readPreference, writePreference } from '@/lib/local-preference';

const STORAGE_KEY = 'drivetube.sidebar';
const SIDEBAR_STATES = ['expanded', 'collapsed'] as const;

/** Whether the wide-screen sidebar is collapsed to the icon rail; remembered across launches. */
export function useSidebarCollapsed(): [boolean, (collapsed: boolean) => void] {
  const [collapsed, setCollapsedState] = useState(
    () => readPreference(STORAGE_KEY, SIDEBAR_STATES, 'expanded') === 'collapsed',
  );
  const setCollapsed = useCallback((value: boolean) => {
    setCollapsedState(value);
    writePreference(STORAGE_KEY, value ? 'collapsed' : 'expanded');
  }, []);
  return [collapsed, setCollapsed];
}
