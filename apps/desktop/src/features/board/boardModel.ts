import type { Fill, Id, Status, StatusLoad, Task } from '@/shared/api/types';

import { mainTasksOfStatus } from '../tasks/taskModel';

/**
 * How the cards of a board are arranged: for each status, the identifiers of
 * its tasks from top to bottom. A subtask has no place of its own in an
 * arrangement, because the board shows it under its parent.
 *
 * While a card is dragged the board keeps its own arrangement, so cards can
 * make room for it. These functions change an arrangement without touching
 * the tasks themselves.
 */
export type Arrangement = ReadonlyMap<Id, readonly Id[]>;

/** Where a card is: in which status, and at which position from the top. */
export interface CardPosition {
  statusId: Id;
  index: number;
}

/** The arrangement the tasks have according to their statuses and ranks. */
export function arrangementOf(statuses: readonly Status[], tasks: readonly Task[]): Arrangement {
  return new Map(
    statuses.map((status) => [
      status.id,
      mainTasksOfStatus(tasks, status.id).map((task) => task.id),
    ]),
  );
}

/** Finds a card in an arrangement. */
export function locate(arrangement: Arrangement, taskId: Id): CardPosition | undefined {
  for (const [statusId, taskIds] of arrangement) {
    const index = taskIds.indexOf(taskId);
    if (index !== -1) {
      return { statusId, index };
    }
  }
  return undefined;
}

/**
 * The status a drop target belongs to. A card can be dropped on a status
 * itself (its empty area) or on another card.
 */
export function statusUnder(arrangement: Arrangement, targetId: Id): Id | undefined {
  return arrangement.has(targetId) ? targetId : locate(arrangement, targetId)?.statusId;
}

/**
 * Moves a card onto a drop target and returns the new arrangement.
 *
 * Dropped on another card, the card takes that card's place. Dropped on a
 * status, it goes to the bottom. A target that is not on the board, or the
 * card itself, changes nothing.
 */
export function moveCard(arrangement: Arrangement, taskId: Id, targetId: Id): Arrangement {
  const from = locate(arrangement, taskId);
  const toStatusId = statusUnder(arrangement, targetId);
  if (!from || toStatusId === undefined || targetId === taskId) {
    return arrangement;
  }

  const target = arrangement.get(toStatusId) ?? [];
  const targetIsStatus = targetId === toStatusId;
  // The position is read before the card is taken out. Within one status
  // this makes a card dropped on a lower card land below it, and a card
  // dropped on a higher card land above it, as people expect.
  const toIndex = targetIsStatus ? target.length : target.indexOf(targetId);

  const moved = new Map(arrangement);
  moved.set(
    from.statusId,
    (arrangement.get(from.statusId) ?? []).filter((id) => id !== taskId),
  );
  const destination = [...(moved.get(toStatusId) ?? [])];
  destination.splice(toIndex, 0, taskId);
  moved.set(toStatusId, destination);
  return moved;
}

/**
 * The tasks that have just arrived: those among `tasks` that were not among
 * the `known` ones. A task that only moved, or changed, is not new.
 */
export function newArrivals(known: ReadonlySet<Id>, tasks: readonly Task[]): ReadonlySet<Id> {
  return new Set(tasks.filter((task) => !known.has(task.id)).map((task) => task.id));
}

/**
 * The most marks a gauge of a limit draws, one for each task the limit
 * allows. A larger limit is drawn as one bar.
 */
export const MOST_LIMIT_MARKS = 12;

/** What the head of a lane shows about the limit of its status. */
export interface LaneLimit {
  /** The tasks in the status against its limit, as in `5/4`. */
  figure: string;
  /** The same in words, for a reader and a tooltip. */
  words: string;
  /** How full the status is, as the backend counted it. */
  fill: Fill;
  /**
   * One entry for each task the limit allows, true where a task stands, or
   * nothing when the limit is too large to be drawn mark by mark.
   */
  marks: boolean[] | null;
  /** How much of the gauge is filled, from 0 to 1, for a limit drawn as a bar. */
  share: number;
}

/**
 * What the head of a lane shows about a status with a limit, from the load
 * the backend counted. Nothing for a status without a limit, or before the
 * backend has answered. The count is that of the whole project: a limit is
 * about the work the team has under way, whatever this view shows.
 */
export function laneLimit(load: StatusLoad | undefined): LaneLimit | undefined {
  if (load?.limit == null || load.fill === null) {
    return undefined;
  }
  const { count, limit, fill } = load;
  const tasks = count === 1 ? '1 task' : `${count.toString()} tasks`;
  const over = count - limit;
  const words =
    fill === 'over'
      ? `${tasks} for a limit of ${limit.toString()}: ${over.toString()} over`
      : fill === 'at'
        ? `${tasks}, at the limit of ${limit.toString()}`
        : `${tasks} for a limit of ${limit.toString()}`;
  return {
    figure: `${count.toString()}/${limit.toString()}`,
    words: `${words}, counted in the whole project`,
    fill,
    marks:
      limit <= MOST_LIMIT_MARKS ? Array.from({ length: limit }, (_, place) => place < count) : null,
    share: Math.min(count / limit, 1),
  };
}
