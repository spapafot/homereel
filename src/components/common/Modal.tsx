import * as Dialog from '@radix-ui/react-dialog';
import { useState, type FormEvent, type ReactNode } from 'react';

import { toAppError } from '@/lib/app-error';
import { cx } from '@/lib/class-names';

import { Button } from './Button';

type ModalProps = {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  title: string;
  description?: ReactNode;
  children?: ReactNode;
  className?: string;
};

/** YouTube-style dialog: rounded panel over a dimmed page. */
export function Modal({ open, onOpenChange, title, description, children, className }: ModalProps) {
  return (
    <Dialog.Root open={open} onOpenChange={onOpenChange}>
      <Dialog.Portal>
        <Dialog.Overlay className="fixed inset-0 z-50 bg-black/50" />
        <Dialog.Content
          className={cx(
            'fixed top-1/2 left-1/2 z-50 w-[min(92vw,420px)] -translate-x-1/2 -translate-y-1/2',
            'rounded-xl bg-menu p-6 text-text shadow-2xl outline-none',
            className,
          )}
        >
          <Dialog.Title className="text-base font-medium">{title}</Dialog.Title>
          {description ? (
            <Dialog.Description className="mt-2 text-sm text-text-secondary">
              {description}
            </Dialog.Description>
          ) : (
            <Dialog.Description className="sr-only">{title}</Dialog.Description>
          )}
          {children}
        </Dialog.Content>
      </Dialog.Portal>
    </Dialog.Root>
  );
}

type PromptDialogProps = {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  title: string;
  label: string;
  initialValue?: string;
  confirmLabel: string;
  description?: ReactNode;
  maxLength?: number;
  allowEmpty?: boolean;
  /** Extra button on the left, e.g. "Reset". */
  secondaryAction?: { label: string; onClick: () => Promise<unknown> };
  onConfirm: (value: string) => Promise<unknown>;
};

/** A dialog with one text field, used for naming playlists and editing titles. */
export function PromptDialog(props: PromptDialogProps) {
  return (
    <Modal
      open={props.open}
      onOpenChange={props.onOpenChange}
      title={props.title}
      description={props.description}
    >
      {/* Remounting the form on every open resets the field to `initialValue`. */}
      {props.open && <PromptForm {...props} />}
    </Modal>
  );
}

function PromptForm({
  onOpenChange,
  label,
  initialValue = '',
  confirmLabel,
  maxLength = 150,
  allowEmpty = false,
  secondaryAction,
  onConfirm,
}: PromptDialogProps) {
  const [value, setValue] = useState(initialValue);
  const [error, setError] = useState<string | null>(null);
  const [isSaving, setIsSaving] = useState(false);

  const run = async (action: () => Promise<unknown>) => {
    setIsSaving(true);
    setError(null);
    try {
      await action();
      onOpenChange(false);
    } catch (caught) {
      setError(toAppError(caught).message);
    } finally {
      setIsSaving(false);
    }
  };

  const submit = (event: FormEvent) => {
    event.preventDefault();
    if (!allowEmpty && !value.trim()) return;
    void run(() => onConfirm(value.trim()));
  };

  return (
    <form onSubmit={submit} className="mt-4">
      <label className="block">
        <span className="text-xs text-text-secondary">{label}</span>
        <input
          autoFocus
          value={value}
          maxLength={maxLength}
          onChange={(event) => setValue(event.target.value)}
          className="mt-1 w-full border-b border-divider bg-transparent pb-1 text-base outline-none focus:border-b-2 focus:border-text"
        />
      </label>
      <p className="mt-1 text-right text-xs text-text-secondary">
        {value.length}/{maxLength}
      </p>
      {error && <p className="mt-2 text-sm text-red-600 dark:text-red-400">{error}</p>}
      <div className="mt-6 flex items-center gap-2">
        {secondaryAction && (
          <Button disabled={isSaving} onClick={() => void run(secondaryAction.onClick)}>
            {secondaryAction.label}
          </Button>
        )}
        <div className="ml-auto flex gap-2">
          <Button disabled={isSaving} onClick={() => onOpenChange(false)}>
            Cancel
          </Button>
          <Button
            type="submit"
            variant="primary"
            disabled={isSaving || (!allowEmpty && !value.trim())}
          >
            {confirmLabel}
          </Button>
        </div>
      </div>
    </form>
  );
}

type ConfirmDialogProps = {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  title: string;
  description: ReactNode;
  confirmLabel: string;
  onConfirm: () => void;
};

export function ConfirmDialog({
  open,
  onOpenChange,
  title,
  description,
  confirmLabel,
  onConfirm,
}: ConfirmDialogProps) {
  return (
    <Modal open={open} onOpenChange={onOpenChange} title={title} description={description}>
      <div className="mt-6 flex justify-end gap-2">
        <Button onClick={() => onOpenChange(false)}>Cancel</Button>
        <Button
          variant="primary"
          onClick={() => {
            onConfirm();
            onOpenChange(false);
          }}
        >
          {confirmLabel}
        </Button>
      </div>
    </Modal>
  );
}
