import type { Assignee, Id, Member, Project, StatusCategory, Task } from '@/shared/api/types';

/**
 * The arithmetic of the view of people: which column a task stands in, what
 * the three figures of a column are, and how its tasks fall into the three
 * short groups. The component only draws what is worked out here.
 */

/** How many tasks finished lately a column shows. */
export const FINISHED_SHOWN = 5;

/** How many tasks that come next a column shows before it offers to show the rest. */
export const NEXT_SHOWN = 5;

/** Who one column of the view is for. */
export type ColumnSubject =
  | { kind: 'manager' }
  | { kind: 'member'; member: Member }
  | { kind: 'nobody' }
  /** Tasks given to somebody who is no longer on the team, so that none is lost. */
  | { kind: 'other' };

/** The three figures at the head of a column. */
export interface Figures {
  /** The tasks that are not finished. */
  open: number;
  /** The tasks that are begun and not finished: in a status that is active or in review. */
  inWork: number;
  finished: number;
}

/** One column of the view: one person and their work. */
export interface PersonColumn {
  /** Stable among the columns, for React and as the place a task is dropped on. */
  key: string;
  subject: ColumnSubject;
  /**
   * Who a task dropped on the column is given to: nobody for the column of
   * tasks given to nobody. Not given for a column nothing can be dropped on.
   */
  receives?: Assignee | null;
  figures: Figures;
  /** The tasks begun and not finished, the one due first at the top. */
  inWork: Task[];
  /** The tasks not begun, the one due first at the top; those without a date last. */
  next: Task[];
  /** The tasks finished most lately, the newest first, at most `FINISHED_SHOWN`. */
  finishedLately: Task[];
}

/** The category of the status a task is in, or nothing for a status that is gone. */
function categoryOf(project: Pick<Project, 'statuses'>, task: Task): StatusCategory | undefined {
  return project.statuses.find((status) => status.id === task.statusId)?.category;
}

/** Whether a task is begun and not finished. */
function isInWork(category: StatusCategory | undefined): boolean {
  return category === 'active' || category === 'review';
}

/** The key of the column of whoever a task is given to. */
export function columnKeyOf(assignee: Assignee | null): string {
  if (assignee === null) {
    return 'nobody';
  }
  return assignee.kind === 'manager' ? 'manager' : `member:${assignee.memberId}`;
}

/**
 * Orders tasks by due date, the earliest first, with those without a date
 * after them; tasks due the same day keep the order of their numbers.
 */
export function byDueDate(first: Task, second: Task): number {
  if (first.dueOn !== second.dueOn) {
    if (first.dueOn === null) {
      return 1;
    }
    if (second.dueOn === null) {
      return -1;
    }
    // Dates in the form year-month-day compare correctly as text.
    return first.dueOn < second.dueOn ? -1 : 1;
  }
  return first.number - second.number;
}

/** Orders finished tasks, the one finished last first. One with no moment of finishing goes last. */
function byFinishing(first: Task, second: Task): number {
  const firstAt = first.completedAt ?? '';
  const secondAt = second.completedAt ?? '';
  if (firstAt !== secondAt) {
    return firstAt < secondAt ? 1 : -1;
  }
  return second.number - first.number;
}

/** The column of one person, from the tasks given to them. */
function columnOf(
  key: string,
  subject: ColumnSubject,
  receives: Assignee | null | undefined,
  tasks: readonly Task[],
  project: Pick<Project, 'statuses'>,
): PersonColumn {
  const inWork: Task[] = [];
  const next: Task[] = [];
  const finished: Task[] = [];
  for (const task of tasks) {
    const category = categoryOf(project, task);
    if (category === 'done') {
      finished.push(task);
    } else if (isInWork(category)) {
      inWork.push(task);
    } else {
      next.push(task);
    }
  }
  return {
    key,
    subject,
    ...(receives === undefined ? {} : { receives }),
    figures: {
      open: inWork.length + next.length,
      inWork: inWork.length,
      finished: finished.length,
    },
    inWork: inWork.sort(byDueDate),
    next: next.sort(byDueDate),
    finishedLately: finished.sort(byFinishing).slice(0, FINISHED_SHOWN),
  };
}

