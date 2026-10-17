import { invoke } from '@tauri-apps/api/core';
import { screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import type { Person } from '@/lib/api';
import { renderWithProviders } from '@/test/render';

import { PeopleSettings } from './PeopleSettings';

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
  convertFileSrc: (path: string, protocol: string) => `http://${protocol}.localhost/${path}`,
}));

const PEOPLE: Person[] = [
  { id: 1, name: 'Maria', kind: 'person', description: 'my daughter', photoIds: [10, 11] },
  { id: 2, name: 'Rex', kind: 'animal', description: null, photoIds: [12] },
];

describe('PeopleSettings', () => {
  beforeEach(() => {
    vi.mocked(invoke).mockReset();
    vi.mocked(invoke).mockImplementation(async (command) => {
      if (command === 'people_list') return PEOPLE;
      return undefined;
    });
  });

  it('should list each person with their kind, description and photo count', async () => {
    renderWithProviders(<PeopleSettings />);

    expect(await screen.findByText('Maria')).toBeInTheDocument();
    expect(screen.getByText('Person • my daughter • 2 photos')).toBeInTheDocument();
    expect(screen.getByText('Animal • 1 photo')).toBeInTheDocument();
  });

  it('should show each person’s first photo from the photo protocol', async () => {
    const { container } = renderWithProviders(<PeopleSettings />);

    await screen.findByText('Maria');
    const sources = Array.from(container.querySelectorAll('img')).map((image) => image.src);
    expect(sources).toEqual(['http://personphoto.localhost/10', 'http://personphoto.localhost/12']);
  });

  it('should not allow saving a new person until they have a photo', async () => {
    const user = userEvent.setup();
    renderWithProviders(<PeopleSettings />);

    await user.click(await screen.findByRole('button', { name: 'Add person or pet' }));
    await user.type(screen.getByRole('textbox', { name: 'Name' }), 'Nikos');

    expect(screen.getByRole('dialog', { name: 'Add a person or pet' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Save' })).toBeDisabled();
  });

  it('should save a renamed person with their existing photos', async () => {
    const user = userEvent.setup();
    renderWithProviders(<PeopleSettings />);

    await user.click(await screen.findByRole('button', { name: 'Edit Rex' }));
    const name = screen.getByRole('textbox', { name: 'Name' });
    await user.clear(name);
    await user.type(name, 'Rexy');
    await user.click(screen.getByRole('button', { name: 'Save' }));

    await waitFor(() =>
      expect(invoke).toHaveBeenCalledWith('person_save', {
        input: {
          id: 2,
          name: 'Rexy',
          kind: 'animal',
          description: null,
          keepPhotoIds: [12],
          newPhotos: [],
        },
      }),
    );
  });

  it('should remove a person after confirming', async () => {
    const user = userEvent.setup();
    renderWithProviders(<PeopleSettings />);

    await user.click(await screen.findByRole('button', { name: 'Remove Rex' }));
    await user.click(screen.getByRole('button', { name: 'Remove' }));

    await waitFor(() => expect(invoke).toHaveBeenCalledWith('person_delete', { personId: 2 }));
  });
});
