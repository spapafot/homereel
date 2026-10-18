import { createBrowserRouter } from 'react-router';

import { AppShell } from './components/layout/AppShell';
import { AllVideosPage } from './routes/AllVideosPage';
import { FavoritesPage } from './routes/FavoritesPage';
import { FolderPage } from './routes/FolderPage';
import { FoldersPage } from './routes/FoldersPage';
import { HistoryPage } from './routes/HistoryPage';
import { HomePage } from './routes/HomePage';
import { NotFoundPage } from './routes/NotFoundPage';
import { PlaylistPage } from './routes/PlaylistPage';
import { PlaylistsPage } from './routes/PlaylistsPage';
import { SearchPage } from './routes/SearchPage';
import { SettingsPage } from './routes/SettingsPage';
import { ShortsPage } from './routes/ShortsPage';
import { WatchLaterPage } from './routes/WatchLaterPage';
import { WatchPage } from './routes/WatchPage';

export const router = createBrowserRouter([
  {
    Component: AppShell,
    children: [
      { index: true, Component: HomePage },
      { path: 'videos', Component: AllVideosPage },
      { path: 'results', Component: SearchPage },
      { path: 'watch', Component: WatchPage },
      { path: 'shorts/:videoId?', Component: ShortsPage },
      { path: 'folders', Component: FoldersPage },
      { path: 'folder/:folderId', Component: FolderPage },
      { path: 'playlists', Component: PlaylistsPage },
      { path: 'playlist', Component: PlaylistPage },
      { path: 'favorites', Component: FavoritesPage },
      { path: 'watch-later', Component: WatchLaterPage },
      { path: 'history', Component: HistoryPage },
      { path: 'settings', Component: SettingsPage },
      { path: '*', Component: NotFoundPage },
    ],
  },
]);
