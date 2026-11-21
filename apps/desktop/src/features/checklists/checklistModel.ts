import type { ChecklistItem, Id, Task } from '@/shared/api/types';

/** How far the checklist of a task has come: how many items are done, of how many. */
export interface ChecklistProgress {
  done: number;
  total: number;
}

/** The progress of the checklist of a task, or nothing for a task without one. */
export function progressOf(task: Pick<Task, 'checklist'>): ChecklistProgress | null {
  if (task.checklist.length === 0) {
    return null;
  }
  const done = task.checklist.filter((item) => item.done).length;
  return { done, total: task.checklist.length };
}

/** The progress in words, for a tooltip and for screen readers: "3 of 5 items done". */
export function progressInWords({ done, total }: ChecklistProgress): string {
  return `${done.toString()} of ${total.toString()} ${total === 1 ? 'item' : 'items'} done`;
}

/** Whether every item is done. */
export function isComplete({ done, total }: ChecklistProgress): boolean {
  return total > 0 && done === total;
}

/** The checklist as it is once one item has moved to a place. Index 0 is the first. */
export function withMovedItem(
  checklist: readonly ChecklistItem[],
  itemId: Id,
  index: number,
): ChecklistItem[] {
  const moving = checklist.find((item) => item.id === itemId);
  if (!moving) {
    return [...checklist];
  }
  const others = checklist.filter((item) => item.id !== itemId);
  others.splice(Math.min(Math.max(index, 0), others.length), 0, moving);
  return others;
}
