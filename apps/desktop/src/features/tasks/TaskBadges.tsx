import { Flag } from 'lucide-react';

import type { Color, Priority, Status } from '@/shared/api/types';
import { classNames } from '@/shared/lib/classNames';

import styles from './TaskBadges.module.css';

/** The priorities from the most to the least urgent, for menus. */
export const PRIORITIES: readonly Priority[] = ['urgent', 'high', 'normal', 'low'];

const PRIORITY_LABELS: Record<Priority, string> = {
  urgent: 'Urgent',
  high: 'High',
  normal: 'Normal',
  low: 'Low',
};

/** Only the priorities that call for attention get a pigment. */
const PRIORITY_TONES: Record<Priority, Color> = {
  urgent: 'red',
  high: 'amber',
  normal: 'gray',
  low: 'gray',
};

/** The name of a priority for people. */
export function priorityLabel(priority: Priority): string {
  return PRIORITY_LABELS[priority];
}

/**
 * The mark of a status: a small block in the pigment of the status, filled
 * according to how far the work is. Empty for work that has not started,
 * half filled for work under way, a block within a block for work that
 * waits for a check, and solid with a tick for work that is done.
 */
export function StatusMark({ status }: { status: Pick<Status, 'category' | 'color'> }) {
  const outline = (
    <rect
      x="1.25"
      y="1.25"
      width="9.5"
      height="9.5"
      rx="2"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.5"
    />
  );

  return (
    <svg
      className={styles.statusMark}
      data-tone={status.color}
      width="12"
      height="12"
      viewBox="0 0 12 12"
      aria-hidden
    >
      {status.category === 'todo' && outline}
      {status.category === 'active' && (
        <>
          {outline}
          <path d="M2 6h8v2.5A1.5 1.5 0 0 1 8.5 10h-5A1.5 1.5 0 0 1 2 8.5Z" fill="currentColor" />
        </>
      )}
      {status.category === 'review' && (
        <>
          {outline}
          <rect x="4" y="4" width="4" height="4" rx="0.75" fill="currentColor" />
        </>
      )}
      {status.category === 'done' && (
        <>
          <rect x="0.5" y="0.5" width="11" height="11" rx="2.5" fill="currentColor" />
          <path
            d="M3.4 6.2 5.2 8l3.4-3.7"
            fill="none"
            stroke="var(--ground)"
            strokeWidth="1.5"
            strokeLinecap="round"
            strokeLinejoin="round"
          />
        </>
      )}
    </svg>
  );
}

/** The mark of a status followed by its name. */
export function StatusLabel({ status }: { status: Status }) {
  return (
    <span className={styles.statusLabel}>
      <StatusMark status={status} />
      {status.name}
    </span>
  );
}

interface PriorityFlagProps {
  priority: Priority;
  /** Shows the name of the priority next to the flag. */
  withLabel?: boolean;
}

/** A flag for the priority of a task. High and urgent flags are colored. */
export function PriorityFlag({ priority, withLabel = false }: PriorityFlagProps) {
  const label = priorityLabel(priority);
  const callsForAttention = priority === 'urgent' || priority === 'high';
  return (
    <span
      className={classNames(styles.priority, callsForAttention && styles.raised)}
      data-tone={PRIORITY_TONES[priority]}
      title={withLabel ? undefined : `${label} priority`}
    >
      <Flag aria-hidden={withLabel} aria-label={label} />
      {withLabel && label}
    </span>
  );
}
