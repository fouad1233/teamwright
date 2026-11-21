import { ChevronDown } from 'lucide-react';
import { useMemo, useState, type CSSProperties, type ReactNode } from 'react';

import { useNavigation } from '@/app/navigation';
import type { Id, Project, Task, View } from '@/shared/api/types';
import { classNames } from '@/shared/lib/classNames';
import { formatDate, formatEstimate, isOverdue, taskReference } from '@/shared/lib/format';

import { ChecklistProgressMark } from '../checklists/ChecklistProgress';
import { TagChips } from '../tags/TagChip';
import { tagsOf } from '../tags/tagModel';
import { useTags } from '../tags/tagQueries';
import { AssigneeAvatar, AssigneeName } from '../team/Assignee';
import { GroupLabel } from '../views/GroupLabel';
import { groupId } from '../views/groupModel';
import { useGroupNames, useTaskGroups } from '../views/groupQueries';
import { SubtaskFold } from './SubtaskFold';
import { PriorityFlag, StatusLabel, StatusMark } from './TaskBadges';
import styles from './TaskListView.module.css';
import { useSubtaskFolds } from './subtaskFolds';
import { indexSubtasks, mainTasksOfStatus, outlineRows, statusOf } from './taskModel';

interface TaskListViewProps {
  project: Project;
  tasks: Task[];
  /** The view being drawn. Without one, or grouped by status, the list has a section for every status. */
  view?: View;
}

/** The number of columns of the table, for rows that span all of them. */
const COLUMN_COUNT = 6;

/** One headed section of the list: what heads it, and its tasks that are not subtasks. */
interface Section {
  id: string;
  label: ReactNode;
  tasks: Task[];
}

/**
 * The tasks of a project as one table in headed sections that fold. The
 * sections are one for every status, or, for a view grouped by something
 * else, one for every group that has a task in it. A subtask has no row of
 * its own in a section: it follows its parent when the parent is unfolded,
 * set in and with the mark of its own status.
 */
export function TaskListView({ project, tasks, view }: TaskListViewProps) {
  return view !== undefined && view.grouping !== 'status' ? (
    <GroupedList key={`${view.id}:${view.grouping}`} project={project} tasks={tasks} view={view} />
  ) : (
    <StatusList project={project} tasks={tasks} />
  );
}

/** The list with a section for every status, whatever it holds. */
function StatusList({ project, tasks }: { project: Project; tasks: Task[] }) {
  const sections = project.statuses.map((status) => ({
    id: status.id,
    label: <StatusLabel status={status} />,
    tasks: mainTasksOfStatus(tasks, status.id),
  }));
  return <ListTable project={project} tasks={tasks} sections={sections} showsStatus={false} />;
}

/**
 * The list with a section for every group of the view, in the order the
 * backend gives them, leaving out the groups with nothing in them. A task
 * with several tags is in a section for each of its tags.
 */
function GroupedList({ project, tasks, view }: { project: Project; tasks: Task[]; view: View }) {
  const names = useGroupNames(project);
  const groups = useTaskGroups(view).data;
  const sections = useMemo(() => {
    const byId = new Map(tasks.map((task) => [task.id, task]));
    return (groups ?? []).map((group) => ({
      id: groupId(group.key),
      label: <GroupLabel groupKey={group.key} names={names} />,
      tasks: group.taskIds.flatMap((taskId) => byId.get(taskId) ?? []),
    }));
  }, [groups, tasks, names]);
  return (
    <ListTable
      project={project}
      tasks={tasks}
      sections={sections.filter((section) => section.tasks.length > 0)}
      showsStatus
    />
  );
}

interface ListTableProps {
  project: Project;
  tasks: Task[];
  sections: readonly Section[];
  /** Whether every row says its own status: the sections no longer do. */
  showsStatus: boolean;
}

function ListTable({ project, tasks, sections, showsStatus }: ListTableProps) {
  const navigation = useNavigation();
  const folds = useSubtaskFolds(project.id);
  const subtasksByParent = useMemo(() => indexSubtasks(tasks), [tasks]);
  const tags = useTags(project.id).data ?? [];
  const [foldedSections, setFoldedSections] = useState<ReadonlySet<Id>>(new Set());

  function toggleSection(sectionId: string) {
    setFoldedSections((current) => {
      const next = new Set(current);
      if (!next.delete(sectionId)) {
        next.add(sectionId);
      }
      return next;
    });
  }

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

        {sections.map((section) => {
          const isFolded = foldedSections.has(section.id);
          const rows = isFolded ? [] : outlineRows(section.tasks, subtasksByParent, folds.isOpen);
          return (
            <tbody key={section.id}>
              <tr className={styles.group}>
                <th colSpan={COLUMN_COUNT} scope="rowgroup">
                  <button
                    type="button"
                    className={styles.groupToggle}
                    aria-expanded={!isFolded}
                    onClick={() => {
                      toggleSection(section.id);
                    }}
                  >
                    <ChevronDown className={styles.chevron} data-folded={isFolded} aria-hidden />
                    <span className={styles.groupName}>
                      {section.label}
                      <span className={styles.count}>{section.tasks.length}</span>
                    </span>
                  </button>
                </th>
              </tr>

              {rows.map(({ task, depth, subtasks, isOpen }) => {
                const ownStatus = depth > 0 || showsStatus ? statusOf(project, task) : undefined;
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
                        <TagChips className={styles.tags} tags={tagsOf(task, tags)} most={3} />
                        <ChecklistProgressMark task={task} />
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
