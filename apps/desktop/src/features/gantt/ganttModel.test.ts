import { describe, expect, it } from 'vitest';

import type { Task } from '@/shared/api/types';
import { calendarDateOfDay, dayNumberOf, weekdayOf } from '@/shared/lib/days';

import { spanOf } from '../tasks/taskSpan';

import {
  BAR_HEIGHT,
  BAR_INSET,
  DEADLINE_MARK_WIDTH,
  asScheduled,
  barOf,
  boxOf,
  chartRange,
  chartRows,
  draggedDates,
  linkPath,
  linksOf,
  monthsOf,
  type ChartRange,
} from './ganttModel';

/** A task with only the fields a test cares about filled in. */
function task(overrides: Partial<Task> & Pick<Task, 'id'>): Task {
  return {
    projectId: 'project',
    number: 1,
    title: overrides.id,
    description: '',
    statusId: 'todo',
    priority: 'normal',
    assignee: null,
    parentId: null,
    blockedBy: [],
    estimateMinutes: null,
    sprintId: null,
    points: null,
    startOn: null,
    dueOn: null,
    rank: 0,
    createdAt: '2026-11-11T09:00:00.000Z',
    updatedAt: '2026-11-11T09:00:00.000Z',
    completedAt: null,
    ...overrides,
  };
}

const today = dayNumberOf('2026-11-12');

/** A chart that begins on Monday 2 November 2026, with days ten pixels wide. */
const range: ChartRange = { firstDay: dayNumberOf('2026-11-02'), dayCount: 56 };
const DAY = 10;
const ROW = 20;

describe('chartRange', () => {
  it('shows eight weeks around today when nothing is planned', () => {
    const shown = chartRange([], today);

    expect(calendarDateOfDay(shown.firstDay)).toBe('2026-11-02');
    expect(shown.dayCount).toBe(56);
  });

  it('begins on a Monday and covers whole weeks', () => {
    const planned = spanOf({ startOn: '2026-10-01', dueOn: '2027-02-10' });
    const shown = chartRange(planned ? [planned] : [], today);

    expect(weekdayOf(shown.firstDay)).toBe(0);
    expect(shown.dayCount % 7).toBe(0);
  });

  it('leaves a week before the first bar and two weeks after the last', () => {
    const planned = spanOf({ startOn: '2026-10-01', dueOn: '2027-02-10' });
    const shown = chartRange(planned ? [planned] : [], today);
    const lastDay = shown.firstDay + shown.dayCount - 1;

    expect(shown.firstDay).toBeLessThanOrEqual(dayNumberOf('2026-09-24'));
    expect(lastDay).toBeGreaterThanOrEqual(dayNumberOf('2027-02-24'));
  });

  it('always has today on it', () => {
    const longAgo = spanOf({ startOn: '2025-01-06', dueOn: '2025-01-10' });
    const shown = chartRange(longAgo ? [longAgo] : [], today);

    expect(shown.firstDay + shown.dayCount).toBeGreaterThan(today);
  });
});

describe('monthsOf', () => {
  it('lists the months of the chart with the days of each that are on it', () => {
    // Monday 26 October to Sunday 6 December 2026: six weeks.
    const months = monthsOf({ firstDay: dayNumberOf('2026-10-26'), dayCount: 42 });

    expect(
      months.map(({ year, month, firstDay, dayCount }) => [
        year,
        month,
        calendarDateOfDay(firstDay),
        dayCount,
      ]),
    ).toEqual([
      [2026, 10, '2026-10-26', 6],
      [2026, 11, '2026-11-01', 30],
      [2026, 12, '2026-12-01', 6],
    ]);
  });

  it('goes on into the next year', () => {
    const months = monthsOf({ firstDay: dayNumberOf('2026-12-28'), dayCount: 7 });

    expect(months.map(({ year, month, dayCount }) => [year, month, dayCount])).toEqual([
      [2026, 12, 4],
      [2027, 1, 3],
    ]);
  });
});

describe('asScheduled', () => {
  it('lists tasks by the day they begin, then by the day they end, then by number', () => {
    const tasks = [
      task({ id: 'later', number: 1, startOn: '2026-11-20', dueOn: '2026-11-21' }),
      task({ id: 'long', number: 2, startOn: '2026-11-10', dueOn: '2026-11-18' }),
      task({ id: 'short', number: 3, startOn: '2026-11-10', dueOn: '2026-11-12' }),
      task({ id: 'twin', number: 4, startOn: '2026-11-10', dueOn: '2026-11-12' }),
    ];

    expect(tasks.sort(asScheduled).map((listed) => listed.id)).toEqual([
      'short',
      'twin',
      'long',
      'later',
    ]);
  });

  it('puts tasks without a date last, by number', () => {
    const tasks = [
      task({ id: 'undated two', number: 2 }),
      task({ id: 'undated one', number: 1 }),
      task({ id: 'dated', number: 3, dueOn: '2026-12-24' }),
    ];

    expect(tasks.sort(asScheduled).map((listed) => listed.id)).toEqual([
      'dated',
      'undated one',
      'undated two',
    ]);
  });
});

