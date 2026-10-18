import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';

import { api, type DriveFolder, type LibrarySources } from '@/lib/api';
import { toAppError } from '@/lib/app-error';
import { showToast } from '@/lib/toast';

export const SOURCES_QUERY_KEY = ['library-sources'] as const;

export function useLibrarySources() {
  return useQuery({
    queryKey: SOURCES_QUERY_KEY,
    queryFn: api.librarySources,
    staleTime: Infinity,
  });
}

/** Folders inside a Drive folder, or at the top of My Drive, for the folder picker. */
export function useDriveFolders(parentId: string | undefined, enabled: boolean) {
  return useQuery({
    queryKey: ['drive-folders', parentId ?? 'root'],
    queryFn: () => api.driveFoldersList(parentId),
    enabled,
  });
}

/** Saves new sources and reloads every list, which now holds other videos. */
function useSourcesChange<Input>(change: (input: Input) => Promise<LibrarySources>) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: change,
    onSuccess: (sources) => {
      const previous = queryClient.getQueryData<LibrarySources>(SOURCES_QUERY_KEY);
      queryClient.setQueryData(SOURCES_QUERY_KEY, sources);
      // A cancelled folder picker changes nothing.
      if (JSON.stringify(previous) === JSON.stringify(sources)) return;
      void queryClient.invalidateQueries({
        predicate: (query) =>
          query.queryKey[0] !== 'auth' && query.queryKey[0] !== SOURCES_QUERY_KEY[0],
      });
    },
    onError: (error) => showToast(toAppError(error).message),
  });
}

/** Opens the system folder picker and adds the chosen folder to the library. */
export function useAddLocalFolder() {
  return useSourcesChange<void>(() => api.libraryLocalFolderAdd());
}

export function useRemoveLocalFolder() {
  return useSourcesChange((path: string) => api.libraryLocalFolderRemove(path));
}

/** Limits the library's Drive videos to one folder and its subfolders; `null` takes all of Drive. */
export function useSetDriveFolder() {
  return useSourcesChange((folder: DriveFolder | null) => api.libraryDriveFolderSet(folder));
}