/**
 * The columns of the view: the manager, every member of the team in the
 * order they joined, and the tasks given to nobody, which is always there
 * so that a task can be dropped on it. Tasks given to somebody who is no
 * longer on the team go in a last column of their own, only when there are
 * any; nothing can be dropped there.
 */
export function peopleColumns(
  project: Pick<Project, 'statuses'>,
  tasks: readonly Task[],
  members: readonly Member[],
): PersonColumn[] {
  const subjects: [string, ColumnSubject, Assignee | null][] = [
    ['manager', { kind: 'manager' }, { kind: 'manager' }],
    ...members.map((member): [string, ColumnSubject, Assignee] => [
      `member:${member.id}`,
      { kind: 'member', member },
      { kind: 'member', memberId: member.id },
    ]),
    ['nobody', { kind: 'nobody' }, null],
  ];
  const known = new Set(subjects.map(([key]) => key));
  const tasksByKey = new Map<string, Task[]>();
  for (const task of tasks) {
    const ownKey = columnKeyOf(task.assignee);
    const key = known.has(ownKey) ? ownKey : 'other';
    const list = tasksByKey.get(key);
    if (list) {
      list.push(task);
    } else {
      tasksByKey.set(key, [task]);
    }
  }

  const columns = subjects.map(([key, subject, receives]) =>
    columnOf(key, subject, receives, tasksByKey.get(key) ?? [], project),
  );
  const others = tasksByKey.get('other');
  if (others) {
    columns.push(columnOf('other', { kind: 'other' }, undefined, others, project));
  }
  return columns;
}

/** Whether two ways of giving a task are the same. */
export function isSameAssignee(first: Assignee | null, second: Assignee | null): boolean {
  return columnKeyOf(first) === columnKeyOf(second);
}

/**
 * The tasks as they are once one of them is given to somebody else. The
 * view shows this while the backend answers, so that a dropped task does
 * not go back to where it came from for a moment.
 */
export function withAssignee(
  tasks: readonly Task[],
  taskId: Id,
  assignee: Assignee | null,
): Task[] {
  return tasks.map((task) => (task.id === taskId ? { ...task, assignee } : task));
}

/**
 * Who a task dropped on a column goes to, or nothing when the drop changes
 * nothing: a column that receives nothing, or the column the task is in.
 */
export function dropOutcome(
  task: Pick<Task, 'assignee'>,
  column: Pick<PersonColumn, 'receives'> | undefined,
): { assignee: Assignee | null } | undefined {
  if (column?.receives === undefined) {
    return undefined;
  }
  return isSameAssignee(task.assignee, column.receives) ? undefined : { assignee: column.receives };
}

/** The span a column takes across the view, in pixels. */
export interface Across {
  left: number;
  width: number;
}

/**
 * The column next to a point across the view, to the left or to the right,
 * for a task carried with the keyboard: the nearest column that lies wholly
 * on that side of the point. The column the point is in is never the next.
 */
export function neighbourColumn<Column extends Across>(
  columns: readonly Column[],
  point: number,
  side: 'left' | 'right',
): Column | undefined {
  let nearest: Column | undefined;
  for (const column of columns) {
    const isBeyond = side === 'right' ? column.left >= point : column.left + column.width <= point;
    if (!isBeyond) {
      continue;
    }
    if (
      nearest === undefined ||
      (side === 'right' ? column.left < nearest.left : column.left > nearest.left)
    ) {
      nearest = column;
    }
  }
  return nearest;
}

/** The figures in a sentence, for screen readers and the pointer: "3 open, 1 in work, 2 finished". */
export function figuresInWords({ open, inWork, finished }: Figures): string {
  if (open + finished === 0) {
    return 'No tasks';
  }
  return `${open.toString()} open, ${inWork.toString()} in work, ${finished.toString()} finished`;
}
