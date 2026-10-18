import { useEffect } from 'react';
import { NavLink } from 'react-router';

import { Icon } from '@/components/common/Icon';
import { IconButton } from '@/components/common/IconButton';
import { Logo } from '@/components/common/Logo';
import { ShortsIcon } from '@/components/shorts/ShortsIcon';
import { cx } from '@/lib/class-names';
import { reshuffleHome } from '@/lib/home-mix';

type GuideEntry = {
  to: string;
  label: string;
  icon: string;
  end?: boolean;
  /** Runs on click, before navigating. */
  onSelect?: () => void;
};

/** Not a Material Symbols name: `GuideIcon` draws it. */
const SHORTS_ICON = 'shorts';

const HOME: GuideEntry = {
  to: '/',
  label: 'Home',
  icon: 'home',
  end: true,
  onSelect: reshuffleHome,
};
const SHORTS: GuideEntry = { to: '/shorts', label: 'Shorts', icon: SHORTS_ICON };
const ALL_VIDEOS: GuideEntry = { to: '/videos', label: 'All videos', icon: 'video_library' };
const FOLDERS: GuideEntry = { to: '/folders', label: 'Folders', icon: 'folder' };
const PLAYLISTS: GuideEntry = { to: '/playlists', label: 'Playlists', icon: 'playlist_play' };
const FAVORITES: GuideEntry = { to: '/favorites', label: 'Favorites', icon: 'favorite' };
const WATCH_LATER: GuideEntry = { to: '/watch-later', label: 'Watch later', icon: 'schedule' };
const HISTORY: GuideEntry = { to: '/history', label: 'History', icon: 'history' };
const SETTINGS: GuideEntry = { to: '/settings', label: 'Settings', icon: 'settings' };

const MAIN_SECTION = [HOME, SHORTS, ALL_VIDEOS];
const YOU_SECTION = [FOLDERS, PLAYLISTS, FAVORITES, WATCH_LATER, HISTORY];
const MINI_GUIDE = [HOME, SHORTS, ALL_VIDEOS, FOLDERS, PLAYLISTS, HISTORY];

function GuideIcon({ name, filled }: { name: string; filled: boolean }) {
  return name === SHORTS_ICON ? (
    <ShortsIcon filled={filled} />
  ) : (
    <Icon name={name} filled={filled} />
  );
}

function GuideItem({ entry, onNavigate }: { entry: GuideEntry; onNavigate?: () => void }) {
  return (
    <NavLink
      to={entry.to}
      end={entry.end}
      onClick={() => {
        entry.onSelect?.();
        onNavigate?.();
      }}
      className={({ isActive }) =>
        cx(
          'flex h-10 items-center gap-6 rounded-[10px] px-3 text-sm outline-none',
          'focus-visible:ring-2 focus-visible:ring-focus',
          isActive ? 'bg-chip font-medium hover:bg-chip-hover' : 'hover:bg-hover',
        )
      }
    >
      {({ isActive }) => (
        <>
          <GuideIcon name={entry.icon} filled={isActive} />
          <span className="truncate">{entry.label}</span>
        </>
      )}
    </NavLink>
  );
}

/** The expanded 240px guide. As on YouTube, each section's divider spans the whole guide. */
export function FullGuide({ onNavigate }: { onNavigate?: () => void }) {
  return (
    <nav aria-label="Main" className="flex flex-col pb-6">
      <div className="border-b border-divider p-3">
        {MAIN_SECTION.map((entry) => (
          <GuideItem key={entry.to} entry={entry} onNavigate={onNavigate} />
        ))}
      </div>
      <div className="border-b border-divider p-3">
        <h2 className="flex items-center gap-2 px-3 pt-1.5 pb-1 text-base leading-[22px] font-medium">
          You <Icon name="chevron_right" size={20} />
        </h2>
        {YOU_SECTION.map((entry) => (
          <GuideItem key={entry.to} entry={entry} onNavigate={onNavigate} />
        ))}
      </div>
      <div className="p-3">
        <GuideItem entry={SETTINGS} onNavigate={onNavigate} />
      </div>
    </nav>
  );
}

/** The collapsed 72px icon rail. */
export function MiniGuide() {
  return (
    <nav aria-label="Main" className="flex flex-col items-center gap-0.5 px-1 pt-1">
      {MINI_GUIDE.map((entry) => (
        <NavLink
          key={entry.to}
          to={entry.to}
          end={entry.end}
          onClick={entry.onSelect}
          className="flex w-16 flex-col items-center gap-1.5 rounded-lg pt-4 pb-3.5 text-[10px] outline-none hover:bg-hover focus-visible:ring-2 focus-visible:ring-focus"
        >
          {({ isActive }) => (
            <>
              <GuideIcon name={entry.icon} filled={isActive} />
              <span className="max-w-full truncate">{entry.label}</span>
            </>
          )}
        </NavLink>
      ))}
    </nav>
  );
}

/** The overlay guide used on narrow windows and on the watch page. */
export function GuideDrawer({ open, onClose }: { open: boolean; onClose: () => void }) {
  useEffect(() => {
    if (!open) return;
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === 'Escape') onClose();
    };
    window.addEventListener('keydown', onKeyDown);
    return () => window.removeEventListener('keydown', onKeyDown);
  }, [open, onClose]);

  if (!open) return null;
  return (
    <div className="fixed inset-0 z-50">
      <div className="absolute inset-0 bg-black/50" onClick={onClose} aria-hidden="true" />
      <aside className="absolute inset-y-0 left-0 w-60 overflow-y-auto bg-page">
        <div className="flex h-14 items-center pl-4">
          <IconButton icon="menu" label="Close guide" onClick={onClose} />
          <Logo />
        </div>
        <FullGuide onNavigate={onClose} />
      </aside>
    </div>
  );
}
