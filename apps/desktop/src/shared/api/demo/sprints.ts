import { addDays } from '@/shared/lib/days';
import { calendarDateOf } from '@/shared/lib/format';

import { BackendError } from '../backend';
import type {
  CalendarDate,
  Id,
  MovedOn,
  ScopeChange,
  ScopeTask,
  Sprint,
  SprintDay,
  SprintReport,
  SprintSummary,
  SprintUnit,
  StatusCategory,
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

/** Everything the demo knows of the past of one task: its versions, and when it was deleted. */
export interface TaskPast {
  versions: readonly Task[];
  deletedAt: string | null;
}

/** A task as a sprint held it at some moment. */
interface Held {
  past: TaskPast;
  task: ScopeTask;
  isFinished: boolean;
}

/**
 * The course of a sprint, read off the versions of the tasks of its
 * project, as the domain crate works it out (`sprint_course.rs`).
 *
 * What the sprint held at the end of a day is the last version of each task
 * stored by then; the tasks it was started with count from its first day,
 * and a task counts as finished only from the day it was finished. Nothing
 * stored after the moment the sprint was completed counts, and the version
 * that took a task out of the sprint as it was completed does not count
 * either: that version says where the task moved on to. Days after `today`,
 * and after the day the sprint was completed, have not come. A planned
 * sprint has not come at all.
 */
export function reportOf(
  sprint: Sprint,
  categoryOf: (statusId: Id) => StatusCategory | undefined,
  pasts: readonly TaskPast[],
  today: CalendarDate,
): SprintReport {
  const isIn = (version: Task) => version.sprintId === sprint.id && version.parentId === null;
  const unit: SprintUnit = pasts.some((past) =>
    past.versions.some((version) => isIn(version) && version.points !== null),
  )
    ? 'points'
    : 'tasks';
  const workOfVersion = (version: Task) => (unit === 'points' ? (version.points ?? 0) : 1);
  const completedAt = sprint.completedAt === null ? null : Date.parse(sprint.completedAt);
  const startedAt = sprint.startedAt === null ? null : Date.parse(sprint.startedAt);
  const beforeCompletion = (moment: number) => completedAt === null || moment <= completedAt;
  const byStart = (moment: number) => startedAt !== null && moment <= startedAt;

  const lastVersion = (past: TaskPast, counts: (moment: number) => boolean) =>
    [...past.versions].reverse().find((version) => {
      const moment = Date.parse(version.updatedAt);
      return counts(moment) && !(moment === completedAt && !isIn(version));
    });

  const held = (
    counts: (moment: number) => boolean,
    finishedBy: (moment: number) => boolean,
  ): Held[] =>
    pasts
      .flatMap((past): Held[] => {
        if (past.deletedAt !== null && counts(Date.parse(past.deletedAt))) {
          return [];
        }
        const version = lastVersion(past, counts);
        if (!version || !isIn(version)) {
          return [];
        }
        const status = lastVersion(past, finishedBy)?.statusId;
        return [
          {
            past,
            task: {
              taskId: version.id,
              number: version.number,
              title: version.title,
              work: workOfVersion(version),
              hasNoPoints: unit === 'points' && version.points === null,
            },
            isFinished: status !== undefined && categoryOf(status) === 'done',
          },
        ];
      })
      .sort((first, second) => first.task.number - second.task.number);
  const workOf = (tasks: readonly Held[]) => tasks.reduce((sum, each) => sum + each.task.work, 0);

  const heldLast = held(beforeCompletion, beforeCompletion);
  const heldFirst = startedAt === null ? heldLast : held(byStart, byStart);

  const lastDayCome =
    sprint.state === 'planned'
      ? null
      : sprint.completedAt === null
        ? today
        : minimumOf(today, dayOfMoment(sprint.completedAt));

  const days: SprintDay[] = [];
  const changes: ScopeChange[] = [];
  if (sprint.startOn !== null && sprint.endOn !== null) {
    let before = heldFirst;
    for (let date = sprint.startOn; date <= sprint.endOn; date = addDays(date, 1)) {
      if (lastDayCome === null || date > lastDayCome) {
        days.push({ date, remaining: null, scope: null });
        continue;
      }
      const byEndOfDay = (moment: number) =>
        beforeCompletion(moment) && dayOfMoment(new Date(moment).toISOString()) <= date;
      const now = held(
        (moment) => byEndOfDay(moment) || (beforeCompletion(moment) && byStart(moment)),
        byEndOfDay,
      );
      days.push({
        date,
        remaining: workOf(now.filter((each) => !each.isFinished)),
        scope: workOf(now),
      });
      const change = changeBetween(before, now, date);
      if (change) {
        changes.push(change);
      }
      before = now;
    }
  }

  const movedOn: MovedOn[] =
    completedAt === null
      ? []
      : heldLast
          .filter((each) => !each.isFinished)
          .flatMap((each): MovedOn[] => {
            const atCompletion = each.past.versions.find(
              (version) => Date.parse(version.updatedAt) === completedAt,
            );
            return atCompletion && atCompletion.sprintId !== sprint.id
              ? [{ task: each.task, toSprintId: atCompletion.sprintId }]
              : [];
          });

  return {
    sprintId: sprint.id,
    unit,
    total: workOf(heldLast),
    committed: workOf(heldFirst),
    days,
    changes,
    movedOn,
  };
}

/** What changed from what a sprint held `before` to what it holds `after`, or nothing. */
function changeBetween(
  before: readonly Held[],
  after: readonly Held[],
  date: CalendarDate,
): ScopeChange | null {
  const workIn = (list: readonly Held[], taskId: Id) =>
    list.find((each) => each.task.taskId === taskId)?.task.work;
  const joined = after
    .filter((each) => workIn(before, each.task.taskId) === undefined)
    .map((each) => each.task);
  const left = before
    .filter((each) => workIn(after, each.task.taskId) === undefined)
    .map((each) => each.task);
  const reestimated = after.reduce((sum, each) => {
    const earlier = workIn(before, each.task.taskId);
    return earlier === undefined ? sum : sum + each.task.work - earlier;
  }, 0);
  return joined.length > 0 || left.length > 0 || reestimated !== 0
    ? { date, joined, left, reestimated }
    : null;
}

/** The calendar date of a moment where the program runs. */
export function dayOfMoment(moment: string): CalendarDate {
  return calendarDateOf(new Date(moment));
}

function minimumOf(first: CalendarDate, second: CalendarDate): CalendarDate {
  return first < second ? first : second;
}
