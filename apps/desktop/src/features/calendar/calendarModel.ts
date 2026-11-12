import type { Task } from '@/shared/api/types';
import { firstDayOfMonth, mondayOf, partsOfDay, type DayNumber } from '@/shared/lib/days';

import { spanOf } from '../tasks/taskSpan';

/**
 * The arithmetic of the calendar: which days a month shows, and where the
 * tasks of a week go.
 *
 * Everything here is a pure function of the tasks, so it can be tested
 * without drawing anything.
 */

const DAYS_PER_WEEK = 7;

/** A month view always shows this many weeks, so its height never changes. */
const WEEKS_SHOWN = 6;

/** A month of a year. `month` counts from 1. */
export interface Month {
  year: number;
  month: number;
}

/** The month a day is in. */
export function monthOf(day: DayNumber): Month {
  const { year, month } = partsOfDay(day);
  return { year, month };
}

/** The month that lies some months after another. A negative number goes back. */
export function addMonths({ year, month }: Month, months: number): Month {
  return monthOf(firstDayOfMonth(year, month + months));
}

/**
 * The weeks a month view shows: six weeks of seven days, from the Monday on
 * or before the first of the month. The first and the last week usually
 * hold days of the months next to it.
 */
export function weeksOfMonth({ year, month }: Month): DayNumber[][] {
  const firstMonday = mondayOf(firstDayOfMonth(year, month));
  return Array.from({ length: WEEKS_SHOWN }, (_, week) =>
    Array.from({ length: DAYS_PER_WEEK }, (_, weekday) => {
      return firstMonday + week * DAYS_PER_WEEK + weekday;
    }),
  );
}

/** The part of a task that lies in one week of the calendar. */
export interface Piece {
  task: Task;
  /** The first column of the week the piece is in: `0` for Monday. */
  firstColumn: number;
  /** How many days of the week the piece covers. */
  columnCount: number;
  /** The line of the week the piece is on: `0` for the first one. */
  lane: number;
  /** Whether the task begins in this week. Otherwise it comes over from the week before. */
  beginsHere: boolean;
  /** Whether the task ends in this week. Otherwise it goes on into the next week. */
  endsHere: boolean;
}

/** Where the tasks of one week go. */
export interface WeekLayout {
  /** The pieces that have a line to themselves, from the first line down. */
  pieces: Piece[];
  /** For each of the seven days, the tasks of that day that found no line. */
  crowdedOut: Task[][];
}

/**
 * Lays the tasks of one week out on lines, the way a wall calendar is
 * filled in: a task that takes several days is one piece across them, and
 * two tasks share a line only if they do not meet. Longer and earlier tasks
 * get the upper lines. A task that finds no room among the lines is listed
 * with each of its days instead.
 *
 * `days` are the seven days of the week, from Monday on.
 */
export function layoutWeek(
  days: readonly DayNumber[],
  tasks: readonly Task[],
  laneCount: number,
): WeekLayout {
  const firstDay = days[0] ?? 0;
  const lastDay = firstDay + DAYS_PER_WEEK - 1;

  const inWeek = tasks
    .flatMap((task) => {
      const span = spanOf(task);
      return span && span.lastDay >= firstDay && span.firstDay <= lastDay ? [{ task, span }] : [];
    })
    .sort(
      (first, second) =>
        first.span.firstDay - second.span.firstDay ||
        second.span.lastDay - first.span.lastDay ||
        first.task.number - second.task.number,
    );

  const pieces: Piece[] = [];
  const crowdedOut: Task[][] = Array.from({ length: DAYS_PER_WEEK }, () => []);
  // For every line, the last column that is taken on it.
  const takenUntil: number[] = [];

  for (const { task, span } of inWeek) {
    const firstColumn = Math.max(span.firstDay, firstDay) - firstDay;
    const lastColumn = Math.min(span.lastDay, lastDay) - firstDay;
    const freeLane = takenUntil.findIndex((column) => column < firstColumn);
    const lane = freeLane === -1 ? takenUntil.length : freeLane;
    takenUntil[lane] = lastColumn;

    if (lane < laneCount) {
      pieces.push({
        task,
        firstColumn,
        columnCount: lastColumn - firstColumn + 1,
        lane,
        beginsHere: span.firstDay >= firstDay,
        endsHere: span.lastDay <= lastDay,
      });
    } else {
      for (let column = firstColumn; column <= lastColumn; column += 1) {
        crowdedOut[column]?.push(task);
      }
    }
  }

  return { pieces, crowdedOut };
}

/** The tasks that take up a day, in the order the calendar lists them. */
export function tasksOn(day: DayNumber, tasks: readonly Task[]): Task[] {
  return tasks
    .flatMap((task) => {
      const span = spanOf(task);
      return span && span.firstDay <= day && span.lastDay >= day ? [{ task, span }] : [];
    })
    .sort(
      (first, second) =>
        first.span.firstDay - second.span.firstDay ||
        second.span.lastDay - first.span.lastDay ||
        first.task.number - second.task.number,
    )
    .map(({ task }) => task);
}

/**
 * How many lines of tasks a week needs for its pieces, and never fewer than
 * `fewest`, so that a week with little in it still has some height.
 */
export function lanesUsed(pieces: readonly Piece[], fewest: number): number {
  return pieces.reduce((count, piece) => Math.max(count, piece.lane + 1), fewest);
}
