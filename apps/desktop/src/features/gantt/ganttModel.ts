import type { CalendarDate, Id, Task } from '@/shared/api/types';
import { addDays, firstDayOfMonth, mondayOf, partsOfDay, type DayNumber } from '@/shared/lib/days';

import { indexSubtasks, isSubtask, outlineRows, type OutlineRow } from '../tasks/taskModel';
import { spanOf, type Span } from '../tasks/taskSpan';

/**
 * The arithmetic of the Gantt chart: which days a task takes up, which days
 * the chart shows, where a bar lies, and how the line of a dependency runs.
 *
 * Everything here is a pure function of the tasks, so it can be tested
 * without drawing anything. The chart only draws what these functions say.
 */

/** The width of one day on the chart, in pixels. */
export const DAY_WIDTH = 28;

/** The height of one row of the chart, in pixels. */
export const ROW_HEIGHT = 36;

/** The height of a bar, in pixels. */
export const BAR_HEIGHT = 16;

/** The gap between a bar and the edge of its first and last day, in pixels. */
export const BAR_INSET = 2;

/** The width of the mark of a task that has a due date and no start, in pixels. */
export const DEADLINE_MARK_WIDTH = 12;

/** The days a chart shows. */
export interface ChartRange {
  firstDay: DayNumber;
  dayCount: number;
}

const DAYS_SHOWN_BEFORE = 7;
const DAYS_SHOWN_AFTER = 14;
const FEWEST_DAYS_SHOWN = 56;
const LAST_WEEKDAY = 6;

/**
 * The days a chart has to show: from a week before the first thing on it to
 * two weeks after the last, in whole weeks from a Monday to a Sunday, and
 * never fewer than eight weeks. Today is always among them.
 */
export function chartRange(spans: readonly Span[], today: DayNumber): ChartRange {
  const earliest = spans.reduce((day, span) => Math.min(day, span.firstDay), today);
  const latest = spans.reduce((day, span) => Math.max(day, span.lastDay), today);
  const firstDay = mondayOf(earliest - DAYS_SHOWN_BEFORE);
  const lastDay = mondayOf(latest + DAYS_SHOWN_AFTER) + LAST_WEEKDAY;
  return { firstDay, dayCount: Math.max(lastDay - firstDay + 1, FEWEST_DAYS_SHOWN) };
}

/** The part of a month that a chart shows. */
export interface MonthOnChart {
  year: number;
  /** The month, counted from 1. */
  month: number;
  /** The first day of the month that is on the chart. */
  firstDay: DayNumber;
  /** How many days of the month are on the chart. */
  dayCount: number;
}

/** The months a chart shows, each with the days of it that are on the chart. */
export function monthsOf(range: ChartRange): MonthOnChart[] {
  const months: MonthOnChart[] = [];
  const end = range.firstDay + range.dayCount;
  let day = range.firstDay;
  while (day < end) {
    const { year, month } = partsOfDay(day);
    const nextMonth = firstDayOfMonth(year, month + 1);
    const dayCount = Math.min(nextMonth, end) - day;
    months.push({ year, month, firstDay: day, dayCount });
    day += dayCount;
  }
  return months;
}

/**
 * Sorts tasks the way a schedule lists them: by the day they begin, then by
 * the day they end, then by number. Tasks without any date come last.
 */
export function asScheduled(first: Task, second: Task): number {
  const firstSpan = spanOf(first);
  const secondSpan = spanOf(second);
  if (firstSpan && secondSpan) {
    return (
      firstSpan.firstDay - secondSpan.firstDay ||
      firstSpan.lastDay - secondSpan.lastDay ||
      first.number - second.number
    );
  }
  if (firstSpan) {
    return -1;
  }
  if (secondSpan) {
    return 1;
  }
  return first.number - second.number;
}

/** One row of the chart: a task of the outline, and the days it takes up. */
export interface ChartRow extends OutlineRow {
  span: Span | undefined;
}

/**
 * The rows of the chart: the tasks in the order of the schedule, each one
 * followed by its subtasks if it is open.
 */
export function chartRows(tasks: readonly Task[], isOpen: (taskId: Id) => boolean): ChartRow[] {
  const subtasksByParent = indexSubtasks(tasks);
  for (const subtasks of subtasksByParent.values()) {
    subtasks.sort(asScheduled);
  }
  const mainTasks = tasks.filter((task) => !isSubtask(task)).sort(asScheduled);
  return outlineRows(mainTasks, subtasksByParent, isOpen).map((row) => ({
    ...row,
    span: spanOf(row.task),
  }));
}

