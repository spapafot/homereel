/** A tiny store for YouTube-style snackbar messages ("Saved to Watch later"). */
export type Toast = {
  id: number;
  message: string;
};

const DISPLAY_MS = 4_000;

let toasts: Toast[] = [];
let nextId = 1;
const listeners = new Set<() => void>();

function emit() {
  listeners.forEach((listener) => listener());
}

type ToastOptions = {
  /** Stays until dismissed or replaced: for slow work, and errors worth reading in full. */
  persistent?: boolean;
};

export function showToast(message: string, { persistent = false }: ToastOptions = {}): void {
  const toast = { id: nextId++, message };
  // Like YouTube, a new message replaces the one on screen.
  toasts = [toast];
  emit();
  if (!persistent) window.setTimeout(() => dismissToast(toast.id), DISPLAY_MS);
}

export function dismissToast(id: number): void {
  const remaining = toasts.filter((toast) => toast.id !== id);
  if (remaining.length !== toasts.length) {
    toasts = remaining;
    emit();
  }
}

export function getToasts(): Toast[] {
  return toasts;
}

export function subscribeToasts(listener: () => void): () => void {
  listeners.add(listener);
  return () => listeners.delete(listener);
}
