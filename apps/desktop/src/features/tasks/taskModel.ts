import type { Id, Project, Status, Task } from '@/shared/api/types';
import { rankBetween } from '@/shared/lib/rank';

/**
 * Pure functions over lists of tasks. They hold no state and call no
 * backend, which makes them easy to test and safe to use while rendering.
 */

/** Sorts tasks the way the manager arranged them: by rank, then by number. */
export function asArranged(first: Task, second: Task): number {
  return first.rank - second.rank || first.number - second.number;
}

/** The tasks of one status, in the order the manager arranged them. */
export function tasksOfStatus(tasks: readonly Task[], statusId: Id): Task[] {
  return tasks.filter((task) => task.statusId === statusId).sort(asArranged);
}

/** Whether a task is a part of another task. */
export function isSubtask(task: Task): boolean {
  return task.parentId !== null;
}

/**
 * The tasks of one status that are not part of another task, in the order the
 * manager arranged them. These are the tasks a board or a list shows directly
 * under the status. A subtask is shown under its parent instead.
 */
export function mainTasksOfStatus(tasks: readonly Task[], statusId: Id): Task[] {
  return tasksOfStatus(tasks, statusId).filter((task) => !isSubtask(task));
}

/** A copy of the list with one task added, or replaced if it is already in it. */
export function withTask(tasks: readonly Task[], changed: Task): Task[] {
  const exists = tasks.some((task) => task.id === changed.id);
  return exists
    ? tasks.map((task) => (task.id === changed.id ? changed : task))
    : [...tasks, changed];
}

/**
 * A copy of the list in which one task has moved to a status and a position
 * there, the way the backend will move it. The interface shows this at once
 * instead of waiting for the backend's answer.
 *
 * `index` counts, from the top, the other tasks of the status that are listed
 * next to the task: the ones with the same parent. For a task on the board
 * these are the tasks that are not part of another task.
 */
export function withMovedTask(
  tasks: readonly Task[],
  taskId: Id,
  statusId: Id,
  index: number,
): Task[] {
  const moving = tasks.find((task) => task.id === taskId);
  if (!moving) {
    return [...tasks];
  }
  const neighbours = tasksOfStatus(tasks, statusId).filter(
    (task) => task.id !== taskId && task.parentId === moving.parentId,
  );
  const position = Math.min(Math.max(index, 0), neighbours.length);
  const rank = rankBetween(neighbours[position - 1]?.rank, neighbours[position]?.rank);
  return tasks.map((task) => (task.id === taskId ? { ...task, statusId, rank } : task));
}

/** Looks tasks up by their identifier. */
export function indexById(tasks: readonly Task[]): Map<Id, Task> {
  return new Map(tasks.map((task) => [task.id, task]));
}

/** The tasks a task waits for that are not finished yet. */
export function unfinishedBlockers(task: Task, tasksById: ReadonlyMap<Id, Task>): Task[] {
  return task.blockedBy
    .map((blockerId) => tasksById.get(blockerId))
    .filter((blocker): blocker is Task => blocker !== undefined && blocker.completedAt === null);
}

/** The direct subtasks of a task, in the order of their numbers. */
export function subtasksOf(tasks: readonly Task[], parentId: Id): Task[] {
  return tasks
    .filter((task) => task.parentId === parentId)
    .sort((first, second) => first.number - second.number);
}

/**
 * The direct subtasks of every task that has some, by the identifier of the
 * parent and in the order of their numbers. A view that shows many tasks
 * builds this once instead of searching the whole list for every task.
 */
export function indexSubtasks(tasks: readonly Task[]): Map<Id, Task[]> {
  const subtasksByParent = new Map<Id, Task[]>();
  const byNumber = [...tasks].sort((first, second) => first.number - second.number);
  for (const task of byNumber) {
    if (task.parentId === null) {
      continue;
    }
    const siblings = subtasksByParent.get(task.parentId);
    if (siblings) {
      siblings.push(task);
    } else {
      subtasksByParent.set(task.parentId, [task]);
    }
  }
  return subtasksByParent;
}

/** One line of an outline: a task, and how far it is set in under its parents. */
export interface OutlineRow {
  task: Task;
  /** How many tasks the task is nested in: `0` for a task of its own. */
  depth: number;
  /** The direct subtasks of the task. */
  subtasks: readonly Task[];
  /** Whether the subtasks follow the task in the outline. */
  isOpen: boolean;
}

/**
 * The lines an outline shows for some tasks: each task, followed by its
 * subtasks if the task is open, to any depth.
 */
export function outlineRows(
  tasks: readonly Task[],
  subtasksByParent: ReadonlyMap<Id, readonly Task[]>,
  isOpen: (taskId: Id) => boolean,
  depth = 0,
): OutlineRow[] {
  return tasks.flatMap((task) => {
    const subtasks = subtasksByParent.get(task.id) ?? [];
    const row: OutlineRow = {
      task,
      depth,
      subtasks,
      isOpen: subtasks.length > 0 && isOpen(task.id),
    };
    return row.isOpen
      ? [row, ...outlineRows(subtasks, subtasksByParent, isOpen, depth + 1)]
      : [row];
  });
}

/** The status a task is in, if the project still has it. */
export function statusOf(project: Project, task: Task): Status | undefined {
  return project.statuses.find((status) => status.id === task.statusId);
}
