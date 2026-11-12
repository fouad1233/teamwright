import type { Project, Task } from '@/shared/api/types';

import { TaskBlocks } from '../tasks/TaskBlocks';
import { tasksOfStatus } from '../tasks/taskModel';
import styles from './TaskTally.module.css';

/** Above this number of tasks the blocks no longer fit, and only the count is shown. */
const MOST_BLOCKS = 48;

interface TaskTallyProps {
  project: Project;
  tasks: Task[];
}

/**
 * The state of a project at a glance: one small block for every task,
 * followed by the count.
 */
export function TaskTally({ project, tasks }: TaskTallyProps) {
  const finishedCount = tasks.filter((task) => task.completedAt !== null).length;
  const summary = project.statuses.map(
    (status) => `${tasksOfStatus(tasks, status.id).length.toString()} ${status.name}`,
  );

  return (
    <div className={styles.tally} title={summary.join(' · ')}>
      {tasks.length > 0 && tasks.length <= MOST_BLOCKS && (
        <TaskBlocks project={project} tasks={tasks} />
      )}
      <span className={styles.count}>
        {tasks.length === 0
          ? 'No tasks'
          : `${tasks.length.toString()} ${tasks.length === 1 ? 'task' : 'tasks'} · ${finishedCount.toString()} done`}
      </span>
    </div>
  );
}
