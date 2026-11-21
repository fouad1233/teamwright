import type { Task } from '@/shared/api/types';
import { classNames } from '@/shared/lib/classNames';

import styles from './ChecklistProgress.module.css';
import { isComplete, progressInWords, progressOf } from './checklistModel';

interface ChecklistProgressMarkProps {
  task: Pick<Task, 'checklist'>;
  className?: string;
}

/**
 * How far the checklist of a task has come, on a card or in a row: a small
 * box of ticks and "3/5" in the counting hand. A finished checklist is
 * drawn in the pigment of finished work. Nothing for a task without one.
 */
export function ChecklistProgressMark({ task, className }: ChecklistProgressMarkProps) {
  const progress = progressOf(task);
  if (progress === null) {
    return null;
  }
  const words = progressInWords(progress);
  return (
    <span
      className={classNames(styles.progress, isComplete(progress) && styles.complete, className)}
      data-tone={isComplete(progress) ? 'green' : undefined}
      role="img"
      aria-label={`Checklist: ${words}`}
      title={`Checklist: ${words}`}
    >
      <svg className={styles.mark} width="12" height="12" viewBox="0 0 12 12" aria-hidden>
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
        <path
          d="M3.6 6.1 5.3 7.8 8.5 4.4"
          fill="none"
          stroke="currentColor"
          strokeWidth="1.5"
          strokeLinecap="round"
          strokeLinejoin="round"
        />
      </svg>
      {progress.done}/{progress.total}
    </span>
  );
}