describe('chartRows', () => {
  const tasks = [
    task({ id: 'second', number: 1, startOn: '2026-11-16', dueOn: '2026-11-18' }),
    task({ id: 'first', number: 2, startOn: '2026-11-09', dueOn: '2026-11-13' }),
    task({ id: 'late part', number: 3, parentId: 'first', dueOn: '2026-11-13' }),
    task({ id: 'early part', number: 4, parentId: 'first', dueOn: '2026-11-10' }),
  ];

  it('lists the tasks that are not part of another, in the order of the schedule', () => {
    const rows = chartRows(tasks, () => false);

    expect(rows.map((row) => row.task.id)).toEqual(['first', 'second']);
  });

  it('lists the subtasks of an open task under it, in the order of the schedule', () => {
    const rows = chartRows(tasks, (taskId) => taskId === 'first');

    expect(rows.map((row) => `${row.depth.toString()} ${row.task.id}`)).toEqual([
      '0 first',
      '1 early part',
      '1 late part',
      '0 second',
    ]);
  });

  it('gives every row the days its task takes up', () => {
    const [first] = chartRows(tasks, () => false);

    expect(first?.span?.kind).toBe('planned');
  });
});

describe('boxOf and barOf', () => {
  it('place a span by its days, counted from the first day of the chart', () => {
    const span = spanOf({ startOn: '2026-11-04', dueOn: '2026-11-06' });

    expect(span && boxOf(span, range, DAY)).toEqual({ left: 20, width: 30 });
  });

  it('draw a bar a little inside its days', () => {
    const span = spanOf({ startOn: '2026-11-04', dueOn: '2026-11-06' });

    expect(span && barOf(span, range, DAY)).toEqual({
      left: 20 + BAR_INSET,
      width: 30 - 2 * BAR_INSET,
    });
  });

  it('draw a deadline as a mark in the middle of its day', () => {
    const span = spanOf({ startOn: null, dueOn: '2026-11-04' });
    const bar = span && barOf(span, range, 40);

    expect(bar).toEqual({ left: 80 + (40 - DEADLINE_MARK_WIDTH) / 2, width: DEADLINE_MARK_WIDTH });
  });
});

describe('linksOf', () => {
  const blocker = task({ id: 'blocker', number: 1, startOn: '2026-11-02', dueOn: '2026-11-04' });
  const waiting = task({
    id: 'waiting',
    number: 2,
    startOn: '2026-11-09',
    dueOn: '2026-11-10',
    blockedBy: ['blocker'],
  });

  it('draws a line from the end of the blocker to the beginning of the waiting task', () => {
    const rows = chartRows([blocker, waiting], () => false);

    const [link] = linksOf(rows, range, DAY, ROW);

    expect(link).toMatchObject({ blockerId: 'blocker', waitingId: 'waiting', isBroken: false });
    // From the right end of the first bar, in the middle of the first row...
    expect(link?.path.startsWith(`M ${(30 - BAR_INSET).toString()} 10 `)).toBe(true);
    // ...to the left end of the second bar, in the middle of the second row.
    expect(link?.path.endsWith(` L ${(70 + BAR_INSET).toString()} 30`)).toBe(true);
  });

  it('calls a dependency broken when the waiting task begins before its blocker is due', () => {
    const early = { ...waiting, startOn: '2026-11-03' };
    const rows = chartRows([blocker, early], () => false);

    expect(linksOf(rows, range, DAY, ROW)[0]?.isBroken).toBe(true);
  });

  it('accepts a waiting task that begins on the day its blocker is due', () => {
    const sameDay = { ...waiting, startOn: '2026-11-04' };
    const rows = chartRows([blocker, sameDay], () => false);

    expect(linksOf(rows, range, DAY, ROW)[0]?.isBroken).toBe(false);
  });

  it('does not call a dependency broken once the blocker is finished', () => {
    const finished = { ...blocker, completedAt: '2026-11-05T10:00:00.000Z' };
    const early = { ...waiting, startOn: '2026-11-03' };
    const rows = chartRows([finished, early], () => false);

    expect(linksOf(rows, range, DAY, ROW)[0]?.isBroken).toBe(false);
  });

  it('draws no line to or from a task without a date', () => {
    const undated = { ...blocker, startOn: null, dueOn: null };
    const rows = chartRows([undated, waiting], () => false);

    expect(linksOf(rows, range, DAY, ROW)).toEqual([]);
  });

  it('draws no line to a task that is folded away', () => {
    const part = { ...waiting, parentId: 'blocker' };
    const rows = chartRows([blocker, part], () => false);

    expect(linksOf(rows, range, DAY, ROW)).toEqual([]);
  });
});

