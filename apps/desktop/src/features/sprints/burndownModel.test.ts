import { describe, expect, it } from 'vitest';

import type { ScopeTask, SprintReport } from '@/shared/api/types';

import { burndownOf, scopeChangeInWords } from './burndownModel';

/** Four days and ten points, two days of which have come. */
const REPORT: SprintReport = {
  sprintId: 'sprint',
  unit: 'points',
  total: 10,
  committed: 10,
  days: [
    { date: '2026-11-10', remaining: 10, scope: 10 },
    { date: '2026-11-11', remaining: 5, scope: 10 },
    { date: '2026-11-12', remaining: null, scope: null },
    { date: '2026-11-13', remaining: null, scope: null },
  ],
  changes: [],
  movedOn: [],
};

/** A task as a sprint held it. */
function held(number: number, work: number, hasNoPoints = false): ScopeTask {
  return {
    taskId: `task-${number.toString()}`,
    number,
    title: `Task ${number.toString()}`,
    work,
    hasNoPoints,
  };
}

const SIZE = { width: 200, height: 50 };

describe('the burndown chart', () => {
  it('draws the ideal line from all the work at the start to none at the end', () => {
    expect(burndownOf(REPORT, SIZE, '2026-11-11')?.ideal).toBe('M0 0L200 50');
  });

  it('draws what remained as steps that end with the last day that has come', () => {
    expect(burndownOf(REPORT, SIZE, '2026-11-11')?.actual).toBe('M0 0H50V0H100V25');
  });

  it('marks today at the end of its day, where the line ends', () => {
    const chart = burndownOf(REPORT, SIZE, '2026-11-11');

    expect(chart?.todayX).toBe(100);
    expect(chart?.end).toEqual({ x: 100, y: 25 });
  });

  it('has no mark of today outside the days of the sprint', () => {
    expect(burndownOf(REPORT, SIZE, '2026-11-20')?.todayX).toBeNull();
  });

  it('puts a tick at the beginning of every day and at the end of the last', () => {
    expect(burndownOf(REPORT, SIZE, '2026-11-11')?.dayTicks).toEqual([0, 50, 100, 150, 200]);
  });

  it('tells what remains now', () => {
    expect(burndownOf(REPORT, SIZE, '2026-11-11')?.remaining).toBe(5);
  });

  it('has no line of what remained before the first day has come', () => {
    const notYet = {
      ...REPORT,
      days: REPORT.days.map((day) => ({ ...day, remaining: null, scope: null })),
    };

    const chart = burndownOf(notYet, SIZE, '2026-11-09');

    expect(chart?.actual).toBeNull();
    expect(chart?.remaining).toBeNull();
  });

  it('lays a sprint without work along the bottom', () => {
    const empty = {
      ...REPORT,
      total: 0,
      committed: 0,
      days: [{ date: '2026-11-10', remaining: 0, scope: 0 }],
    };

    expect(burndownOf(empty, SIZE, '2026-11-10')).toMatchObject({
      ideal: 'M0 50L200 50',
      actual: 'M0 50H200V50',
    });
  });

  it('has no mark of today without a today, as for a sprint that is over', () => {
    const chart = burndownOf(REPORT, SIZE, null);

    expect(chart?.todayX).toBeNull();
    expect(chart?.actual).toBe('M0 0H50V0H100V25');
  });

  it('draws nothing for a report without days', () => {
    expect(burndownOf({ ...REPORT, days: [] }, SIZE, '2026-11-11')).toBeNull();
  });

  it('steps up on the day work joined, draws what the sprint held, and marks the day', () => {
    const grown: SprintReport = {
      ...REPORT,
      total: 15,
      days: [
        { date: '2026-11-10', remaining: 10, scope: 10 },
        { date: '2026-11-11', remaining: 15, scope: 15 },
        { date: '2026-11-12', remaining: null, scope: null },
        { date: '2026-11-13', remaining: null, scope: null },
      ],
      changes: [{ date: '2026-11-11', joined: [held(4, 5)], left: [], reestimated: 0 }],
    };

    const chart = burndownOf(grown, SIZE, '2026-11-11');

    // The top is the most the sprint held: 15 points.
    expect(chart?.ideal).toBe('M0 16.67L200 50');
    expect(chart?.actual).toBe('M0 16.67H50V16.67H100V0');
    expect(chart?.scope).toBe('M0 16.67H50V16.67H100V0');
    expect(chart?.marks).toMatchObject([{ x: 100, y: 0, direction: 'up' }]);
  });

  it('marks a day on which more left than joined as a step down', () => {
    const shrunk: SprintReport = {
      ...REPORT,
      days: [
        { date: '2026-11-10', remaining: 10, scope: 10 },
        { date: '2026-11-11', remaining: 7, scope: 7 },
        { date: '2026-11-12', remaining: null, scope: null },
        { date: '2026-11-13', remaining: null, scope: null },
      ],
      changes: [{ date: '2026-11-11', joined: [], left: [held(2, 3)], reestimated: 0 }],
    };

    expect(burndownOf(shrunk, SIZE, '2026-11-11')?.marks[0]?.direction).toBe('down');
  });

  it('draws no line of what the sprint held while nothing joined or left', () => {
    expect(burndownOf(REPORT, SIZE, '2026-11-11')?.scope).toBeNull();
  });
});

describe('what changed on a day of a sprint, in words', () => {
  it('counts the tasks and their points that joined and left', () => {
    expect(
      scopeChangeInWords(
        {
          date: '2026-11-11',
          joined: [held(4, 3), held(5, 2)],
          left: [held(6, 2)],
          reestimated: 0,
        },
        'points',
      ),
    ).toBe('2 tasks joined, 5 points; 1 task left, 2 points');
  });

  it('says how many had no points, and new points on tasks that stayed', () => {
    expect(
      scopeChangeInWords(
        { date: '2026-11-11', joined: [held(4, 0, true)], left: [], reestimated: -3 },
        'points',
      ),
    ).toBe('1 task joined, 0 points (1 without points); re-estimated, −3 points');
  });

  it('counts only tasks in a sprint counted in tasks', () => {
    expect(
      scopeChangeInWords(
        { date: '2026-11-11', joined: [held(4, 1)], left: [], reestimated: 0 },
        'tasks',
      ),
    ).toBe('1 task joined');
  });
});
