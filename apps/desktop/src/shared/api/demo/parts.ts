import { BackendError } from '../backend';
import type { Id, Task } from '../types';

/**
 * Which task is a part of which when tasks are regrouped, as the domain of
 * the real application says it (`parts.rs`): by hand on the board, or by a
 * proposal of the Advisor that moves tasks that exist. No task is a part of
 * itself, directly or further down; and because a task with parts starts
 * only once its parts are finished, a part may not wait for the task it is
 * a part of, directly or through other tasks.
 */

/** A task a plan names: one the project has, or a new one by its place from 0. */
export type TaskInPlan = { kind: 'existing'; id: Id } | { kind: 'proposed'; place: number };

/** A new task of a regrouping: what it is a part of, and the new tasks it waits for. */
export interface NewPart {
  partOf: TaskInPlan | null;
  waitsFor: readonly number[];
}

/** A task that exists, moved under another task or made a task of its own. */
export interface PartMove {
  taskId: Id;
  partOf: TaskInPlan | null;
}

/** What breaks the rules of parts in a regrouping. */
export type RegroupingProblem =
  | { kind: 'notThere' }
  | { kind: 'movedTwice' }
  | { kind: 'partOfItsOwnPart'; task: TaskInPlan; whole: TaskInPlan }
  | { kind: 'partWaitsForWhole'; task: TaskInPlan; whole: TaskInPlan };

/** One key for a task of a plan, to keep tasks of both kinds in one map. */
function keyOf(task: TaskInPlan): string {
  return task.kind === 'existing' ? `e:${task.id}` : `p:${task.place.toString()}`;
}

/**
 * Checks a regrouping of the tasks of a project: new tasks, which may be
 * parts of tasks that exist or of each other, and moves of tasks that
 * exist, each with its own parts. Only what the regrouping changes is
 * looked at. Answers with the first problem, or nothing.
 */
export function checkRegrouping(
  tasks: readonly Task[],
  newTasks: readonly NewPart[],
  moves: readonly PartMove[],
): RegroupingProblem | null {
  const byId = new Map(tasks.map((task) => [task.id, task]));
  const isNamed = (whole: TaskInPlan | null) =>
    whole === null ||
    (whole.kind === 'existing' ? byId.has(whole.id) : whole.place < newTasks.length);
  const moved = new Map<Id, TaskInPlan | null>();
  for (const move of moves) {
    if (!byId.has(move.taskId) || !isNamed(move.partOf)) {
      return { kind: 'notThere' };
    }
    if (moved.has(move.taskId)) {
      return { kind: 'movedTwice' };
    }
    moved.set(move.taskId, move.partOf);
  }
  if (!newTasks.every((task) => isNamed(task.partOf))) {
    return { kind: 'notThere' };
  }

  const wholeOf = (task: TaskInPlan): TaskInPlan | null => {
    if (task.kind === 'proposed') {
      return newTasks[task.place]?.partOf ?? null;
    }
    if (moved.has(task.id)) {
      return moved.get(task.id) ?? null;
    }
    const parentId = byId.get(task.id)?.parentId ?? null;
    return parentId === null ? null : { kind: 'existing', id: parentId };
  };
  // The tasks that get a new whole: the moved ones and the new parts.
  const regrouped: { task: TaskInPlan; whole: TaskInPlan }[] = [];
  for (const move of moves) {
    if (move.partOf !== null) {
      regrouped.push({ task: { kind: 'existing', id: move.taskId }, whole: move.partOf });
    }
  }
  newTasks.forEach((task, place) => {
    if (task.partOf !== null) {
      regrouped.push({ task: { kind: 'proposed', place }, whole: task.partOf });
    }
  });

  for (const { task, whole } of regrouped) {
    const seen = new Set<string>();
    for (let above: TaskInPlan | null = whole; above !== null; above = wholeOf(above)) {
      if (keyOf(above) === keyOf(task)) {
        return { kind: 'partOfItsOwnPart', task, whole };
      }
      if (seen.has(keyOf(above))) {
        break;
      }
      seen.add(keyOf(above));
    }
  }

  // What waits for what as the board stays, with each whole waiting for
  // its parts that are not moved; then each new link of a whole to a part,
  // one at a time, none of which may close a circle.
  const waits = new Map<string, Set<string>>();
  const addWait = (task: string, blocker: string) => {
    const blockers = waits.get(task) ?? new Set<string>();
    blockers.add(blocker);
    waits.set(task, blockers);
  };
  for (const task of tasks) {
    for (const blockerId of task.blockedBy) {
      addWait(`e:${task.id}`, `e:${blockerId}`);
    }
    if (task.parentId !== null && !moved.has(task.id)) {
      addWait(`e:${task.parentId}`, `e:${task.id}`);
    }
  }
  newTasks.forEach((task, place) => {
    for (const earlier of task.waitsFor) {
      addWait(`p:${place.toString()}`, `p:${earlier.toString()}`);
    }
  });
  const waitsOn = (waiting: string, target: string): boolean => {
    const seen = new Set<string>();
    const toVisit = [waiting];
    for (let current = toVisit.pop(); current !== undefined; current = toVisit.pop()) {
      if (seen.has(current)) {
        continue;
      }
      seen.add(current);
      for (const blocker of waits.get(current) ?? []) {
        if (blocker === target) {
          return true;
        }
        toVisit.push(blocker);
      }
    }
    return false;
  };
  for (const { task, whole } of regrouped) {
    if (waitsOn(keyOf(task), keyOf(whole))) {
      return { kind: 'partWaitsForWhole', task, whole };
    }
    addWait(keyOf(whole), keyOf(task));
  }
  return null;
}

