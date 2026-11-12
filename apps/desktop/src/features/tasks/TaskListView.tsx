import { useMemo, type CSSProperties } from 'react';

import { useNavigation } from '@/app/navigation';
import type { Project, Task } from '@/shared/api/types';
import { classNames } from '@/shared/lib/classNames';
import { formatDate, formatEstimate, isOverdue, taskReference } from '@/shared/lib/format';

import { AssigneeAvatar, AssigneeName } from '../team/Assignee';
import { SubtaskFold } from './SubtaskFold';
import { PriorityFlag, StatusLabel, StatusMark } from './TaskBadges';
import styles from './TaskListView.module.css';
import { useSubtaskFolds } from './subtaskFolds';
import { indexSubtasks, mainTasksOfStatus, outlineRows, statusOf } from './taskModel';

interface TaskListViewProps {
  project: Project;
  tasks: Task[];
}

/** The number of columns of the table, for rows that span all of them. */
const COLUMN_COUNT = 6;

/**
 * The tasks of a project as one table, with a heading row for every status.
 * A subtask has no row of its own under a status: it follows its parent when
 * the parent is unfolded, set in and with the mark of its own status.
 */
export function TaskListView({ project, tasks }: TaskListViewProps) {
  const navigation = useNavigation();
  const folds = useSubtaskFolds(project.id);
  const subtasksByParent = useMemo(() => indexSubtasks(tasks), [tasks]);

  return (
    <div className={styles.list}>
      <table className={styles.table}>
        <thead>
          <tr>
            <th className={classNames('label', styles.referenceColumn)}>Task</th>
            <th className="label">Title</th>
            <th className={classNames('label', styles.assigneeColumn)}>Assignee</th>
            <th className={classNames('label', styles.narrowColumn)}>Due</th>
            <th className={classNames('label', styles.narrowColumn)}>Priority</th>
            <th className={classNames('label', styles.narrowColumn)}>Estimate</th>
          </tr>
        </thead>

        {project.statuses.map((status) => {
          const tasksInStatus = mainTasksOfStatus(tasks, status.id);
          const rows = outlineRows(tasksInStatus, subtasksByParent, folds.isOpen);
          return (
            <tbody key={status.id}>
              <tr className={styles.group}>
                <th colSpan={COLUMN_COUNT} scope="rowgroup">
                  <span className={styles.groupName}>
                    <StatusLabel status={status} />
                    <span className={styles.count}>{tasksInStatus.length}</span>
                  </span>
                </th>
              </tr>

              {rows.map(({ task, depth, subtasks, isOpen }) => {
                const ownStatus = depth > 0 ? statusOf(project, task) : undefined;
                return (
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
                    <td className={styles.title}>
                      <span
                        className={styles.titleLine}
                        style={{ '--depth': depth } as CSSProperties}
                      >
                        {ownStatus && (
                          <span role="img" aria-label={ownStatus.name} title={ownStatus.name}>
                            <StatusMark status={ownStatus} />
                          </span>
                        )}
                        <span className={styles.titleText}>{task.title}</span>
                        {subtasks.length > 0 && (
                          <SubtaskFold
                            className={styles.fold}
                            project={project}
                            subtasks={subtasks}
                            isOpen={isOpen}
                            onToggle={() => {
                              folds.toggle(task.id);
                            }}
                          />
                        )}
                      </span>
                    </td>
                    <td>
                      <span className={styles.assignee}>
                        <AssigneeAvatar assignee={task.assignee} />
                        <AssigneeName assignee={task.assignee} />
                      </span>
                    </td>
                    <td className={classNames(styles.number, isOverdue(task) && styles.overdue)}>
                      {task.dueOn === null ? '' : formatDate(task.dueOn)}
                    </td>
                    <td>
                      {task.priority !== 'normal' && (
                        <PriorityFlag priority={task.priority} withLabel />
                      )}
                    </td>
                    <td className={styles.number}>
                      {task.estimateMinutes === null ? '' : formatEstimate(task.estimateMinutes)}
                    </td>
                  </tr>
                );
              })}
            </tbody>
          );
        })}
      </table>
    </div>
  );
}
