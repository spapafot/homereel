import { useQueryClient } from '@tanstack/react-query';
import { useNavigate } from 'react-router';

import { IconButton } from '@/components/common/IconButton';
import { Logo } from '@/components/common/Logo';
import { SearchBar } from '@/components/search/SearchBar';
import { reshuffleHome } from '@/lib/home-mix';

import { AccountMenu } from './AccountMenu';
import { BackButton } from './BackButton';

export function TopBar({ onToggleGuide }: { onToggleGuide: () => void }) {
  const navigate = useNavigate();
  const queryClient = useQueryClient();

  const refreshLibrary = () => {
    reshuffleHome();
    return queryClient.invalidateQueries({ predicate: (query) => query.queryKey[0] !== 'auth' });
  };

  return (
    <header className="fixed inset-x-0 top-0 z-40 flex h-14 items-center justify-between gap-4 bg-page px-4">
      <div className="flex shrink-0 items-center">
        <BackButton />
        <IconButton icon="menu" label="Guide" onClick={onToggleGuide} />
        <Logo />
      </div>
      <div className="flex min-w-0 flex-1 justify-center">
        <SearchBar />
      </div>
      <div className="flex shrink-0 items-center gap-1">
        <IconButton icon="refresh" label="Refresh library" onClick={refreshLibrary} />
        <IconButton icon="settings" label="Settings" onClick={() => navigate('/settings')} />
        <AccountMenu />
      </div>
    </header>
  );
}
