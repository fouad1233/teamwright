import { describe, expect, it } from 'vitest';

import type { TaskActual } from '@/shared/api/types';
import { dayNumberOf } from '@/shared/lib/days';

import {
  DAY_WIDTH,
  actualBarOf,
  actualInWords,
  parseGanttContent,
  serializeGanttContent,
} from './ganttModel';

const RANGE = { firstDay: dayNumberOf('2026-11-09'), dayCount: 56 };
const TODAY = dayNumberOf('2026-11-15');

const NOTHING_LATE: TaskActual = {
  taskId: 'task',
  startedAt: '2026-11-10T09:00:00.000Z',
  finishedAt: '2026-11-12T09:00:00.000Z',
  startedOn: '2026-11-10',
  finishedOn: '2026-11-12',
  reopened: false,
  startedLateDays: null,
  notStarted: false,
  finishedLateDays: null,
  runningLateDays: null,
};

describe('the line of what really happened', () => {
  it('runs from the day work began to the day it was finished', () => {
    expect(actualBarOf(NOTHING_LATE, '2026-11-12', RANGE, TODAY)).toEqual({
      left: DAY_WIDTH + 2,
      width: 3 * DAY_WIDTH - 4,
      isRunning: false,
      overdueFrom: null,
    });
  });

  it('runs to today while work goes on, and hatches the days past the due day', () => {
    const running = { ...NOTHING_LATE, finishedAt: null, finishedOn: null };

    const line = actualBarOf(running, '2026-11-11', RANGE, TODAY);

    expect(line?.isRunning).toBe(true);
    expect(line?.width).toBe(6 * DAY_WIDTH - 4);
    // The due day is the second day of the line.
    expect(line?.overdueFrom).toBe(2 * DAY_WIDTH - 2);
  });

  it('is not drawn for a task on which work has not begun', () => {
    const notBegun = { ...NOTHING_LATE, startedOn: null, startedAt: null };
    expect(actualBarOf(notBegun, null, RANGE, TODAY)).toBeNull();
  });
});

describe('what really happened, in words', () => {
  it('tells a late start, a late finish, work past its due day and a reopening apart', () => {
    expect(
      actualInWords(
        { ...NOTHING_LATE, startedLateDays: 1, finishedLateDays: 2, reopened: true },
        true,
      ),
    ).toEqual(['began 1 day late', 'finished 2 days late', 'opened again']);
    expect(actualInWords({ ...NOTHING_LATE, runningLateDays: 3 }, true)).toEqual([
      '3 days past due',
    ]);
    expect(
      actualInWords(
        { ...NOTHING_LATE, startedOn: null, startedLateDays: 2, notStarted: true },
        true,
      ),
    ).toEqual(['not begun, 2 days late']);
  });

  it('says so for a task without planned dates, and nothing for one on time', () => {
    expect(actualInWords(NOTHING_LATE, false)).toEqual(['no planned dates']);
    expect(actualInWords(NOTHING_LATE, true)).toEqual([]);
  });
});

describe('what the Gantt view keeps', () => {
  it('keeps whether it shows what really happened, and reads anything else as the plan only', () => {
    expect(parseGanttContent(serializeGanttContent({ showsActual: true }))).toEqual({
      showsActual: true,
    });
    expect(serializeGanttContent({ showsActual: false })).toBeNull();
    expect(parseGanttContent('not json')).toEqual({ showsActual: false });
    expect(parseGanttContent(null)).toEqual({ showsActual: false });
  });
});
