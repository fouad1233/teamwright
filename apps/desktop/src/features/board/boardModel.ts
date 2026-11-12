import type { Id, Status, Task } from '@/shared/api/types';

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
