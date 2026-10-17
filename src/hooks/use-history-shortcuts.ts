import { useEffect } from 'react';
import { useNavigate } from 'react-router';

/** `MouseEvent.button` values of the side buttons most mice have. */
const MOUSE_BACK_BUTTON = 3;
const MOUSE_FORWARD_BUTTON = 4;

/** Mouse side buttons and Alt+Left/Right move through history, as they do in a browser. */
export function useHistoryShortcuts() {
  const navigate = useNavigate();

  useEffect(() => {
    const onMouseUp = (event: MouseEvent) => {
      if (event.button !== MOUSE_BACK_BUTTON && event.button !== MOUSE_FORWARD_BUTTON) return;
      // WebView2 may also navigate on these buttons; cancelling it keeps one press one step.
      event.preventDefault();
      navigate(event.button === MOUSE_BACK_BUTTON ? -1 : 1);
    };
    const onKeyDown = (event: KeyboardEvent) => {
      if (!event.altKey || event.ctrlKey || event.metaKey || event.shiftKey || event.repeat) {
        return;
      }
      if (event.key !== 'ArrowLeft' && event.key !== 'ArrowRight') return;
      event.preventDefault();
      navigate(event.key === 'ArrowLeft' ? -1 : 1);
    };

    window.addEventListener('mouseup', onMouseUp);
    window.addEventListener('keydown', onKeyDown);
    return () => {
      window.removeEventListener('mouseup', onMouseUp);
      window.removeEventListener('keydown', onKeyDown);
    };
  }, [navigate]);
}
