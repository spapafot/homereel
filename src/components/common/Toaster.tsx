import { useSyncExternalStore } from 'react';

import { dismissToast, getToasts, subscribeToasts } from '@/lib/toast';

/** Bottom-left snackbar, as on YouTube. */
export function Toaster() {
  const toasts = useSyncExternalStore(subscribeToasts, getToasts);

  return (
    <div
      aria-live="polite"
      className="pointer-events-none fixed bottom-6 left-6 z-[60] flex flex-col gap-2"
    >
      {toasts.map((toast) => (
        <div
          key={toast.id}
          role="status"
          className="pointer-events-auto flex max-w-md items-center gap-6 rounded-lg bg-[#0f0f0f] px-4 py-3.5 text-sm text-white shadow-lg dark:bg-[#f1f1f1] dark:text-[#0f0f0f]"
        >
          <span>{toast.message}</span>
          <button
            type="button"
            onClick={() => dismissToast(toast.id)}
            className="ml-auto text-sm font-medium text-[#3ea6ff] dark:text-[#065fd4]"
          >
            Dismiss
          </button>
        </div>
      ))}
    </div>
  );
}
