import { useCallback, useState } from 'react';
import { Outlet, ScrollRestoration, useLocation } from 'react-router';

import { Toaster } from '@/components/common/Toaster';
import { useHistoryShortcuts } from '@/hooks/use-history-shortcuts';
import { useMediaQuery } from '@/hooks/use-media-query';
import { useSidebarCollapsed } from '@/hooks/use-sidebar-collapsed';
import { useTitleEvents } from '@/hooks/use-titles';
import { useRecordedDateEvents } from '@/hooks/use-videos';
import { cx } from '@/lib/class-names';

import { FullGuide, GuideDrawer, MiniGuide } from './Sidebar';
import { TopBar } from './TopBar';

/** YouTube's breakpoints for the persistent guide and the icon rail. */
const WIDE_QUERY = '(min-width: 1312px)';
const MEDIUM_QUERY = '(min-width: 792px)';

type GuideMode = 'full' | 'mini' | 'none';

export function AppShell() {
  const location = useLocation();
  const isWide = useMediaQuery(WIDE_QUERY);
  const isMedium = useMediaQuery(MEDIUM_QUERY);
  const [collapsed, setCollapsed] = useSidebarCollapsed();
  const [drawerOpen, setDrawerOpen] = useState(false);
  const [drawerPath, setDrawerPath] = useState(location.pathname);
  useTitleEvents();
  useRecordedDateEvents();
  useHistoryShortcuts();

  // Close the drawer whenever the user navigates somewhere else.
  if (drawerPath !== location.pathname) {
    setDrawerPath(location.pathname);
    setDrawerOpen(false);
  }

  // Like YouTube, the watch page gives the video the full width and only offers the drawer.
  const isWatchPage = location.pathname === '/watch';
  const usesDrawer = isWatchPage || !isWide;
  let guide: GuideMode = 'none';
  if (!isWatchPage) {
    if (isWide) guide = collapsed ? 'mini' : 'full';
    else if (isMedium) guide = 'mini';
  }

  const toggleGuide = () => {
    if (usesDrawer) setDrawerOpen((open) => !open);
    else setCollapsed(!collapsed);
  };
  const closeDrawer = useCallback(() => setDrawerOpen(false), []);

  return (
    <>
      <TopBar onToggleGuide={toggleGuide} />
      {guide === 'full' && (
        <aside className="scrollbar-none fixed top-14 bottom-0 left-0 z-20 w-60 overflow-y-auto bg-page hover:[scrollbar-width:thin]">
          <FullGuide />
        </aside>
      )}
      {guide === 'mini' && (
        <aside className="fixed top-14 bottom-0 left-0 z-20 w-[72px] bg-page">
          <MiniGuide />
        </aside>
      )}
      <GuideDrawer open={drawerOpen && usesDrawer} onClose={closeDrawer} />
      <main className={cx('pt-14', guide === 'full' && 'pl-60', guide === 'mini' && 'pl-[72px]')}>
        <Outlet />
      </main>
      <Toaster />
      {/* New pages open at the top; Back returns to where the user was in the grid. */}
      <ScrollRestoration />
    </>
  );
}
