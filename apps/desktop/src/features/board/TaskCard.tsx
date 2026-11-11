import { useSortable } from '@dnd-kit/sortable';
import { CSS } from '@dnd-kit/utilities';
import { CalendarDays, Hourglass, Timer } from 'lucide-react';
import type { KeyboardEvent } from 'react';

import type { Project, Task } from '@/shared/api/types';
import { classNames } from '@/shared/lib/classNames';
import { formatDate, formatEstimate, isOverdue, taskReference } from '@/shared/lib/format';

import { AssigneeAvatar, PriorityFlag } from '../tasks/TaskBadges';
import styles from './TaskCard.module.css';

interface TaskCardProps {
  project: Project;
  task: Task;
  /** How many of the tasks this one waits for are not finished yet. */
  unfinishedBlockerCount: number;
}

/** How a task looks on the board. */
export function TaskCard({ project, task, unfinishedBlockerCount }: TaskCardProps) {
  return (
    <div className={styles.card}>
      <div className={styles.heading}>
        <span className={styles.reference}>{taskReference(project, task)}</span>
        {task.priority !== 'normal' && <PriorityFlag priority={task.priority} />}
      </div>

      <p className={styles.title}>{task.title}</p>

      <div className={styles.details}>
        {unfinishedBlockerCount > 0 && (
          <span
            className={classNames(styles.detail, styles.waiting)}
            title="Waits for tasks that are not finished"
          >
            <Hourglass />
            Waits for {unfinishedBlockerCount}
          </span>
        )}
        {task.dueOn !== null && (
          <span
            className={classNames(styles.detail, isOverdue(task) && styles.overdue)}
            title={isOverdue(task) ? 'Overdue' : 'Due date'}
          >
            <CalendarDays />
            {formatDate(task.dueOn)}
          </span>
        )}
        {task.estimateMinutes !== null && (
          <span className={styles.detail} title="Estimate">
            <Timer />
            {formatEstimate(task.estimateMinutes)}
          </span>
        )}
        <span className={styles.assignee}>
          <AssigneeAvatar assignee={task.assignee} />
        </span>
      </div>
    </div>
  );
}

interface SortableTaskCardProps extends TaskCardProps {
  /** Called when the user opens the task: a click, or Enter on the keyboard. */
  onOpen: () => void;
}

/**
 * A card that can be dragged to another position or status. With the
 * keyboard, Space picks it up and drops it, the arrow keys move it, and
 * Enter opens the task.
 */
export function SortableTaskCard({ onOpen, ...card }: SortableTaskCardProps) {
  const { attributes, listeners, setNodeRef, transform, transition, isDragging } = useSortable({
    id: card.task.id,
  });

  function handleKeyDown(event: KeyboardEvent<HTMLLIElement>) {
    if (event.key === 'Enter') {
      onOpen();
      return;
    }
    listeners?.onKeyDown?.(event);
  }

  return (
    <li
      ref={setNodeRef}
      className={classNames(styles.item, isDragging && styles.lifted)}
      style={{ transform: CSS.Translate.toString(transform), transition }}
      {...attributes}
      {...listeners}
      onKeyDown={handleKeyDown}
      onClick={onOpen}
    >
      <TaskCard {...card} />
    </li>
  );
}
