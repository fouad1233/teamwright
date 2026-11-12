import type { Area, Id, Task } from '@/shared/api/types';

/**
 * What the interface works out about the areas of a project without drawing
 * anything: whether they are shown at all, which one is open, which tasks
 * it holds, and how the list looks after a move.
 */

/**
 * Whether a project is cut into areas that are worth showing. A project
 * with a single area looks exactly as one without areas: nothing of them is
 * shown until there are two.
 */
export function showsAreas(areas: readonly Area[]): boolean {
  return areas.length > 1;
}

/**
 * The area that is open: the one asked for, if the project shows areas and
 * still has it. Nothing means the whole project, every area.
 */
export function areaToShow(areas: readonly Area[], areaId: Id | null): Area | undefined {
  if (areaId === null || !showsAreas(areas)) {
    return undefined;
  }
  return areas.find((area) => area.id === areaId);
}

/** The tasks of one area, subtasks included, or all of them when no area is open. */
export function tasksOfArea(tasks: readonly Task[], area: Area | undefined): Task[] {
  return area === undefined ? [...tasks] : tasks.filter((task) => task.areaId === area.id);
}

/** How many tasks each area holds, subtasks included. */
export function taskCountsByArea(tasks: readonly Task[]): ReadonlyMap<Id, number> {
  const counts = new Map<Id, number>();
  for (const task of tasks) {
    counts.set(task.areaId, (counts.get(task.areaId) ?? 0) + 1);
  }
  return counts;
}

/**
 * The name a new area is given in the dialog, ready to be typed over: "New
 * area", or with the first number from 2 on that no area has.
 */
export function nameForNewArea(areas: readonly Area[]): string {
  const taken = new Set(areas.map((area) => area.name.toLowerCase()));
  const base = 'New area';
  if (!taken.has(base.toLowerCase())) {
    return base;
  }
  for (let number = 2; ; number += 1) {
    const candidate = `${base} ${number.toString()}`;
    if (!taken.has(candidate.toLowerCase())) {
      return candidate;
    }
  }
}

/** The areas as they are after one of them has moved to a place. Index 0 is the first. */
export function withMovedArea(areas: readonly Area[], areaId: Id, index: number): Area[] {
  const moving = areas.find((area) => area.id === areaId);
  if (!moving) {
    return [...areas];
  }
  const others = areas.filter((area) => area.id !== areaId);
  others.splice(Math.min(Math.max(index, 0), others.length), 0, moving);
  return others;
}

/**
 * The area the tasks of a removed area are offered to go to first: the one
 * above it, or the one below it if it is the first.
 */
export function likelyAreaForTasksOf(areas: readonly Area[], area: Area): Area | undefined {
  const place = areas.findIndex((other) => other.id === area.id);
  return areas[place - 1] ?? areas[place + 1];
}