describe('linkPath', () => {
  // Rows are 20 pixels high, so their middles are 10, 30, 50 and so on. A
  // bar is 16 pixels high: its top is 8 above the middle of its row.
  const blocker = { left: 100, right: 200, middle: 10 };

  it('turns twice into the beginning of a bar that begins well after the first', () => {
    const path = linkPath(blocker, { left: 260, right: 320, middle: 50 }, ROW);

    expect(path.startsWith('M 200 10 ')).toBe(true);
    expect(path.match(/Q/g)).toHaveLength(2);
    expect(path.endsWith('L 260 50')).toBe(true);
  });

  it('drops onto a bar that begins right after the first', () => {
    const path = linkPath(blocker, { left: 204, right: 300, middle: 50 }, ROW);

    // It leaves the end of the first bar and comes down on the top of the
    // second, a little behind its beginning.
    expect(path.startsWith('M 200 10 ')).toBe(true);
    expect(path.match(/Q/g)).toHaveLength(1);
    expect(path.endsWith(`L 210 ${(50 - BAR_HEIGHT / 2).toString()}`)).toBe(true);
  });

  it('is a plumb line to a bar that begins before the first one ends', () => {
    const path = linkPath(blocker, { left: 150, right: 300, middle: 50 }, ROW);

    // From the bottom of the first bar straight down to the top of the second.
    expect(path).toBe(`M 194 ${(10 + BAR_HEIGHT / 2).toString()} L 194 42`);
  });

  it('is a plumb line upwards to a bar in a row above', () => {
    const path = linkPath(
      { left: 100, right: 200, middle: 50 },
      { left: 150, right: 300, middle: 10 },
      ROW,
    );

    expect(path).toBe('M 194 42 L 194 18');
  });

  it('meets a short mark in its middle', () => {
    // A mark twelve pixels wide that lies within the days of the first bar.
    const path = linkPath(blocker, { left: 180, right: 192, middle: 50 }, ROW);

    expect(path).toBe('M 186 18 L 186 42');
  });

  it('goes around to a bar that lies entirely before the first', () => {
    const path = linkPath(blocker, { left: 20, right: 60, middle: 50 }, ROW);

    expect(path.match(/Q/g)).toHaveLength(4);
    // It runs back between the two rows, ten pixels under the first one.
    expect(path).toContain(' 20 ');
    expect(path.endsWith('L 20 50')).toBe(true);
  });
});

describe('draggedDates', () => {
  const planned = { startOn: '2026-11-10', dueOn: '2026-11-14' };

  it('moves both dates when the bar is held as a whole', () => {
    expect(draggedDates(planned, 'whole', 3)).toEqual({
      startOn: '2026-11-13',
      dueOn: '2026-11-17',
    });
    expect(draggedDates(planned, 'whole', -10)).toEqual({
      startOn: '2026-10-31',
      dueOn: '2026-11-04',
    });
  });

  it('moves the one date of a task that has only one', () => {
    expect(draggedDates({ startOn: null, dueOn: '2026-11-14' }, 'whole', 2)).toEqual({
      startOn: null,
      dueOn: '2026-11-16',
    });
    expect(draggedDates({ startOn: '2026-11-10', dueOn: null }, 'whole', -1)).toEqual({
      startOn: '2026-11-09',
      dueOn: null,
    });
  });

  it('changes only the start when the bar is held at its beginning', () => {
    expect(draggedDates(planned, 'start', -2)).toEqual({
      startOn: '2026-11-08',
      dueOn: '2026-11-14',
    });
  });

  it('stops the start at the due date', () => {
    expect(draggedDates(planned, 'start', 30)).toEqual({
      startOn: '2026-11-14',
      dueOn: '2026-11-14',
    });
  });

  it('changes only the due date when the bar is held at its end', () => {
    expect(draggedDates(planned, 'end', 5)).toEqual({
      startOn: '2026-11-10',
      dueOn: '2026-11-19',
    });
  });

  it('stops the due date at the start', () => {
    expect(draggedDates(planned, 'end', -30)).toEqual({
      startOn: '2026-11-10',
      dueOn: '2026-11-10',
    });
  });

  it('gives a task with only a start a due date when its end is pulled out', () => {
    const open = { startOn: '2026-11-10', dueOn: null };

    expect(draggedDates(open, 'end', 4)).toEqual({ startOn: '2026-11-10', dueOn: '2026-11-14' });
    expect(draggedDates(open, 'end', -1)).toEqual(open);
  });

  it('leaves a task without a start alone when it is held at its beginning', () => {
    const deadline = { startOn: null, dueOn: '2026-11-14' };

    expect(draggedDates(deadline, 'start', -3)).toEqual(deadline);
  });
});
