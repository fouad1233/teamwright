import type { Project, Task } from '@/shared/api/types';
import { classNames } from '@/shared/lib/classNames';

import styles from './TaskBlocks.module.css';

interface TaskBlocksProps {
  project: Project;
  tasks: readonly Task[];
}

/**
 * A group of tasks at a glance: one small block for every task, in the
 * pigment of its status and in the order of the board. A block is hollow
 * while work on its task has not started.
 *
 * The blocks say nothing a screen reader could use, so they are hidden from
 * it. Put the count next to them in words.
 */
export function TaskBlocks({ project, tasks }: TaskBlocksProps) {
  return (
    <span className={styles.blocks} aria-hidden>
      {project.statuses.flatMap((status) =>
        tasks
          .filter((task) => task.statusId === status.id)
          .map((task) => (
            <span
              key={task.id}
              className={classNames(styles.block, status.category === 'todo' && styles.hollow)}
              data-tone={status.color}
            />
          )),
      )}
    </span>
  );
}
