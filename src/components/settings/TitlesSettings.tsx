import { useState, type FormEvent } from 'react';

import { Button } from '@/components/common/Button';
import { Icon } from '@/components/common/Icon';
import { ConfirmDialog } from '@/components/common/Modal';
import {
  useGeminiModels,
  useGeminiSettings,
  usePauseTitles,
  useRemoveGeminiKey,
  useSetGeminiInstructions,
  useSetGeminiKey,
  useSetGeminiModel,
  useStartTitles,
  useTitlesStatus,
} from '@/hooks/use-titles';
import { MAX_GEMINI_INSTRUCTIONS, type TitleQueueStatus } from '@/lib/api';
import { toAppError } from '@/lib/app-error';

/** Gemini key and model, and the background job that titles every video once. */
export function TitlesSettings() {
  const gemini = useGeminiSettings();
  const keyConfigured = gemini.data?.keyConfigured ?? false;

  return (
    <div className="flex flex-col gap-6">
      <p className="text-sm leading-5 text-text-secondary">
        DriveTube can name each video once with Gemini, from five small frames and the file’s
        details (about $0.002 per video). Titles are stored only in DriveTube; files in Google Drive
        are never renamed. Use a key from a project with billing enabled: on Gemini’s free tier,
        Google may use the frames to improve its products.
      </p>
      <ApiKeyField keyConfigured={keyConfigured} />
      {keyConfigured && gemini.data && <ModelField model={gemini.data.model} />}
      {keyConfigured && gemini.data && <InstructionsField saved={gemini.data.instructions} />}
      <TitleProgress keyConfigured={keyConfigured} />
    </div>
  );
}

function ApiKeyField({ keyConfigured }: { keyConfigured: boolean }) {
  const setKey = useSetGeminiKey();
  const removeKey = useRemoveGeminiKey();
  const [editing, setEditing] = useState(false);
  const [keyText, setKeyText] = useState('');

  if (keyConfigured && !editing) {
    return (
      <div className="flex flex-wrap items-center gap-3">
        <Icon name="key" className="text-text-secondary" />
        <p className="flex-1 text-sm">Gemini API key saved in Windows Credential Manager</p>
        <Button onClick={() => setEditing(true)}>Replace</Button>
        <Button onClick={() => removeKey.mutate()} disabled={removeKey.isPending}>
          Remove
        </Button>
      </div>
    );
  }

  const submit = (event: FormEvent) => {
    event.preventDefault();
    if (!keyText.trim()) return;
    setKey.mutate(keyText.trim(), {
      onSuccess: () => {
        setEditing(false);
        setKeyText('');
      },
    });
  };

  return (
    <form onSubmit={submit} className="flex flex-col gap-2">
      <label className="text-sm font-medium" htmlFor="gemini-key">
        Gemini API key
      </label>
      <div className="flex flex-wrap gap-2">
        <input
          id="gemini-key"
          type="password"
          autoComplete="off"
          value={keyText}
          onChange={(event) => setKeyText(event.target.value)}
          placeholder="Paste your key from Google AI Studio"
          className="h-9 min-w-0 flex-1 rounded-lg border border-search-border bg-search-bg px-3 text-sm outline-none focus:border-focus"
        />
        <Button type="submit" variant="primary" disabled={!keyText.trim() || setKey.isPending}>
          {setKey.isPending ? 'Checking…' : 'Save key'}
        </Button>
        {editing && <Button onClick={() => setEditing(false)}>Cancel</Button>}
      </div>
      {setKey.error && (
        <p className="text-sm text-red-600 dark:text-red-400">{toAppError(setKey.error).message}</p>
      )}
    </form>
  );
}

function ModelField({ model }: { model: string }) {
  const models = useGeminiModels(true);
  const setModel = useSetGeminiModel();
  const options = models.data?.includes(model) ? models.data : [model, ...(models.data ?? [])];

  return (
    <label className="flex flex-wrap items-center gap-3 text-sm">
      <span className="font-medium">Model</span>
      <select
        value={model}
        onChange={(event) => setModel.mutate(event.target.value)}
        className="h-9 rounded-lg border border-search-border bg-search-bg px-2 outline-none focus:border-focus"
      >
        {options.map((option) => (
          <option key={option} value={option}>
            {option}
          </option>
        ))}
      </select>
      {models.isError && (
        <span className="text-text-secondary">Could not load the model list.</span>
      )}
    </label>
  );
}

const INSTRUCTIONS_PLACEHOLDER =
  'For example: The baby in videos recorded after 24 August 2024 is almost always our daughter Anna.';

