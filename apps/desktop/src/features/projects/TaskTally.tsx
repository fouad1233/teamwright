import type { Project, Task } from '@/shared/api/types';

import { TaskBlocks } from '../tasks/TaskBlocks';
import { tasksOfStatus } from '../tasks/taskModel';
import styles from './TaskTally.module.css';

/** Above this number of tasks the blocks no longer fit, and only the count is shown. */
const MOST_BLOCKS = 48;

interface TaskTallyProps {
  project: Project;
  /** The tasks counted: those of the project, or of its open area. */
  tasks: Task[];
  /** The name of the open area, when the tasks are those of one area. */
  areaName?: string | undefined;
}

/**
 * The state of a project, or of one of its areas, at a glance: one small
 * block for every task, followed by the count.
 */
export function TaskTally({ project, tasks, areaName }: TaskTallyProps) {
  const finishedCount = tasks.filter((task) => task.completedAt !== null).length;
  const summary = project.statuses.map(
    (status) => `${tasksOfStatus(tasks, status.id).length.toString()} ${status.name}`,
  );
  const where = areaName === undefined ? '' : ` in ${areaName}`;

  return (
    <div className={styles.tally} title={summary.join(' · ')}>
      {tasks.length > 0 && tasks.length <= MOST_BLOCKS && (
        <TaskBlocks project={project} tasks={tasks} />
      )}
      <span className={styles.count}>
        {tasks.length === 0
          ? `No tasks${where}`
          : `${tasks.length.toString()} ${tasks.length === 1 ? 'task' : 'tasks'}${where} · ${finishedCount.toString()} done`}
      </span>
    </div>
  );
}
