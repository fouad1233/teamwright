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
 * `index` counts the other tasks of the status from the top.
 */
export function withMovedTask(
  tasks: readonly Task[],
  taskId: Id,
  statusId: Id,
  index: number,
): Task[] {
  const others = tasksOfStatus(tasks, statusId).filter((task) => task.id !== taskId);
  const position = Math.min(Math.max(index, 0), others.length);
  const rank = rankBetween(others[position - 1]?.rank, others[position]?.rank);
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

/** The status a task is in, if the project still has it. */
export function statusOf(project: Project, task: Task): Status | undefined {
  return project.statuses.find((status) => status.id === task.statusId);
}
