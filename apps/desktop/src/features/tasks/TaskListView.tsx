import { useNavigation } from '@/app/navigation';
import type { Project, Task } from '@/shared/api/types';
import { classNames } from '@/shared/lib/classNames';
import { formatDate, formatEstimate, isOverdue, taskReference } from '@/shared/lib/format';

import { AssigneeAvatar, PriorityFlag, StatusPill, assigneeLabel } from './TaskBadges';
import styles from './TaskListView.module.css';
import { tasksOfStatus } from './taskModel';

interface TaskListViewProps {
  project: Project;
  tasks: Task[];
}

/** The tasks of a project as a table, grouped by status. */
export function TaskListView({ project, tasks }: TaskListViewProps) {
  const navigation = useNavigation();

  return (
    <div className={styles.list}>
      {project.statuses.map((status) => {
        const tasksInStatus = tasksOfStatus(tasks, status.id);
        return (
          <section key={status.id} className={styles.group} aria-label={status.name}>
            <header className={styles.groupHeader}>
              <StatusPill status={status} />
              <span className={styles.count}>{tasksInStatus.length}</span>
            </header>

            {tasksInStatus.length === 0 ? (
              <p className={styles.empty}>No tasks.</p>
            ) : (
              <table className={styles.table}>
                <thead>
                  <tr>
                    <th className={styles.referenceColumn}>Task</th>
                    <th>Title</th>
                    <th className={styles.assigneeColumn}>Assignee</th>
                    <th className={styles.narrowColumn}>Due</th>
                    <th className={styles.narrowColumn}>Priority</th>
                    <th className={styles.narrowColumn}>Estimate</th>
                  </tr>
                </thead>
                <tbody>
                  {tasksInStatus.map((task) => (
                    <tr
                      key={task.id}
                      className={styles.row}
                      tabIndex={0}
                      onClick={() => {
                        navigation.openTask(task.id);
                      }}
                      onKeyDown={(event) => {
                        if (event.key === 'Enter') {
                          navigation.openTask(task.id);
                        }
                      }}
                    >
                      <td className={styles.reference}>{taskReference(project, task)}</td>
                      <td className={styles.title}>{task.title}</td>
                      <td>
                        <span className={styles.assignee}>
                          <AssigneeAvatar assignee={task.assignee} />
                          {assigneeLabel(task.assignee)}
                        </span>
                      </td>
                      <td className={classNames(isOverdue(task) && styles.overdue)}>
                        {task.dueOn === null ? '' : formatDate(task.dueOn)}
                      </td>
                      <td>
                        <PriorityFlag priority={task.priority} withLabel />
                      </td>
                      <td>
                        {task.estimateMinutes === null ? '' : formatEstimate(task.estimateMinutes)}
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            )}
          </section>
        );
      })}
    </div>
  );
}
