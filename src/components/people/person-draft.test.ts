import { describe, expect, it } from 'vitest';

import type { Person } from '@/lib/api';

import { draftFromPerson, draftProblem, newPersonDraft, toPersonInput } from './person-draft';

const MARIA: Person = {
  id: 7,
  name: 'Maria',
  kind: 'person',
  description: 'my daughter',
  photoIds: [11, 12],
};

describe('person drafts', () => {
  it('should need a name and at least one photo', () => {
    expect(draftProblem(newPersonDraft())).toBe('Add a name.');
    expect(draftProblem({ ...newPersonDraft(), name: 'Rex' })).toBe(
      'Add a photo so Gemini can recognize them.',
    );
    expect(draftProblem({ ...newPersonDraft(['data:image/jpeg;base64,AA']), name: 'Rex' })).toBe(
      null,
    );
  });

  it('should ask to remove a photo when a new one goes over the limit', () => {
    const draft = draftFromPerson({ ...MARIA, photoIds: [1, 2, 3] }, ['data:image/jpeg;base64,AA']);

    expect(draftProblem(draft)).toBe('Remove a photo. Each person can have up to 3.');
  });

  it('should send trimmed fields, kept photo IDs and bare base64 for new photos', () => {
    const draft = {
      ...draftFromPerson(MARIA, ['data:image/jpeg;base64,/9j/AA==']),
      name: '  Maria ',
      description: '   ',
    };

    expect(toPersonInput(draft)).toEqual({
      id: 7,
      name: 'Maria',
      kind: 'person',
      description: null,
      keepPhotoIds: [11, 12],
      newPhotos: ['/9j/AA=='],
    });
  });
});
