import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';

import { api } from '@/lib/api';

export const AUTH_QUERY_KEY = ['auth'] as const;

export function useAuthStatus() {
  return useQuery({ queryKey: AUTH_QUERY_KEY, queryFn: api.authStatus, staleTime: Infinity });
}

/** Runs the browser sign-in and reloads every Drive query for the new account. */
export function useConnectDrive() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: api.authConnect,
    onSuccess: (status) => {
      queryClient.setQueryData(AUTH_QUERY_KEY, status);
      queryClient.removeQueries({ predicate: (query) => query.queryKey[0] !== 'auth' });
    },
  });
}

export function useDisconnectDrive() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: api.authDisconnect,
    onSuccess: (status) => {
      queryClient.setQueryData(AUTH_QUERY_KEY, status);
      queryClient.removeQueries({ predicate: (query) => query.queryKey[0] !== 'auth' });
    },
  });
}
