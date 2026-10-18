import { PromptDialog } from '@/components/common/Modal';
import { useSetCustomTitle } from '@/hooks/use-video-actions';
import type { VideoSummary } from '@/lib/api';

type EditTitleDialogProps = {
  video: VideoSummary;
  open: boolean;
  onOpenChange: (open: boolean) => void;
};

/** Edits the title Homereel shows. The file keeps its name. */
export function EditTitleDialog({ video, open, onOpenChange }: EditTitleDialogProps) {
  const setTitle = useSetCustomTitle();

  return (
    <PromptDialog
      open={open}
      onOpenChange={onOpenChange}
      title="Edit title"
      label="Title"
      maxLength={200}
      initialValue={video.title}
      confirmLabel="Save"
      description={
        <>
          Shown only in Homereel. The file keeps its name, <strong>{video.fileName}</strong>.
        </>
      }
      secondaryAction={
        video.customTitle
          ? {
              label: `Reset to ${video.aiTitle ? 'AI title' : 'file name'}`,
              onClick: () => setTitle.mutateAsync({ videoId: video.id, title: null }),
            }
          : undefined
      }
      onConfirm={(title) =>
        // Saving the AI title or file name unchanged is a reset, not a custom title.
        setTitle.mutateAsync({
          videoId: video.id,
          title: title === (video.aiTitle ?? video.fileName) ? null : title,
        })
      }
    />
  );
}
