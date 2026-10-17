import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { MemoryRouter, useLocation } from 'react-router';
import { describe, expect, it } from 'vitest';

import { SearchBar } from './SearchBar';

function LocationProbe() {
  const location = useLocation();
  return <output data-testid="location">{location.pathname + location.search}</output>;
}

function renderSearchBar(initialPath = '/') {
  return render(
    <MemoryRouter initialEntries={[initialPath]}>
      <SearchBar />
      <LocationProbe />
    </MemoryRouter>,
  );
}

describe('SearchBar', () => {
  it('should open the results page for the typed query on Enter', async () => {
    const user = userEvent.setup();
    renderSearchBar();

    await user.type(screen.getByRole('textbox', { name: 'Search' }), '  beach day {Enter}');

    expect(screen.getByTestId('location')).toHaveTextContent('/results?search_query=beach+day');
  });

  it('should not navigate for a blank query', async () => {
    const user = userEvent.setup();
    renderSearchBar();

    await user.type(screen.getByRole('textbox', { name: 'Search' }), '   {Enter}');

    expect(screen.getByTestId('location')).toHaveTextContent(/^\/$/);
  });

  it('should show the current query on the results page', () => {
    renderSearchBar('/results?search_query=cats');

    expect(screen.getByRole('textbox', { name: 'Search' })).toHaveValue('cats');
  });

  it('should clear the query with the clear button', async () => {
    const user = userEvent.setup();
    renderSearchBar('/results?search_query=cats');

    await user.click(screen.getByRole('button', { name: 'Clear search query' }));

    expect(screen.getByRole('textbox', { name: 'Search' })).toHaveValue('');
  });
});
