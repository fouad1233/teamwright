import type {
  CalendarDate,
  Id,
  Project,
  Sprint,
  SprintSummary,
  SprintUnit,
  Task,
} from '@/shared/api/types';
import { addDays, dayNumberOf } from '@/shared/lib/days';
import { formatDate } from '@/shared/lib/format';

import { asArranged } from '../tasks/taskModel';

/**
 * Pure functions for the backlog and its sprints. They decide nothing that
 * the backend decides: they sort, count and word what the backend answered.
 */

/** The sprints of a project the way the Backlog view lays them out. */
export interface SprintLayout {
  active: Sprint | undefined;
  planned: Sprint[];
  /** The completed sprints, the one completed last first. */
  completed: Sprint[];
}

/** Sorts the sprints the backend listed into the parts of the view. */
export function layoutOf(sprints: readonly Sprint[]): SprintLayout {
  return {
    active: sprints.find((sprint) => sprint.state === 'active'),
    planned: sprints.filter((sprint) => sprint.state === 'planned'),
    completed: sprints.filter((sprint) => sprint.state === 'completed'),
  };
}

/**
 * The tasks of a sprint, or of the backlog for nothing, as rows: without
 * subtasks, which go with their task, and ordered the way the list view
 * orders them, by the place of their status in the workflow and then by
 * their rank.
 */
export function rowsOf(project: Project, tasks: readonly Task[], sprintId: Id | null): Task[] {
  const placeOfStatus = new Map(project.statuses.map((status, place) => [status.id, place]));
  const place = (task: Task) => placeOfStatus.get(task.statusId) ?? project.statuses.length;
  return tasks
    .filter((task) => task.parentId === null && task.sprintId === sprintId)
    .sort((first, second) => place(first) - place(second) || asArranged(first, second));
}

/** The finished tasks among some: those in a status of the category done. */
export function finishedOf(project: Project, tasks: readonly Task[]): Task[] {
  const done = new Set(
    project.statuses.filter((status) => status.category === 'done').map((status) => status.id),
  );
  return tasks.filter((task) => done.has(task.statusId));
}

/** How much a list of tasks holds: how many there are, and their points. */
export interface Totals {
  tasks: number;
  points: number;
  /** How many of the tasks have no points. */
  withoutPoints: number;
}

export function totalsOf(rows: readonly Task[]): Totals {
  return {
    tasks: rows.length,
    points: rows.reduce((sum, task) => sum + (task.points ?? 0), 0),
    withoutPoints: rows.filter((task) => task.points === null).length,
  };
}

/** Says an amount of work: "1 task", "8 points". */
export function formatWork(amount: number, unit: SprintUnit): string {
  const word = unit === 'points' ? 'point' : 'task';
  return `${amount.toString()} ${amount === 1 ? word : `${word}s`}`;
}

/** Says the totals of a list in a few words: "6 tasks · 22 points". */
export function formatTotals(totals: Totals): string {
  const tasks = formatWork(totals.tasks, 'tasks');
  return totals.points === 0 && totals.withoutPoints === totals.tasks
    ? tasks
    : `${tasks} · ${formatWork(totals.points, 'points')}`;
}

/** Says what a completed sprint did: "5 of 10 points". */
export function formatSummary(summary: SprintSummary): string {
  return `${summary.completed.toString()} of ${formatWork(summary.committed, summary.unit)}`;
}

/** Says the dates of a sprint: "10 Nov – 23 Nov", or that it has none yet. */
export function formatSprintDates(
  sprint: Pick<Sprint, 'startOn' | 'endOn'>,
  now: Date = new Date(),
): string {
  if (sprint.startOn !== null && sprint.endOn !== null) {
    return `${formatDate(sprint.startOn, now)} – ${formatDate(sprint.endOn, now)}`;
  }
  if (sprint.startOn !== null) {
    return `From ${formatDate(sprint.startOn, now)}`;
  }
  if (sprint.endOn !== null) {
    return `Until ${formatDate(sprint.endOn, now)}`;
  }
  return 'No dates yet';
}

/** The days from today to the last day of a sprint. Zero on the last day, negative after it. */
export function daysLeft(endOn: CalendarDate, today: CalendarDate): number {
  return dayNumberOf(endOn) - dayNumberOf(today);
}

