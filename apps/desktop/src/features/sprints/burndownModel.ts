import type {
  CalendarDate,
  ScopeChange,
  ScopeTask,
  SprintReport,
  SprintUnit,
} from '@/shared/api/types';

import { formatWork } from './sprintModel';

/**
 * The geometry of a burndown chart, worked out from the report of a sprint.
 *
 * The chart reads from left to right through the days of the sprint. The
 * left edge is the beginning of the first day and the right edge the end of
 * the last; every day is a band of the same width. Up is work: the top is
 * the most work the sprint ever held and the bottom is none.
 */

/** The size of the drawing, in the units of its view box. */
export interface ChartSize {
  width: number;
  height: number;
}

export interface Point {
  x: number;
  y: number;
}

/** A day on which what the sprint held changed, as it is marked on the chart. */
export interface ScopeMark {
  /** Where the mark stands: at the end of the day, on the line of what the sprint held. */
  x: number;
  y: number;
  /** Whether the sprint held more at the end of the day than before, or less. */
  direction: 'up' | 'down';
  /** What changed, in words: "1 task joined, 3 points". */
  words: string;
  change: ScopeChange;
}

export interface Burndown {
  /** The line from what the sprint was started with to none at the end: the pace the dates ask for. */
  ideal: string;
  /**
   * What really remained, as a path of steps: level through each day, then
   * to what remained at its end. It steps up on a day work joined the
   * sprint. Nothing if no day has come yet.
   */
  actual: string | null;
  /**
   * All the work the sprint held, as a quieter path of steps, drawn only
   * when it changed while the sprint ran. Nothing otherwise.
   */
  scope: string | null;
  /** The days on which what the sprint held changed. */
  marks: ScopeMark[];
  /** Where the line of what remained ends. */
  end: Point | null;
  /** Where the end of today lies, for its mark. */
  todayX: number | null;
  /** Where the bands of the days begin, and where the last one ends: a tick for each. */
  dayTicks: number[];
  /** The work that remains now: at the end of the last day that has come. */
  remaining: number | null;
}

/**
 * Works out the burndown chart of a report, or nothing for a report
 * without days.
 *
 * `today` places the mark of today at the end of the band of today, where
 * the line of what remained ends. A sprint that has not reached today, or
 * that is past its last day, has no mark. Without a `today`, as for a sprint
 * that is over, there is no mark either.
 */
export function burndownOf(
  report: SprintReport,
  size: ChartSize,
  today: CalendarDate | null,
): Burndown | null {
  const dayCount = report.days.length;
  if (dayCount === 0) {
    return null;
  }
  // The top is the most the sprint held, so that work that joined stays on
  // the chart. A sprint without work is drawn as if it had one unit, so the
  // lines lie at the bottom instead of dividing by nothing.
  const top = Math.max(
    report.committed,
    ...report.days.map((day) => Math.max(day.scope ?? 0, day.remaining ?? 0)),
    1,
  );
  const x = (day: number) => round((day / dayCount) * size.width);
  const y = (work: number) => round(size.height - (work / top) * size.height);

  const steps: string[] = [];
  const scopeSteps: string[] = [];
  let remaining: number | null = null;
  let end: Point | null = null;
  for (const [index, day] of report.days.entries()) {
    if (day.remaining !== null) {
      steps.push(`H${x(index + 1).toString()}`, `V${y(day.remaining).toString()}`);
      remaining = day.remaining;
      end = { x: x(index + 1), y: y(day.remaining) };
    }
    if (day.scope !== null) {
      scopeSteps.push(`H${x(index + 1).toString()}`, `V${y(day.scope).toString()}`);
    }
  }

  const marks = report.changes.flatMap((change): ScopeMark[] => {
    const index = report.days.findIndex((day) => day.date === change.date);
    const held = report.days[index]?.scope;
    if (index === -1 || held === null || held === undefined) {
      return [];
    }
    const net = sumOf(change.joined) - sumOf(change.left) + change.reestimated;
    const tasks = change.joined.length - change.left.length;
    return [
      {
        x: x(index + 1),
        y: y(held),
        direction: net > 0 || (net === 0 && tasks >= 0) ? 'up' : 'down',
        words: scopeChangeInWords(change, report.unit),
        change,
      },
    ];
  });

  const todayIndex = today === null ? -1 : report.days.findIndex((day) => day.date === today);
  const start = y(report.committed).toString();
  return {
    ideal: `M0 ${start}L${size.width.toString()} ${y(0).toString()}`,
    actual: steps.length === 0 ? null : `M0 ${start}${steps.join('')}`,
    scope:
      marks.length === 0 || scopeSteps.length === 0 ? null : `M0 ${start}${scopeSteps.join('')}`,
    marks,
    end,
    todayX: todayIndex === -1 ? null : x(todayIndex + 1),
    dayTicks: Array.from({ length: dayCount + 1 }, (_, day) => x(day)),
    remaining,
  };
}

/**
 * What changed on one day, in a few words: "2 tasks joined, 5 points; 1 task
 * left, 2 points". In a sprint counted in tasks the points are not said; in
 * one counted in points, tasks without points are said to have none.
 */
export function scopeChangeInWords(change: ScopeChange, unit: SprintUnit): string {
  const parts: string[] = [];
  const group = (tasks: readonly ScopeTask[], verb: string) => {
    if (tasks.length === 0) {
      return;
    }
    const count = formatWork(tasks.length, 'tasks');
    if (unit === 'tasks') {
      parts.push(`${count} ${verb}`);
      return;
    }
    const unpointed = tasks.filter((task) => task.hasNoPoints).length;
    const weight = formatWork(sumOf(tasks), 'points');
    parts.push(
      unpointed === 0
        ? `${count} ${verb}, ${weight}`
        : `${count} ${verb}, ${weight} (${unpointed.toString()} without points)`,
    );
  };
  group(change.joined, 'joined');
  group(change.left, 'left');
  if (change.reestimated !== 0) {
    const sign = change.reestimated > 0 ? '+' : '−';
    parts.push(`re-estimated, ${sign}${formatWork(Math.abs(change.reestimated), 'points')}`);
  }
  return parts.join('; ');
}

function sumOf(tasks: readonly ScopeTask[]): number {
  return tasks.reduce((sum, task) => sum + task.work, 0);
}

/** Keeps the coordinates short and exact to a hundredth. */
function round(value: number): number {
  return Math.round(value * 100) / 100;
}
