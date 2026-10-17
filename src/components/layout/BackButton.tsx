import { useLocation, useNavigate } from 'react-router';

import { IconButton } from '@/components/common/IconButton';

/** The desktop window has no browser chrome, so the app supplies its own Back control. */
export function BackButton() {
  const navigate = useNavigate();
  const location = useLocation();
  // React Router gives only the entry the app opened on the key 'default'; every later entry
  // has a page behind it, since the app never replaces history entries.
  const canGoBack = location.key !== 'default';

  return (
    <IconButton icon="arrow_back" label="Back" disabled={!canGoBack} onClick={() => navigate(-1)} />
  );
}
