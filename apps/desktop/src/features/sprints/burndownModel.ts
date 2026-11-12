import type { CalendarDate, SprintReport } from '@/shared/api/types';

/**
 * The geometry of a burndown chart, worked out from the report of a sprint.
 *
 * The chart reads from left to right through the days of the sprint. The
 * left edge is the beginning of the first day and the right edge the end of
 * the last; every day is a band of the same width. Up is work: the top is
 * all the work of the sprint and the bottom is none.
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

export interface Burndown {
  /** The line from all the work at the start to none at the end: the pace the dates ask for. */
  ideal: string;
  /**
   * What really remained, as a path of steps: level through each day, then
   * down to what remained at its end. Nothing if no day has come yet.
   */
  actual: string | null;
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
 * that is past its last day, has no mark.
 */
export function burndownOf(
  report: SprintReport,
  size: ChartSize,
  today: CalendarDate,
): Burndown | null {
  const dayCount = report.days.length;
  if (dayCount === 0) {
    return null;
  }
  // A sprint without work is drawn as if it had one unit, so the lines lie
  // at the bottom instead of dividing by nothing.
  const top = Math.max(report.total, 1);
  const x = (day: number) => round((day / dayCount) * size.width);
  const y = (work: number) => round(size.height - (work / top) * size.height);

  const steps: string[] = [];
  let remaining: number | null = null;
  let end: Point | null = null;
  for (const [index, day] of report.days.entries()) {
    if (day.remaining !== null) {
      steps.push(`H${x(index + 1).toString()}`, `V${y(day.remaining).toString()}`);
      remaining = day.remaining;
      end = { x: x(index + 1), y: y(day.remaining) };
    }
  }

  const todayIndex = report.days.findIndex((day) => day.date === today);
  return {
    ideal: `M0 ${y(report.total).toString()}L${size.width.toString()} ${y(0).toString()}`,
    actual: steps.length === 0 ? null : `M0 ${y(report.total).toString()}${steps.join('')}`,
    end,
    todayX: todayIndex === -1 ? null : x(todayIndex + 1),
    dayTicks: Array.from({ length: dayCount + 1 }, (_, day) => x(day)),
    remaining,
  };
}

/** Keeps the coordinates short and exact to a hundredth. */
function round(value: number): number {
  return Math.round(value * 100) / 100;
}
