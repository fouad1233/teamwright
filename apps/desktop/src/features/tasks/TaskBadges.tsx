import { Bot, CircleDashed, Flag, UserRound } from 'lucide-react';

import type { Assignee, Priority, Status, StatusColor } from '@/shared/api/types';
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

const PRIORITY_TONES: Record<Priority, StatusColor> = {
  urgent: 'red',
  high: 'amber',
  normal: 'blue',
  low: 'gray',
};

/** The name of a priority for people. */
export function priorityLabel(priority: Priority): string {
  return PRIORITY_LABELS[priority];
}

/** Says who is responsible for a task, in a word or two. */
export function assigneeLabel(assignee: Assignee | null): string {
  if (assignee === null) {
    return 'Unassigned';
  }
  return assignee.kind === 'manager' ? 'You' : 'Team member';
}

/** The name of a status in its color. */
export function StatusPill({ status }: { status: Status }) {
  return (
    <span className={styles.statusPill} data-tone={status.color}>
      <span className={styles.statusDot} />
      {status.name}
    </span>
  );
}

interface PriorityFlagProps {
  priority: Priority;
  /** Shows the name of the priority next to the flag. */
  withLabel?: boolean;
}

/** A flag in the color of a priority. */
export function PriorityFlag({ priority, withLabel = false }: PriorityFlagProps) {
  const label = priorityLabel(priority);
  return (
    <span
      className={styles.priority}
      data-tone={PRIORITY_TONES[priority]}
      title={withLabel ? undefined : `${label} priority`}
    >
      <Flag className={styles.priorityIcon} aria-hidden={withLabel} aria-label={label} />
      {withLabel && label}
    </span>
  );
}

/** A small round picture of who is responsible for a task. */
export function AssigneeAvatar({ assignee }: { assignee: Assignee | null }) {
  const label = assigneeLabel(assignee);
  if (assignee === null) {
    return (
      <span className={classNames(styles.avatar, styles.unassigned)} title={label}>
        <CircleDashed aria-label={label} />
      </span>
    );
  }
  const Icon = assignee.kind === 'manager' ? UserRound : Bot;
  return (
    <span className={classNames(styles.avatar, styles[assignee.kind])} title={label}>
      <Icon aria-label={label} />
    </span>
  );
}
