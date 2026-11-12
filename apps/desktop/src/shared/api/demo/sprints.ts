import { addDays } from '@/shared/lib/days';
import { calendarDateOf } from '@/shared/lib/format';

import { BackendError } from '../backend';
import type {
  CalendarDate,
  Sprint,
  SprintDay,
  SprintReport,
  SprintSummary,
  SprintUnit,
  Task,
} from '../types';

/**
 * The rules about sprints that concern one sprint and its tasks, for the
 * demo backend. The real application has them in its domain crate.
 */

/** The most characters the name of a sprint can have. */
const SPRINT_NAME_MAX_LENGTH = 60;

/** The most story points a task can have. */
const POINTS_MAX = 999;

/** How the sprints of a project are listed: active, planned, completed. */
const PLACE_OF_STATE: Record<Sprint['state'], number> = { active: 0, planned: 1, completed: 2 };

/**
 * Tidies the name of a sprint: trimmed and on one line. An empty name is
 * replaced with "Sprint N", where N is the number the sprint gets among the
 * sprints of its project, or refused when there is no such number.
 */
export function tidySprintName(raw: string, numberIfEmpty?: number): string {
  const name = raw.trim().split(/\s+/).join(' ');
  if (name === '') {
    if (numberIfEmpty === undefined) {
      throw new BackendError('rule_broken', 'the sprint name must not be empty');
    }
    return `Sprint ${numberIfEmpty.toString()}`;
  }
  const length = [...name].length;
  if (length > SPRINT_NAME_MAX_LENGTH) {
    throw new BackendError(
      'rule_broken',
      `the sprint name can have at most ${SPRINT_NAME_MAX_LENGTH.toString()} characters, but this one has ${length.toString()}`,
    );
  }
  return name;
}

/** Refuses a start date that lies after the end date. */
export function requireStartNotAfterEnd(
  startOn: CalendarDate | null,
  endOn: CalendarDate | null,
): void {
  // Dates in the form year-month-day compare correctly as text.
  if (startOn !== null && endOn !== null && startOn > endOn) {
    throw new BackendError(
      'rule_broken',
      `the start date ${startOn} is after the end date ${endOn}`,
    );
  }
}

/** Refuses story points that are not a whole number from 0 to 999. */
export function requirePoints(points: number | null): void {
  if (points !== null && (!Number.isInteger(points) || points < 0 || points > POINTS_MAX)) {
    throw new BackendError(
      'rule_broken',
      `story points are a whole number from 0 to ${POINTS_MAX.toString()}`,
    );
  }
}

/** Sorts sprints the way they are listed: the active one, the planned ones, the completed ones. */
export function asListed(first: Sprint, second: Sprint): number {
  const byState = PLACE_OF_STATE[first.state] - PLACE_OF_STATE[second.state];
  if (byState !== 0) {
    return byState;
  }
  if (first.state === 'completed') {
    // The sprint completed last comes first.
    return (second.completedAt ?? '').localeCompare(first.completedAt ?? '');
  }
  return first.createdAt.localeCompare(second.createdAt);
}

/** A task of a sprint as its work is counted: what it weighs, and whether it is finished. */
export interface CountedTask {
  points: Task['points'];
  completedAt: Task['completedAt'];
  isFinished: boolean;
}

/** What the work of some tasks is counted in: points if any of them has points. */
export function unitOf(tasks: readonly CountedTask[]): SprintUnit {
  return tasks.some((task) => task.points !== null) ? 'points' : 'tasks';
}

/** The work of one task in a unit. A task without points weighs nothing in points. */
function workOf(task: CountedTask, unit: SprintUnit): number {
  return unit === 'points' ? (task.points ?? 0) : 1;
}

/**
 * What a sprint took on and finished, counted over the tasks that are in it
 * when it is completed. Subtasks are not counted: they are part of their task.
 */
export function summaryOf(tasks: readonly CountedTask[]): SprintSummary {
  const unit = unitOf(tasks);
  const sum = (counted: readonly CountedTask[]) =>
    counted.reduce((total, task) => total + workOf(task, unit), 0);
  return {
    unit,
    committed: sum(tasks),
    completed: sum(tasks.filter((task) => task.isFinished)),
  };
}

/**
 * The course of a sprint: for each day from its start to its end, the work
 * that remained at the end of the day.
 *
 * A task that was finished counts as done from the day it was finished, or
 * from the first day if that was earlier. Days after `today`, and after the
 * day the sprint was completed, have not come for the sprint: they have no
 * value. A planned sprint has not come at all.
 */
export function reportOf(
  sprint: Sprint,
  tasks: readonly CountedTask[],
  today: CalendarDate,
): SprintReport {
  const unit = unitOf(tasks);
  const total = tasks.reduce((sum, task) => sum + workOf(task, unit), 0);
  const finished = tasks.flatMap((task) =>
    task.isFinished && task.completedAt !== null
      ? [{ day: dayOfMoment(task.completedAt), work: workOf(task, unit) }]
      : [],
  );

  const lastDayCome =
    sprint.state === 'planned'
      ? null
      : sprint.completedAt === null
        ? today
        : minimumOf(today, dayOfMoment(sprint.completedAt));

  const days: SprintDay[] = [];
  if (sprint.startOn !== null && sprint.endOn !== null) {
    for (let day = sprint.startOn; day <= sprint.endOn; day = addDays(day, 1)) {
      const hasCome = lastDayCome !== null && day <= lastDayCome;
      const done = finished
        .filter((task) => task.day <= day)
        .reduce((sum, task) => sum + task.work, 0);
      days.push({ date: day, remaining: hasCome ? total - done : null });
    }
  }
  return { sprintId: sprint.id, unit, total, days };
}

/** The calendar date of a moment where the program runs. */
export function dayOfMoment(moment: string): CalendarDate {
  return calendarDateOf(new Date(moment));
}

function minimumOf(first: CalendarDate, second: CalendarDate): CalendarDate {
  return first < second ? first : second;
}
