import { useState } from 'react';

import { Button } from '@/components/common/Button';
import { IconButton } from '@/components/common/IconButton';
import { ConfirmDialog } from '@/components/common/Modal';
import { useDeletePerson, usePeople } from '@/hooks/use-people';
import type { Person } from '@/lib/api';
import { toAppError } from '@/lib/app-error';

import { PersonAvatar } from './PersonAvatar';
import { PersonDialog } from './PersonDialog';
import { draftFromPerson, newPersonDraft, type PersonDraft } from './person-draft';

function personDetails(person: Person): string {
  const photos = `${person.photoIds.length} ${person.photoIds.length === 1 ? 'photo' : 'photos'}`;
  return [person.kind === 'animal' ? 'Animal' : 'Person', person.description, photos]
    .filter(Boolean)
    .join(' • ');
}

/** The people and pets Gemini looks for when it titles videos. */
export function PeopleSettings() {
  const people = usePeople();
  const deletePerson = useDeletePerson();
  const [draft, setDraft] = useState<PersonDraft | null>(null);
  const [pendingDelete, setPendingDelete] = useState<Person | null>(null);
  const hasPeople = (people.data?.length ?? 0) > 0;

  return (
    <div className="flex flex-col gap-4">
      <p className="text-sm leading-5 text-text-secondary">
        Gemini compares these people and pets with every video it titles and uses their names when
        it recognizes them. Their photos are sent with each title request, adding about $0.0004 per
        photo to every video. While watching, you can pause on a clear shot and choose{' '}
        <strong className="font-medium text-text">Tag a person or pet</strong> from the ⋮ menu.
      </p>

      {people.isError && (
        <p className="text-sm text-red-600 dark:text-red-400">{toAppError(people.error).message}</p>
      )}
      {hasPeople && (
        <ul className="flex flex-col">
          {people.data?.map((person) => (
            <li key={person.id} className="flex items-center gap-4 py-2">
              <PersonAvatar person={person} size={48} />
              <div className="min-w-0 flex-1">
                <p className="truncate text-base">{person.name}</p>
                <p className="truncate text-sm text-text-secondary">{personDetails(person)}</p>
              </div>
              <IconButton
                icon="edit"
                label={`Edit ${person.name}`}
                onClick={() => setDraft(draftFromPerson(person))}
              />
              <IconButton
                icon="delete"
                label={`Remove ${person.name}`}
                onClick={() => setPendingDelete(person)}
              />
            </li>
          ))}
        </ul>
      )}

      <div className="flex flex-wrap items-center gap-3">
        <Button icon="person_add" onClick={() => setDraft(newPersonDraft())}>
          Add person or pet
        </Button>
      </div>
      {hasPeople && (
        <p className="text-xs text-text-secondary">
          Titles generated from now on can use these names. To update titles you already have,
          choose Regenerate all titles under AI titles.
        </p>
      )}

      <PersonDialog draft={draft} onClose={() => setDraft(null)} />
      <ConfirmDialog
        open={pendingDelete != null}
        onOpenChange={(open) => !open && setPendingDelete(null)}
        title={pendingDelete ? `Remove ${pendingDelete.name}?` : 'Remove'}
        description="Their photos are deleted from DriveTube. Titles that already use their name stay as they are."
        confirmLabel="Remove"
        onConfirm={() => pendingDelete && deletePerson.mutate(pendingDelete)}
      />
    </div>
  );
}
