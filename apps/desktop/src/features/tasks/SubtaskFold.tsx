import { ChevronRight } from 'lucide-react';

import type { Project, Task } from '@/shared/api/types';
import { classNames } from '@/shared/lib/classNames';

import styles from './SubtaskFold.module.css';
import { TaskBlocks } from './TaskBlocks';

interface SubtaskFoldProps {
  project: Project;
  /** The subtasks that the control shows or hides. */
  subtasks: readonly Task[];
  isOpen: boolean;
  onToggle: () => void;
  /** Lays the control out where it is used: as the last row of a card, or within a line. */
  className?: string;
}

/**
 * The control that shows or hides the subtasks of a task. It says how many
 * there are, with one block for each in the pigment of its status, so the
 * state of the parts can be read without opening them.
 */
export function SubtaskFold({ project, subtasks, isOpen, onToggle, className }: SubtaskFoldProps) {
  return (
    <button
      type="button"
      className={classNames(styles.fold, className)}
      aria-expanded={isOpen}
      // The control often sits inside something that opens the task when it
      // is pressed. A press on the control must not reach that.
      onClick={(event) => {
        event.stopPropagation();
        onToggle();
      }}
      onKeyDown={(event) => {
        event.stopPropagation();
      }}
    >
      <ChevronRight className={styles.chevron} aria-hidden />
      {subtasks.length} {subtasks.length === 1 ? 'subtask' : 'subtasks'}
      <span className={styles.blocks}>
        <TaskBlocks project={project} tasks={subtasks} />
      </span>
    </button>
  );
}
