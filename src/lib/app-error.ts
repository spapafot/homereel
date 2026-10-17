/** Error shape returned by every Rust command (`AppError` in `src-tauri/src/error.rs`). */
export type AppError = {
  code: string;
  message: string;
};

function isAppError(value: unknown): value is AppError {
  return (
    typeof value === 'object' &&
    value !== null &&
    typeof (value as AppError).code === 'string' &&
    typeof (value as AppError).message === 'string'
  );
}

export function toAppError(error: unknown): AppError {
  if (isAppError(error)) return error;
  if (error instanceof Error) return { code: 'internal', message: error.message };
  if (typeof error === 'string') return { code: 'internal', message: error };
  return { code: 'internal', message: 'Something went wrong.' };
}
