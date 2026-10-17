import { fireEvent, render, screen } from '@testing-library/react';
import { MemoryRouter, useLocation } from 'react-router';
import { describe, expect, it } from 'vitest';

import { useHistoryShortcuts } from './use-history-shortcuts';

function ShortcutsProbe() {
  useHistoryShortcuts();
  const location = useLocation();
  return <output data-testid="location">{location.pathname}</output>;
}

/** Starts on the middle of three pages so both directions have somewhere to go. */
function renderOnMiddlePage() {
  render(
    <MemoryRouter initialEntries={['/', '/watch', '/history']} initialIndex={1}>
      <ShortcutsProbe />
    </MemoryRouter>,
  );
}

describe('useHistoryShortcuts', () => {
  it('should go back when the mouse back button is released', () => {
    renderOnMiddlePage();

    const notCancelled = fireEvent.mouseUp(window, { button: 3 });

    expect(screen.getByTestId('location')).toHaveTextContent(/^\/$/);
    expect(notCancelled).toBe(false);
  });

  it('should go forward when the mouse forward button is released', () => {
    renderOnMiddlePage();

    fireEvent.mouseUp(window, { button: 4 });

    expect(screen.getByTestId('location')).toHaveTextContent('/history');
  });

  it('should go back and forward with Alt+Left and Alt+Right', () => {
    renderOnMiddlePage();

    fireEvent.keyDown(window, { key: 'ArrowLeft', altKey: true });
    expect(screen.getByTestId('location')).toHaveTextContent(/^\/$/);

    fireEvent.keyDown(window, { key: 'ArrowRight', altKey: true });
    expect(screen.getByTestId('location')).toHaveTextContent('/watch');
  });

  it('should leave plain arrow keys and the left mouse button to the page', () => {
    renderOnMiddlePage();

    fireEvent.keyDown(window, { key: 'ArrowLeft' });
    fireEvent.keyDown(window, { key: 'ArrowLeft', altKey: true, ctrlKey: true });
    fireEvent.mouseUp(window, { button: 0 });

    expect(screen.getByTestId('location')).toHaveTextContent('/watch');
  });
});