/** The owner's notes, sent with every title request. */
function InstructionsField({ saved }: { saved: string }) {
  const setInstructions = useSetGeminiInstructions();
  const [text, setText] = useState(saved);
  const changed = text.trim() !== saved;

  const submit = (event: FormEvent) => {
    event.preventDefault();
    setInstructions.mutate(text);
  };

  return (
    <form onSubmit={submit} className="flex flex-col gap-2">
      <label className="text-sm font-medium" htmlFor="gemini-instructions">
        Instructions for Gemini
      </label>
      <p id="gemini-instructions-help" className="text-sm leading-5 text-text-secondary">
        Sent with every video, for what Gemini can’t tell from the frames, such as who usually
        appears when. Gemini also gets the date each video was recorded, when the file has one.
        Existing titles change only when you regenerate them.
      </p>
      <textarea
        id="gemini-instructions"
        aria-describedby="gemini-instructions-help"
        value={text}
        rows={4}
        maxLength={MAX_GEMINI_INSTRUCTIONS}
        onChange={(event) => setText(event.target.value)}
        placeholder={INSTRUCTIONS_PLACEHOLDER}
        className="resize-y rounded-lg border border-search-border bg-search-bg px-3 py-2 text-sm leading-5 outline-none focus:border-focus"
      />
      <div className="flex items-center gap-3">
        <Button type="submit" variant="primary" disabled={!changed || setInstructions.isPending}>
          Save instructions
        </Button>
        <span className="text-xs text-text-secondary">
          {text.length} / {MAX_GEMINI_INSTRUCTIONS}
        </span>
      </div>
    </form>
  );
}

function queueMessage(queue: TitleQueueStatus): string {
  if (queue.scanning) return 'Finding videos without a title…';
  if (queue.stopping) return 'Pausing after the videos in progress…';
  return `Generating titles: ${queue.completed + queue.failed} of ${queue.total}`;
}

function TitleProgress({ keyConfigured }: { keyConfigured: boolean }) {
  const status = useTitlesStatus();
  const start = useStartTitles();
  const pause = usePauseTitles();
  const [confirmRegenerate, setConfirmRegenerate] = useState(false);
  const queue = status.data?.queue;
  const counts = status.data?.counts;

  if (queue?.running) {
    const processed = queue.completed + queue.failed;
    const percent = queue.total > 0 ? (processed / queue.total) * 100 : 0;
    return (
      <div className="flex flex-col gap-3">
        <div className="flex items-center justify-between gap-4 text-sm">
          <span>{queueMessage(queue)}</span>
          <Button onClick={() => pause.mutate()} disabled={queue.stopping}>
            Pause
          </Button>
        </div>
        <div
          className="h-1 overflow-hidden rounded-full bg-chip"
          role="progressbar"
          aria-valuenow={Math.round(percent)}
          aria-label="Title generation progress"
        >
          <div className="h-full bg-progress transition-[width]" style={{ width: `${percent}%` }} />
        </div>
        {queue.failed > 0 && (
          <p className="text-xs text-text-secondary">{queue.failed} could not be titled so far.</p>
        )}
      </div>
    );
  }

  return (
    <div className="flex flex-col gap-3">
      {counts && (
        <p className="text-sm">
          {counts.done} {counts.done === 1 ? 'video has' : 'videos have'} an AI title
          {counts.failed > 0 ? `, ${counts.failed} could not be titled` : ''}.
        </p>
      )}
      {queue?.pauseReason && (
        <p className="text-sm text-red-600 dark:text-red-400">Stopped: {queue.pauseReason}</p>
      )}
      <div className="flex flex-wrap gap-2">
        <Button
          variant="primary"
          icon="auto_awesome"
          disabled={!keyConfigured || start.isPending}
          onClick={() => start.mutate('missing')}
        >
          Generate missing titles
        </Button>
        {(counts?.failed ?? 0) > 0 && (
          <Button
            disabled={!keyConfigured || start.isPending}
            onClick={() => start.mutate('missingAndFailed')}
          >
            Retry failed
          </Button>
        )}
        {(counts?.done ?? 0) > 0 && (
          <Button
            icon="refresh"
            disabled={!keyConfigured || start.isPending}
            onClick={() => setConfirmRegenerate(true)}
          >
            Regenerate all titles
          </Button>
        )}
      </div>
      <ConfirmDialog
        open={confirmRegenerate}
        onOpenChange={setConfirmRegenerate}
        title="Regenerate all AI titles?"
        description="Gemini titles every video again, so new people and pets can be named. This costs about the same as generating them the first time. Videos you renamed yourself are skipped."
        confirmLabel="Regenerate all"
        onConfirm={() => start.mutate('all')}
      />
    </div>
  );
}
