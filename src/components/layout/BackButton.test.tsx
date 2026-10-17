import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { Link, MemoryRouter, useLocation } from 'react-router';
import { describe, expect, it } from 'vitest';

import { BackButton } from './BackButton';

function LocationProbe() {
  const location = useLocation();
  return <output data-testid="location">{location.pathname + location.search}</output>;
}

function renderBackButton() {
  return render(
    <MemoryRouter initialEntries={['/']}>
      <BackButton />
      <Link to="/watch?v=abc">Open video</Link>
      <LocationProbe />
    </MemoryRouter>,
  );
}

describe('BackButton', () => {
  it('should be disabled on the page the app opened on', () => {
    renderBackButton();

    expect(screen.getByRole('button', { name: 'Back' })).toBeDisabled();
  });

  it('should return to the previous page after opening a video', async () => {
    const user = userEvent.setup();
    renderBackButton();
    await user.click(screen.getByRole('link', { name: 'Open video' }));
    expect(screen.getByTestId('location')).toHaveTextContent('/watch?v=abc');

    await user.click(screen.getByRole('button', { name: 'Back' }));

    expect(screen.getByTestId('location')).toHaveTextContent(/^\/$/);
    expect(screen.getByRole('button', { name: 'Back' })).toBeDisabled();
  });
});
