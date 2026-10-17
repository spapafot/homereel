import { EmptyState } from '@/components/common/EmptyState';

export function NotFoundPage() {
  return (
    <EmptyState icon="explore_off" title="This page isn’t available">
      Try going back to Home.
    </EmptyState>
  );
}
