import { useAddLocalFolder } from '@/hooks/use-library-sources';

import { Button } from './Button';

/** Adds a folder on this computer to the library through the system folder picker. */
export function AddFolderButton({ variant = 'tonal' }: { variant?: 'tonal' | 'outline' }) {
  const addFolder = useAddLocalFolder();

  return (
    <Button
      variant={variant}
      icon="create_new_folder"
      onClick={() => addFolder.mutate()}
      disabled={addFolder.isPending}
    >
      Add a folder
    </Button>
  );
}
