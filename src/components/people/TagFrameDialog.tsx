import { useQuery } from '@tanstack/react-query';
import { useState } from 'react';

import { Button } from '@/components/common/Button';
import { Icon } from '@/components/common/Icon';
import { Modal } from '@/components/common/Modal';
import { usePeople } from '@/hooks/use-people';
import { api, type Person } from '@/lib/api';
import { toAppError } from '@/lib/app-error';

import { PersonAvatar } from './PersonAvatar';
import { PersonForm } from './PersonForm';
import { PhotoCropper } from './PhotoCropper';
import { draftFromPerson, newPersonDraft, type PersonDraft } from './person-draft';

type Step =
  { name: 'crop' } | { name: 'who'; photo: string } | { name: 'details'; draft: PersonDraft };

type TagFrameDialogProps = {
  videoId: string;
  /** Open while set: the paused position to take the frame from. */
  positionMs: number | null;
  onClose: () => void;
};

/** Crops a person or pet out of the current frame and adds the photo to someone. */
export function TagFrameDialog({ videoId, positionMs, onClose }: TagFrameDialogProps) {
  return (
    <Modal
      open={positionMs != null}
      onOpenChange={(open) => !open && onClose()}
      title="Tag a person or pet"
      className="w-[min(92vw,640px)]"
    >
      {positionMs != null && (
        <TagFrameSteps videoId={videoId} positionMs={positionMs} onClose={onClose} />
      )}
    </Modal>
  );
}

function TagFrameSteps({
  videoId,
  positionMs,
  onClose,
}: {
  videoId: string;
  positionMs: number;
  onClose: () => void;
}) {
  const people = usePeople();
  const [step, setStep] = useState<Step>({ name: 'crop' });
  const frame = useQuery({
    queryKey: ['video-frame', videoId, positionMs],
    queryFn: () => api.videoFrameCapture(videoId, positionMs),
    staleTime: Infinity,
    gcTime: 0,
    retry: false,
  });

  if (step.name === 'details') {
    return <PersonForm initial={step.draft} onDone={onClose} onCancel={onClose} />;
  }
  if (step.name === 'who') {
    return (
      <WhoStep
        people={people.data ?? []}
        onBack={() => setStep({ name: 'crop' })}
        onChoose={(person) =>
          setStep({
            name: 'details',
            draft: person ? draftFromPerson(person, [step.photo]) : newPersonDraft([step.photo]),
          })
        }
      />
    );
  }
  if (frame.isPending) {
    return (
      <div className="mt-4 flex aspect-video animate-pulse items-center justify-center rounded-lg bg-skeleton text-sm text-text-secondary">
        Capturing the frame…
      </div>
    );
  }
  if (frame.isError) {
    return (
      <>
        <p className="mt-4 text-sm text-red-600 dark:text-red-400">
          Could not capture this frame ({toAppError(frame.error).message}). Try pausing on another
          moment.
        </p>
        <div className="mt-6 flex justify-end">
          <Button onClick={onClose}>Close</Button>
        </div>
      </>
    );
  }
  return (
    <PhotoCropper
      src={frame.data}
      onCancel={onClose}
      onCrop={(photo) =>
        setStep(
          people.data?.length
            ? { name: 'who', photo }
            : { name: 'details', draft: newPersonDraft([photo]) },
        )
      }
    />
  );
}

const SOMEONE_NEW = 'new';

function WhoStep({
  people,
  onBack,
  onChoose,
}: {
  people: Person[];
  onBack: () => void;
  onChoose: (person: Person | null) => void;
}) {
  const [choice, setChoice] = useState<number | typeof SOMEONE_NEW>(SOMEONE_NEW);

  return (
    <div className="mt-4">
      <p id="tag-who-label" className="text-sm text-text-secondary">
        Who is this? Adding another photo of someone helps Gemini recognize them.
      </p>
      <div
        role="radiogroup"
        aria-labelledby="tag-who-label"
        className="mt-2 flex max-h-80 flex-col overflow-y-auto"
      >
        {people.map((person) => (
          <label key={person.id} className="flex h-14 cursor-pointer items-center gap-4 text-sm">
            <input
              type="radio"
              name="tag-who"
              checked={choice === person.id}
              onChange={() => setChoice(person.id)}
              className="size-4 accent-link"
            />
            <PersonAvatar person={person} />
            <span className="truncate">{person.name}</span>
          </label>
        ))}
        <label className="flex h-14 cursor-pointer items-center gap-4 text-sm">
          <input
            type="radio"
            name="tag-who"
            checked={choice === SOMEONE_NEW}
            onChange={() => setChoice(SOMEONE_NEW)}
            className="size-4 accent-link"
          />
          <span className="flex size-10 items-center justify-center rounded-full bg-chip">
            <Icon name="person_add" />
          </span>
          Someone new
        </label>
      </div>
      <div className="mt-6 flex justify-end gap-2">
        <Button onClick={onBack}>Back</Button>
        <Button
          variant="primary"
          onClick={() =>
            onChoose(
              choice === SOMEONE_NEW
                ? null
                : (people.find((person) => person.id === choice) ?? null),
            )
          }
        >
          Next
        </Button>
      </div>
    </div>
  );
}
