import { useSortable } from '@dnd-kit/sortable';
import { CSS } from '@dnd-kit/utilities';
import { Hourglass } from 'lucide-react';
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

/** How a task looks on the board: its title, and a line of facts below it. */
export function TaskCard({ project, task, unfinishedBlockerCount }: TaskCardProps) {
  const overdue = isOverdue(task);
  const hasRaisedPriority = task.priority === 'urgent' || task.priority === 'high';

  return (
    <div className={styles.card}>
      <p className={styles.title}>{task.title}</p>

      <div className={styles.facts}>
        <span className={styles.reference}>{taskReference(project, task)}</span>
        {hasRaisedPriority && <PriorityFlag priority={task.priority} />}
        {unfinishedBlockerCount > 0 && (
          <span className={styles.waiting} title="Waits for tasks that are not finished">
            <Hourglass />
            {unfinishedBlockerCount}
          </span>
        )}
        {task.dueOn !== null && (
          <span
            className={classNames(styles.fact, overdue && styles.overdue)}
            title={overdue ? 'Overdue' : 'Due date'}
          >
            {formatDate(task.dueOn)}
          </span>
        )}
        {task.estimateMinutes !== null && (
          <span className={styles.fact} title="Estimate">
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
