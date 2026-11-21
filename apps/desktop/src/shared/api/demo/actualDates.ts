import { dayNumberOf } from '@/shared/lib/days';

import type { CalendarDate, StatusCategory, Task, TaskActual } from '../types';
import { dayOfMoment } from './sprints';

/**
 * What really happened to a task against its plan, as the domain crate
 * reads it off the versions of the task (`actual_dates.rs`): work began
 * when the task first went into a status of a started category, and ended
 * when it reached the category done, if it is there now. Only the category
 * of a status counts, never its name. A status that is gone counts as not
 * started.
 */
export function actualOf(
  versions: readonly Task[],
  categoryOf: (statusId: string) => StatusCategory | undefined,
  today: CalendarDate,
): TaskActual | null {
  const task = versions.at(-1);
  if (!task) {
    return null;
  }
  const started = versions.find((version) => {
    const category = categoryOf(version.statusId);
    return category === 'active' || category === 'review' || category === 'done';
  });
  let wasDone = false;
  let reopened = false;
  for (const version of versions) {
    const isDone = categoryOf(version.statusId) === 'done';
    if (wasDone && !isDone) {
      reopened = true;
    }
    wasDone = isDone;
  }
  const startedAt = started?.updatedAt ?? null;
  const finishedAt =
    categoryOf(task.statusId) === 'done' ? (task.completedAt ?? task.updatedAt) : null;
  const startedOn = startedAt === null ? null : dayOfMoment(startedAt);
  const finishedOn = finishedAt === null ? null : dayOfMoment(finishedAt);

  const daysAfter = (later: CalendarDate, earlier: CalendarDate) => {
    const days = dayNumberOf(later) - dayNumberOf(earlier);
    return days > 0 ? days : null;
  };
  let startedLateDays: number | null = null;
  let notStarted = false;
  if (task.startOn !== null && startedOn !== null) {
    startedLateDays = daysAfter(startedOn, task.startOn);
  } else if (task.startOn !== null && finishedOn === null) {
    startedLateDays = daysAfter(today, task.startOn);
    notStarted = startedLateDays !== null;
  }
  const finishedLateDays =
    task.dueOn !== null && finishedOn !== null ? daysAfter(finishedOn, task.dueOn) : null;
  const runningLateDays =
    task.dueOn !== null && finishedOn === null ? daysAfter(today, task.dueOn) : null;

  return {
    taskId: task.id,
    startedAt,
    finishedAt,
    startedOn,
    finishedOn,
    reopened,
    startedLateDays,
    notStarted,
    finishedLateDays,
    runningLateDays,
  };
}