/** A stretch of the chart from left to right, in pixels from its left edge. */
export interface Box {
  left: number;
  width: number;
}

/** Where the days of a span lie on the chart. */
export function boxOf(span: Span, range: ChartRange, dayWidth: number = DAY_WIDTH): Box {
  return {
    left: (span.firstDay - range.firstDay) * dayWidth,
    width: (span.lastDay - span.firstDay + 1) * dayWidth,
  };
}

/**
 * Where the bar of a span is drawn within its days. A bar keeps a small gap
 * to the edges of its days, so two bars that follow each other do not touch.
 * A task with only a due date is a small mark in the middle of that day.
 */
export function barOf(span: Span, range: ChartRange, dayWidth: number = DAY_WIDTH): Box {
  const days = boxOf(span, range, dayWidth);
  if (span.kind === 'deadline') {
    return {
      left: days.left + (days.width - DEADLINE_MARK_WIDTH) / 2,
      width: DEADLINE_MARK_WIDTH,
    };
  }
  return { left: days.left + BAR_INSET, width: days.width - 2 * BAR_INSET };
}

/** A point of the chart, in pixels from its top left corner. */
export interface Point {
  x: number;
  y: number;
}

/** The line that shows that one task waits for another. */
export interface Link {
  /** The task that has to be finished first. */
  blockerId: Id;
  /** The task that waits for it. */
  waitingId: Id;
  /**
   * Whether the plan contradicts the dependency: the waiting task begins
   * before its blocker is due, and the blocker is not finished.
   */
  isBroken: boolean;
  /** The line as the `d` of an SVG path, from the blocker to the waiting task. */
  path: string;
}

/** A bar as the line of a dependency sees it, in pixels from the top left of the chart. */
export interface BarOnRow {
  left: number;
  right: number;
  /** The middle of the row the bar is in, from the top of the chart. */
  middle: number;
}

/** How far a line runs straight out of a bar before it turns, in pixels. */
const LINK_STUB = 10;

/** The radius of the corners of a line, in pixels. */
const LINK_CORNER = 4;

/** How far from the end of a bar a line meets its top or its bottom, in pixels. */
const LINK_INSET = 6;

/**
 * The lines of the dependencies between the rows that are shown. A
 * dependency whose blocker or waiting task has no date, or is folded away,
 * has no line.
 */
export function linksOf(
  rows: readonly ChartRow[],
  range: ChartRange,
  dayWidth: number = DAY_WIDTH,
  rowHeight: number = ROW_HEIGHT,
): Link[] {
  const positions = new Map(rows.map((row, position) => [row.task.id, position]));
  const middleOf = (position: number) => position * rowHeight + rowHeight / 2;

  return rows.flatMap((waiting, waitingPosition) => {
    const waitingSpan = waiting.span;
    if (!waitingSpan) {
      return [];
    }
    return waiting.task.blockedBy.flatMap((blockerId): Link[] => {
      const blockerPosition = positions.get(blockerId);
      const blocker = blockerPosition === undefined ? undefined : rows[blockerPosition];
      if (blockerPosition === undefined || !blocker?.span) {
        return [];
      }
      const blockerBar = barOf(blocker.span, range, dayWidth);
      const waitingBar = barOf(waitingSpan, range, dayWidth);
      return [
        {
          blockerId,
          waitingId: waiting.task.id,
          isBroken:
            blocker.task.completedAt === null && waitingSpan.firstDay < blocker.span.lastDay,
          path: linkPath(
            {
              left: blockerBar.left,
              right: blockerBar.left + blockerBar.width,
              middle: middleOf(blockerPosition),
            },
            {
              left: waitingBar.left,
              right: waitingBar.left + waitingBar.width,
              middle: middleOf(waitingPosition),
            },
            rowHeight,
          ),
        },
      ];
    });
  });
}

/**
 * The line that leads from the bar of a task to the bar of a task that waits
 * for it. Which way it runs depends on how the two bars lie:
 *
 * - With room between them, the line leaves the end of the first bar, turns
 *   towards the row of the second, and enters it at its beginning.
 * - When the second bar begins right after the first, there is no room for
 *   two turns. The line leaves the end of the first bar and drops onto the
 *   second, just behind its beginning.
 * - When the second bar begins before the first one ends, the two overlap in
 *   time. The line is a plumb line between them, near the end of the first.
 * - When the second bar lies entirely before the first, the line goes
 *   around: back between the rows, and into the second bar from the left.
 */