/**
 * The refusal of a regrouping that breaks a rule of parts, in the words of
 * the real application, with the tasks named by `name`.
 */
export function regroupingError(
  problem: RegroupingProblem,
  name: (task: TaskInPlan) => string,
): BackendError {
  switch (problem.kind) {
    case 'notThere':
      return new BackendError(
        'rule_broken',
        'a task this proposal moves or adds a part to is no longer in the project; take that out of the proposal, or ask the Advisor to propose again',
      );
    case 'movedTwice':
      return new BackendError('rule_broken', 'the proposal moves the same task twice');
    case 'partOfItsOwnPart':
      return keyOf(problem.task) === keyOf(problem.whole)
        ? new BackendError('part_in_circle', `${name(problem.task)} cannot be a part of itself`)
        : new BackendError(
            'part_in_circle',
            `${name(problem.task)} cannot be a part of ${name(problem.whole)}, which is a part of ${name(problem.task)} already`,
          );
    case 'partWaitsForWhole':
      return new BackendError(
        'part_in_circle',
        `${name(problem.task)} cannot be a part of ${name(problem.whole)}, because ${name(problem.task)} or one of its parts waits for ${name(problem.whole)}, and a task with parts starts only once its parts are finished`,
      );
  }
}

/**
 * Puts each of the regrouped tasks into the area and the sprint of the top
 * of its chain of wholes, as a subtask goes where the task it is a part of
 * goes. Only what differs is changed.
 */
export function followTops(tasks: readonly Task[], regrouped: ReadonlySet<Id>, now: string): void {
  const byId = new Map(tasks.map((task) => [task.id, task]));
  const places = new Map(tasks.map((task) => [task.id, [task.areaId, task.sprintId] as const]));
  for (const task of tasks) {
    if (!regrouped.has(task.id)) {
      continue;
    }
    let top = task;
    for (let step = 0; step < tasks.length && top.parentId !== null; step += 1) {
      const whole = byId.get(top.parentId);
      if (!whole) {
        break;
      }
      top = whole;
    }
    const [areaId, sprintId] = places.get(top.id) ?? [task.areaId, task.sprintId];
    if (task.areaId !== areaId || task.sprintId !== sprintId) {
      task.areaId = areaId;
      task.sprintId = sprintId;
      task.updatedAt = now;
    }
  }
}
