import * as DropdownMenu from '@radix-ui/react-dropdown-menu';
import { useState } from 'react';

import { Icon } from '@/components/common/Icon';
import { IconButton } from '@/components/common/IconButton';
import {
  MENU_CONTENT_CLASS,
  MENU_ITEM_CLASS,
  MENU_SEPARATOR_CLASS,
} from '@/components/common/menu-styles';
import { useGeminiSettings } from '@/hooks/use-titles';
import {
  useRegeneratePreview,
  useRegenerateTitle,
  useToggleFavorite,
  useToggleWatchLater,
} from '@/hooks/use-video-actions';
import type { VideoSummary } from '@/lib/api';
import { cx } from '@/lib/class-names';
import { showToast } from '@/lib/toast';
import { revealVideo } from '@/lib/video-source';

import { EditTitleDialog } from './EditTitleDialog';
import { SaveToPlaylistDialog } from './SaveToPlaylistDialog';

export type VideoMenuAction = {
  icon: string;
  label: string;
  onSelect: () => void;
};

type VideoMenuProps = {
  video: VideoSummary;
  /** Page-specific actions shown first, such as "Remove from Watch later". */
  extraActions?: VideoMenuAction[];
  /** The watch page uses YouTube's horizontal ⋯ instead of the cards' ⋮. */
  icon?: 'more_vert' | 'more_horiz';
  className?: string;
};

/** The ⋮ action menu on cards, rows and the watch page. */
export function VideoMenu({
  video,
  extraActions = [],
  icon = 'more_vert',
  className,
}: VideoMenuProps) {
  const [dialog, setDialog] = useState<'playlist' | 'title' | null>(null);
  const toggleFavorite = useToggleFavorite();
  const toggleWatchLater = useToggleWatchLater();
  const regenerate = useRegenerateTitle();
  const regeneratePreview = useRegeneratePreview();
  const gemini = useGeminiSettings();

  const copyFileName = () => {
    navigator.clipboard
      .writeText(video.fileName)
      .then(() => showToast('File name copied'))
      .catch(() => showToast('Could not copy the file name'));
  };

  return (
    <>
      <DropdownMenu.Root>
        <DropdownMenu.Trigger asChild>
          <IconButton
            icon={icon}
            label="Action menu"
            className={cx('data-[state=open]:opacity-100', className)}
          />
        </DropdownMenu.Trigger>
        <DropdownMenu.Portal>
          <DropdownMenu.Content align="end" sideOffset={4} className={MENU_CONTENT_CLASS}>
            {extraActions.map((action) => (
              <DropdownMenu.Item
                key={action.label}
                className={MENU_ITEM_CLASS}
                onSelect={action.onSelect}
              >
                <Icon name={action.icon} />
                {action.label}
              </DropdownMenu.Item>
            ))}
            {extraActions.length > 0 && <DropdownMenu.Separator className={MENU_SEPARATOR_CLASS} />}
            <DropdownMenu.Item
              className={MENU_ITEM_CLASS}
              onSelect={() => toggleWatchLater.mutate({ video, value: !video.isWatchLater })}
            >
              <Icon name="schedule" filled={video.isWatchLater} />
              {video.isWatchLater ? 'Remove from Watch later' : 'Save to Watch later'}
            </DropdownMenu.Item>
            <DropdownMenu.Item
              className={MENU_ITEM_CLASS}
              onSelect={() => toggleFavorite.mutate({ video, value: !video.isFavorite })}
            >
              <Icon name="favorite" filled={video.isFavorite} />
              {video.isFavorite ? 'Remove from Favorites' : 'Add to Favorites'}
            </DropdownMenu.Item>
            <DropdownMenu.Item className={MENU_ITEM_CLASS} onSelect={() => setDialog('playlist')}>
              <Icon name="playlist_add" />
              Save to playlist
            </DropdownMenu.Item>
            <DropdownMenu.Separator className={MENU_SEPARATOR_CLASS} />
            <DropdownMenu.Item className={MENU_ITEM_CLASS} onSelect={() => setDialog('title')}>
              <Icon name="edit" />
              Edit title
            </DropdownMenu.Item>
            {gemini.data?.keyConfigured && (
              <DropdownMenu.Item
                className={MENU_ITEM_CLASS}
                disabled={regenerate.isPending}
                onSelect={() => regenerate.mutate(video.id)}
              >
                <Icon name="auto_awesome" />
                {video.aiTitle ? 'Regenerate AI title' : 'Generate AI title'}
              </DropdownMenu.Item>
            )}
            <DropdownMenu.Item
              className={MENU_ITEM_CLASS}
              disabled={regeneratePreview.isPending}
              onSelect={() => regeneratePreview.mutate({ videoId: video.id })}
            >
              <Icon name="image" />
              Regenerate preview
            </DropdownMenu.Item>
            <DropdownMenu.Separator className={MENU_SEPARATOR_CLASS} />
            <DropdownMenu.Item className={MENU_ITEM_CLASS} onSelect={() => revealVideo(video)}>
              <Icon name={video.source === 'local' ? 'folder_open' : 'open_in_new'} />
              {video.source === 'local' ? 'Show in folder' : 'Open in Google Drive'}
            </DropdownMenu.Item>
            <DropdownMenu.Item className={MENU_ITEM_CLASS} onSelect={copyFileName}>
              <Icon name="content_copy" />
              Copy file name
            </DropdownMenu.Item>
          </DropdownMenu.Content>
        </DropdownMenu.Portal>
      </DropdownMenu.Root>
      <SaveToPlaylistDialog
        videoId={video.id}
        open={dialog === 'playlist'}
        onOpenChange={(open) => setDialog(open ? 'playlist' : null)}
      />
      <EditTitleDialog
        video={video}
        open={dialog === 'title'}
        onOpenChange={(open) => setDialog(open ? 'title' : null)}
      />
    </>
  );
}
