import { useEffect, useRef, useState, type FormEvent } from 'react';

import { Button } from '@/components/common/Button';
import { Icon } from '@/components/common/Icon';
import { useSavePerson } from '@/hooks/use-people';
import { MAX_PERSON_PHOTOS, personPhotoUrl, type PersonKind } from '@/lib/api';
import { toAppError } from '@/lib/app-error';

import { PhotoCropper } from './PhotoCropper';
import { draftPhotoCount, draftProblem, toPersonInput, type PersonDraft } from './person-draft';

const MAX_NAME_LENGTH = 60;
const MAX_DESCRIPTION_LENGTH = 120;
const KIND_LABELS: Record<PersonKind, string> = { person: 'Person', animal: 'Animal' };
const FIELD_CLASS =
  'mt-1 w-full border-b border-divider bg-transparent pb-1 text-base outline-none focus:border-b-2 focus:border-text';

function PhotoTile({ src, label, onRemove }: { src: string; label: string; onRemove: () => void }) {
  return (
    <div className="relative size-20 shrink-0">
      <img src={src} alt={label} className="size-full rounded-lg bg-skeleton object-cover" />
      <button
        type="button"
        aria-label={`Remove ${label.toLowerCase()}`}
        title="Remove photo"
        onClick={onRemove}
        className="absolute -top-2 -right-2 flex size-6 items-center justify-center rounded-full bg-text text-page outline-none hover:opacity-85 focus-visible:ring-2 focus-visible:ring-focus"
      >
        <Icon name="close" size={16} />
      </button>
    </div>
  );
}

type PersonFormProps = {
  initial: PersonDraft;
  onDone: () => void;
  onCancel: () => void;
};

/** Name, kind, description and up to three cropped photos of one person or pet. */
export function PersonForm({ initial, onDone, onCancel }: PersonFormProps) {
  const savePerson = useSavePerson();
  const fileInputRef = useRef<HTMLInputElement>(null);
  const [draft, setDraft] = useState(initial);
  const [cropSource, setCropSource] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(
    () => () => {
      if (cropSource) URL.revokeObjectURL(cropSource);
    },
    [cropSource],
  );

  if (cropSource) {
    return (
      <PhotoCropper
        src={cropSource}
        cancelLabel="Back"
        onCancel={() => setCropSource(null)}
        onCrop={(photo) => {
          setDraft((current) => ({ ...current, newPhotos: [...current.newPhotos, photo] }));
          setCropSource(null);
        }}
      />
    );
  }

  const photoCount = draftPhotoCount(draft);
  const problem = draftProblem(draft);
  // A missing name or photo shows as a disabled Save; too many photos needs explaining.
  const message = error ?? (photoCount > MAX_PERSON_PHOTOS ? problem : null);
  const update = (changes: Partial<PersonDraft>) =>
    setDraft((current) => ({ ...current, ...changes }));

  const submit = async (event: FormEvent) => {
    event.preventDefault();
    if (problem) return;
    setError(null);
    try {
      await savePerson.mutateAsync(toPersonInput(draft));
      onDone();
    } catch (caught) {
      setError(toAppError(caught).message);
    }
  };

  return (
    <form onSubmit={submit} className="mt-4 flex flex-col gap-5">
      <div>
        <span className="text-xs text-text-secondary">Photos</span>
        <div className="mt-2 flex flex-wrap gap-3">
          {draft.keepPhotoIds.map((photoId, index) => (
            <PhotoTile
              key={photoId}
              src={personPhotoUrl(photoId)}
              label={`Photo ${index + 1}`}
              onRemove={() =>
                update({ keepPhotoIds: draft.keepPhotoIds.filter((id) => id !== photoId) })
              }
            />
          ))}
          {draft.newPhotos.map((photo, index) => (
            <PhotoTile
              // New photos have no ID yet; their position is stable until one is removed.
              key={`new-${index}`}
              src={photo}
              label={`Photo ${draft.keepPhotoIds.length + index + 1}`}
              onRemove={() =>
                update({ newPhotos: draft.newPhotos.filter((_, position) => position !== index) })
              }
            />
          ))}
          {photoCount < MAX_PERSON_PHOTOS && (
            <button
              type="button"
              onClick={() => fileInputRef.current?.click()}
              className="flex size-20 shrink-0 flex-col items-center justify-center gap-1 rounded-lg border border-dashed border-divider text-xs text-text-secondary outline-none hover:bg-hover focus-visible:ring-2 focus-visible:ring-focus"
            >
              <Icon name="add_a_photo" />
              Add photo
            </button>
          )}
        </div>
        <input
          ref={fileInputRef}
          type="file"
          accept="image/jpeg,image/png,image/webp"
          aria-label="Choose a photo"
          className="hidden"
          onChange={(event) => {
            const file = event.target.files?.[0];
            // Clearing the input lets the same file be chosen again after Back.
            event.target.value = '';
            if (file) setCropSource(URL.createObjectURL(file));
          }}
        />
        <p className="mt-2 text-xs text-text-secondary">
          Up to {MAX_PERSON_PHOTOS} clear, close photos, for example at different ages.
        </p>
      </div>

      <label className="block">
        <span className="text-xs text-text-secondary">Name</span>
        <input
          autoFocus
          value={draft.name}
          maxLength={MAX_NAME_LENGTH}
          onChange={(event) => update({ name: event.target.value })}
          className={FIELD_CLASS}
        />
      </label>

      <div role="radiogroup" aria-label="Person or animal" className="flex gap-6">
        {(Object.keys(KIND_LABELS) as PersonKind[]).map((kind) => (
          <label key={kind} className="flex h-8 cursor-pointer items-center gap-3 text-sm">
            <input
              type="radio"
              name="person-kind"
              value={kind}
              checked={draft.kind === kind}
              onChange={() => update({ kind })}
              className="size-4 accent-link"
            />
            {KIND_LABELS[kind]}
          </label>
        ))}
      </div>

      <label className="block">
        <span className="text-xs text-text-secondary">Description (optional)</span>
        <input
          value={draft.description}
          maxLength={MAX_DESCRIPTION_LENGTH}
          placeholder="For example: my daughter, or golden retriever"
          onChange={(event) => update({ description: event.target.value })}
          className={FIELD_CLASS}
        />
        <span className="mt-1 block text-xs text-text-secondary">
          Helps Gemini tell people and pets apart.
        </span>
      </label>

      {message && <p className="text-sm text-red-600 dark:text-red-400">{message}</p>}
      <div className="flex justify-end gap-2">
        <Button disabled={savePerson.isPending} onClick={onCancel}>
          Cancel
        </Button>
        <Button
          type="submit"
          variant="primary"
          disabled={problem != null || savePerson.isPending}
          title={problem ?? undefined}
        >
          {savePerson.isPending ? 'Saving…' : 'Save'}
        </Button>
      </div>
    </form>
  );
}
