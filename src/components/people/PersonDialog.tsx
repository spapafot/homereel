import { Modal } from '@/components/common/Modal';

import { PersonForm } from './PersonForm';
import type { PersonDraft } from './person-draft';

type PersonDialogProps = {
  /** Open while set; `id: null` adds someone new. */
  draft: PersonDraft | null;
  onClose: () => void;
};

export function PersonDialog({ draft, onClose }: PersonDialogProps) {
  return (
    <Modal
      open={draft != null}
      onOpenChange={(open) => !open && onClose()}
      title={draft?.id != null ? `Edit ${draft.name}` : 'Add a person or pet'}
      className="w-[min(92vw,560px)]"
    >
      {draft && <PersonForm initial={draft} onDone={onClose} onCancel={onClose} />}
    </Modal>
  );
}
