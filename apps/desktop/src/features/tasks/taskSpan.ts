import type { Task } from '@/shared/api/types';
import { dayNumberOf, type DayNumber } from '@/shared/lib/days';

/**
 * The days a task takes up, for the views that draw tasks against time: the
 * Gantt chart and the calendar.
 */

/** The days a task takes up. */
export interface Span {
  firstDay: DayNumber;
  lastDay: DayNumber;
  /**
   * `planned` for a task with a start and a due date, `open` for a task
   * that only has a start, and `deadline` for one that only has a due date.
   */
  kind: 'planned' | 'open' | 'deadline';
}

/** The days a task takes up, or nothing for a task without any date. */
export function spanOf(task: Pick<Task, 'startOn' | 'dueOn'>): Span | undefined {
  if (task.startOn !== null && task.dueOn !== null) {
    return {
      firstDay: dayNumberOf(task.startOn),
      lastDay: dayNumberOf(task.dueOn),
      kind: 'planned',
    };
  }
  if (task.startOn !== null) {
    const day = dayNumberOf(task.startOn);
    return { firstDay: day, lastDay: day, kind: 'open' };
  }
  if (task.dueOn !== null) {
    const day = dayNumberOf(task.dueOn);
    return { firstDay: day, lastDay: day, kind: 'deadline' };
  }
  return undefined;
}