/** Says how long a sprint still runs: "9 days left", "Last day", "Over by 2 days". */
export function formatDaysLeft(days: number): string {
  if (days === 0) {
    return 'Last day';
  }
  const count = Math.abs(days);
  const dayWord = count === 1 ? 'day' : 'days';
  return days > 0
    ? `${count.toString()} ${dayWord} left`
    : `Over by ${count.toString()} ${dayWord}`;
}

/** The lengths a sprint can be given in one step, in weeks. */
export const SPRINT_LENGTHS_IN_WEEKS = [1, 2, 4] as const;

/** The last day of a sprint that starts on a day and lasts some weeks. */
export function endAfterWeeks(startOn: CalendarDate, weeks: number): CalendarDate {
  return addDays(startOn, weeks * 7);
}

/** The dates a sprint is offered when it is started: today and two weeks later. */
export function suggestedDates(
  sprint: Pick<Sprint, 'startOn' | 'endOn'>,
  today: CalendarDate,
): { startOn: CalendarDate; endOn: CalendarDate } {
  const startOn = sprint.startOn ?? today;
  return { startOn, endOn: sprint.endOn ?? endAfterWeeks(startOn, 2) };
}

/** The sprints a task can be moved to from a menu: the planned ones and the active one. */
export function openSprints(sprints: readonly Sprint[]): Sprint[] {
  return sprints.filter((sprint) => sprint.state !== 'completed');
}

/**
 * A copy of the list in which a task and its subtasks are in another sprint,
 * or in the backlog, the way the backend will plan them. The interface
 * shows this at once instead of waiting for the backend's answer.
 */
export function withPlannedTask(tasks: readonly Task[], taskId: Id, sprintId: Id | null): Task[] {
  const moving = new Set<Id>([taskId]);
  for (let sizeBefore = 0; sizeBefore !== moving.size;) {
    sizeBefore = moving.size;
    for (const task of tasks) {
      if (task.parentId !== null && moving.has(task.parentId)) {
        moving.add(task.id);
      }
    }
  }
  return tasks.map((task) => (moving.has(task.id) ? { ...task, sprintId } : task));
}

/** One completed sprint in the comparison of sprints: its work as shares of the largest. */
export interface PaceBar {
  sprint: Sprint;
  summary: SprintSummary;
  /** What the sprint took on, as a share of what the largest sprint took on. */
  committedShare: number;
  /** What it finished, on the same scale. */
  completedShare: number;
}

/**
 * The bars that set completed sprints side by side, so that the pace from
 * sprint to sprint can be seen. Sprints counted in points are measured
 * against each other, and so are sprints counted in tasks.
 */
export function paceBarsOf(completed: readonly Sprint[]): PaceBar[] {
  const largest = (unit: SprintUnit) =>
    Math.max(
      1,
      ...completed.flatMap((sprint) =>
        sprint.summary?.unit === unit ? [sprint.summary.committed] : [],
      ),
    );
  const scale = { points: largest('points'), tasks: largest('tasks') };
  return completed.flatMap((sprint) => {
    const summary = sprint.summary;
    if (!summary) {
      return [];
    }
    const of = scale[summary.unit];
    return [
      {
        sprint,
        summary,
        committedShare: summary.committed / of,
        completedShare: summary.completed / of,
      },
    ];
  });
}

/**
 * Something that tells when the report of a sprint is out of date: it
 * changes whenever a task of the sprint changes, joins or leaves.
 */
export function reportVersionOf(tasks: readonly Task[], sprintId: Id): string {
  const ofSprint = tasks.filter((task) => task.sprintId === sprintId);
  const latest = ofSprint.reduce(
    (last, task) => (task.updatedAt > last ? task.updatedAt : last),
    '',
  );
  return `${ofSprint.length.toString()}:${latest}`;
}

/**
 * Reads story points the way people type them: digits. Returns the number,
 * `null` for an empty field (no points), or `undefined` for anything else.
 * Whether the number is allowed is for the backend to say.
 */
export function parsePoints(text: string): number | null | undefined {
  const trimmed = text.trim();
  if (trimmed === '') {
    return null;
  }
  return /^\d+$/.test(trimmed) ? Number(trimmed) : undefined;
}
