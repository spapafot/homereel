import { MAX_PERSON_PHOTOS, type Person, type PersonInput, type PersonKind } from '@/lib/api';

/** The person dialog's working copy, saved in one call. */
export type PersonDraft = {
  id: number | null;
  name: string;
  kind: PersonKind;
  description: string;
  keepPhotoIds: number[];
  /** Cropped JPEG data URLs, not saved yet. */
  newPhotos: string[];
};

const JPEG_DATA_URL_PREFIX = /^data:image\/jpeg;base64,/;

export function newPersonDraft(newPhotos: string[] = []): PersonDraft {
  return { id: null, name: '', kind: 'person', description: '', keepPhotoIds: [], newPhotos };
}

export function draftFromPerson(person: Person, newPhotos: string[] = []): PersonDraft {
  return {
    id: person.id,
    name: person.name,
    kind: person.kind,
    description: person.description ?? '',
    keepPhotoIds: person.photoIds,
    newPhotos,
  };
}

export function draftPhotoCount(draft: PersonDraft): number {
  return draft.keepPhotoIds.length + draft.newPhotos.length;
}

/** Why the draft cannot be saved yet, or `null` when it can. */
export function draftProblem(draft: PersonDraft): string | null {
  const photos = draftPhotoCount(draft);
  if (photos > MAX_PERSON_PHOTOS) {
    return `Remove a photo. Each person can have up to ${MAX_PERSON_PHOTOS}.`;
  }
  if (!draft.name.trim()) return 'Add a name.';
  if (photos === 0) return 'Add a photo so Gemini can recognize them.';
  return null;
}

export function toPersonInput(draft: PersonDraft): PersonInput {
  return {
    id: draft.id,
    name: draft.name.trim(),
    kind: draft.kind,
    description: draft.description.trim() || null,
    keepPhotoIds: draft.keepPhotoIds,
    newPhotos: draft.newPhotos.map((photo) => photo.replace(JPEG_DATA_URL_PREFIX, '')),
  };
}
