import type { ReactNode } from 'react';

import { Icon } from './Icon';

type EmptyStateProps = {
  icon: string;
  title: string;
  children?: ReactNode;
  action?: ReactNode;
};

export function EmptyState({ icon, title, children, action }: EmptyStateProps) {
  return (
    <div className="mx-auto flex max-w-lg flex-col items-center gap-4 px-6 py-24 text-center">
      <Icon name={icon} size={96} className="text-text-secondary" />
      <h2 className="text-2xl">{title}</h2>
      {children && <div className="text-sm leading-5 text-text-secondary">{children}</div>}
      {action}
    </div>
  );
}
