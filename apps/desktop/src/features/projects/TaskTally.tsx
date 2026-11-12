import type { Project, Task } from '@/shared/api/types';
import { classNames } from '@/shared/lib/classNames';

import { tasksOfStatus } from '../tasks/taskModel';
import styles from './TaskTally.module.css';

/** Above this number of tasks the blocks no longer fit, and only the count is shown. */
const MOST_BLOCKS = 48;

interface TaskTallyProps {
  project: Project;
  tasks: Task[];
}

/**
 * The state of a project at a glance: one small block for every task, in the
 * pigment of its status and in the order of the board, followed by the
 * count. A block is hollow while work on its task has not started.
 */
export function TaskTally({ project, tasks }: TaskTallyProps) {
  const finishedCount = tasks.filter((task) => task.completedAt !== null).length;
  const perStatus = project.statuses.map((status) => ({
    status,
    count: tasksOfStatus(tasks, status.id).length,
  }));
  const summary = perStatus.map(({ status, count }) => `${count.toString()} ${status.name}`);

  return (
    <div className={styles.tally} title={summary.join(' · ')}>
      {tasks.length > 0 && tasks.length <= MOST_BLOCKS && (
        <span className={styles.blocks} aria-hidden>
          {perStatus.flatMap(({ status, count }) =>
            Array.from({ length: count }, (_, position) => (
              <span
                key={`${status.id}:${position.toString()}`}
                className={classNames(styles.block, status.category === 'todo' && styles.hollow)}
                data-tone={status.color}
              />
            )),
          )}
        </span>
      )}
      <span className={styles.count}>
        {tasks.length === 0
          ? 'No tasks'
          : `${tasks.length.toString()} ${tasks.length === 1 ? 'task' : 'tasks'} · ${finishedCount.toString()} done`}
      </span>
    </div>
  );
}