export function linkPath(
  blocker: BarOnRow,
  waiting: BarOnRow,
  rowHeight: number = ROW_HEIGHT,
): string {
  // `1` when the waiting task is in a row further down, `-1` when further up.
  const direction = Math.sign(waiting.middle - blocker.middle) || 1;
  const halfBar = BAR_HEIGHT / 2;
  // The edge of the waiting bar that faces the blocker: its top or its bottom.
  const facingEdge = waiting.middle - direction * halfBar;
  const end = { x: blocker.right, y: blocker.middle };

  if (waiting.left >= blocker.right + 2 * LINK_STUB) {
    const turn = blocker.right + LINK_STUB;
    return roundedPath(
      [
        end,
        { x: turn, y: blocker.middle },
        { x: turn, y: waiting.middle },
        { x: waiting.left, y: waiting.middle },
      ],
      LINK_CORNER,
    );
  }

  if (waiting.left >= blocker.right) {
    const drop = Math.min(waiting.left + LINK_INSET, waiting.right);
    return roundedPath(
      [end, { x: drop, y: blocker.middle }, { x: drop, y: facingEdge }],
      LINK_CORNER,
    );
  }

  const plumb = Math.min(blocker.right, waiting.right) - LINK_INSET;
  if (plumb >= Math.max(blocker.left, waiting.left)) {
    return roundedPath(
      [
        { x: plumb, y: blocker.middle + direction * halfBar },
        { x: plumb, y: facingEdge },
      ],
      LINK_CORNER,
    );
  }

  const out = blocker.right + LINK_STUB;
  const between = blocker.middle + (direction * rowHeight) / 2;
  const back = waiting.left - LINK_STUB;
  return roundedPath(
    [
      end,
      { x: out, y: blocker.middle },
      { x: out, y: between },
      { x: back, y: between },
      { x: back, y: waiting.middle },
      { x: waiting.left, y: waiting.middle },
    ],
    LINK_CORNER,
  );
}

/** A line through some points, with its corners eased. */
function roundedPath(points: readonly Point[], radius: number): string {
  const [first, ...rest] = points;
  if (!first) {
    return '';
  }
  const commands = [`M ${first.x.toString()} ${first.y.toString()}`];
  let previous = first;
  rest.forEach((corner, position) => {
    const next = rest[position + 1];
    if (!next) {
      commands.push(`L ${corner.x.toString()} ${corner.y.toString()}`);
      return;
    }
    const before = towards(corner, previous, radius);
    const after = towards(corner, next, radius);
    commands.push(
      `L ${before.x.toString()} ${before.y.toString()}`,
      `Q ${corner.x.toString()} ${corner.y.toString()} ${after.x.toString()} ${after.y.toString()}`,
    );
    previous = corner;
  });
  return commands.join(' ');
}

/** The point that lies `distance` from `start` in the direction of `target`, at most half way. */
function towards(start: Point, target: Point, distance: number): Point {
  const length = Math.hypot(target.x - start.x, target.y - start.y);
  if (length === 0) {
    return start;
  }
  const step = Math.min(distance, length / 2) / length;
  return {
    x: start.x + (target.x - start.x) * step,
    y: start.y + (target.y - start.y) * step,
  };
}

/** The part of a bar that is held while it is dragged. */
export type Grip = 'whole' | 'start' | 'end';

/** The dates of a task. */
export interface Dates {
  startOn: CalendarDate | null;
  dueOn: CalendarDate | null;
}

/**
 * The dates a task gets when its bar is dragged some days to the right, or
 * with a negative number to the left.
 *
 * Held as a whole, the bar moves and keeps its length. Held at one end, only
 * that date changes, and it stops at the other date: a task cannot start
 * after it is due. Pulling out the end of a task that has only a start
 * gives it a due date.
 */
export function draggedDates(task: Dates, grip: Grip, days: number): Dates {
  const { startOn, dueOn } = task;
  if (grip === 'whole') {
    return {
      startOn: startOn === null ? null : addDays(startOn, days),
      dueOn: dueOn === null ? null : addDays(dueOn, days),
    };
  }
  if (grip === 'start') {
    if (startOn === null) {
      return task;
    }
    const moved = addDays(startOn, days);
    return { startOn: dueOn !== null && moved > dueOn ? dueOn : moved, dueOn };
  }
  if (dueOn === null) {
    return startOn !== null && days > 0 ? { startOn, dueOn: addDays(startOn, days) } : task;
  }
  const moved = addDays(dueOn, days);
  return { startOn, dueOn: startOn !== null && moved < startOn ? startOn : moved };
}
