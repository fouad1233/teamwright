import { useDraggable } from '@dnd-kit/core';
import { ArrowRight, Ellipsis } from 'lucide-react';
import type { ReactNode } from 'react';

import type { Id, Project, Sprint, Task } from '@/shared/api/types';
import { classNames } from '@/shared/lib/classNames';
import { taskReference } from '@/shared/lib/format';
import { IconButton } from '@/shared/ui/Button';
import { ActionMenu, type Action } from '@/shared/ui/Menu';

import { StatusLabel } from '../tasks/TaskBadges';
import { statusOf } from '../tasks/taskModel';
import { AssigneeAvatar, AssigneeName } from '../team/Assignee';
import styles from './BacklogRow.module.css';
import { PointsEditor } from './PointsEditor';

interface BacklogRowProps {
  project: Project;
  task: Task;
  /** The sprints the task can be moved to: the planned ones and the active one. */
  destinations: readonly Sprint[];
  onOpen: (taskId: Id) => void;
  onPlan: (taskId: Id, sprintId: Id | null) => void;
  onSetPoints: (taskId: Id, points: number | null) => Promise<void>;
}

/**
 * One task in the backlog or in a sprint: its reference, title, who it is
 * given to, its points and its status. The row can be dragged into another
 * sprint, and its menu moves it there without dragging.
 */
export function BacklogRow({
  project,
  task,
  destinations,
  onOpen,
  onPlan,
  onSetPoints,
}: BacklogRowProps) {
  const { setNodeRef, attributes, listeners, isDragging } = useDraggable({ id: task.id });
  const reference = taskReference(project, task);

  const moves: Action[] = [
    ...destinations
      .filter((sprint) => sprint.id !== task.sprintId)
      .map((sprint) => ({
        label: `Move to ${sprint.name}`,
        icon: <ArrowRight />,
        onSelect: () => {
          onPlan(task.id, sprint.id);
        },
      })),
    ...(task.sprintId === null
      ? []
      : [
          {
            label: 'Move to the backlog',
            icon: <ArrowRight />,
            onSelect: () => {
              onPlan(task.id, null);
            },
          },
        ]),
  ];

  return (
    <li
      ref={setNodeRef}
      className={classNames(styles.row, isDragging && styles.lifted)}
      {...attributes}
      {...listeners}
      // The keyboard opens the task and reaches the menu; moving is done
      // from the menu, so the row does not take the keys of dragging.
      tabIndex={0}
      role="button"
      aria-roledescription="task"
      aria-label={`${reference} ${task.title}`}
      onClick={() => {
        onOpen(task.id);
      }}
      onKeyDown={(event) => {
        if (event.key === 'Enter') {
          onOpen(task.id);
        }
      }}
    >
      <TaskRowFace project={project} task={task}>
        <span className={styles.points}>
          <PointsEditor
            key={`${task.id}:${String(task.points)}`}
            look="row"
            label={`Points of ${reference}`}
            points={task.points}
            onChange={(points) => onSetPoints(task.id, points)}
          />
        </span>
      </TaskRowFace>
      <span
        className={styles.menu}
        onPointerDown={(event) => {
          event.stopPropagation();
        }}
        onClick={(event) => {
          event.stopPropagation();
        }}
        onKeyDown={(event) => {
          event.stopPropagation();
        }}
      >
        {moves.length > 0 && (
          <ActionMenu label={`Move ${reference}`} actions={moves}>
            <IconButton label="Move to">
              <Ellipsis />
            </IconButton>
          </ActionMenu>
        )}
      </span>
    </li>
  );
}

interface TaskRowFaceProps {
  project: Project;
  task: Task;
  /** What stands in the column of the points. */
  children: ReactNode;
}

/** The cells of a row that only show the task. The copy under the pointer shows them too. */
export function TaskRowFace({ project, task, children }: TaskRowFaceProps) {
  const status = statusOf(project, task);
  return (
    <>
      <span className={styles.reference}>{taskReference(project, task)}</span>
      <span className={styles.title}>{task.title}</span>
      <span className={styles.assignee}>
        <AssigneeAvatar assignee={task.assignee} />
        <span className={styles.assigneeName}>
          <AssigneeName assignee={task.assignee} />
        </span>
      </span>
      {children}
      <span className={styles.status}>{status && <StatusLabel status={status} />}</span>
    </>
  );
}

/** The row as it follows the pointer while it is dragged. */
export function DraggedRow({ project, task }: { project: Project; task: Task }) {
  return (
    <div className={classNames(styles.row, styles.dragged)}>
      <TaskRowFace project={project} task={task}>
        <span className={classNames(styles.points, styles.pointsText)}>
          {task.points === null ? '–' : task.points.toString()}
        </span>
      </TaskRowFace>
      <span className={styles.menu} />
    </div>
  );
}
