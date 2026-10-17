import { QueryCache, QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { StrictMode } from 'react';
import { createRoot } from 'react-dom/client';
import { RouterProvider } from 'react-router';

import { AUTH_QUERY_KEY } from './hooks/use-auth';
import { toAppError } from './lib/app-error';
import { applyTheme, watchSystemTheme } from './lib/theme';
import { router } from './router';
import './index.css';

applyTheme();
watchSystemTheme();

const queryClient: QueryClient = new QueryClient({
  queryCache: new QueryCache({
    // A revoked Google grant shows up on whichever request hits it first; re-checking the
    // auth status swaps every page to the reconnect prompt.
    onError: (error) => {
      const { code } = toAppError(error);
      if (code === 'auth_revoked' || code === 'not_connected') {
        void queryClient.invalidateQueries({ queryKey: AUTH_QUERY_KEY });
      }
    },
  }),
  defaultOptions: {
    queries: {
      // Drive errors are mapped and retried in Rust where that makes sense.
      retry: false,
      refetchOnWindowFocus: false,
      staleTime: 60_000,
    },
  },
});

createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <QueryClientProvider client={queryClient}>
      <RouterProvider router={router} />
    </QueryClientProvider>
  </StrictMode>,
);
