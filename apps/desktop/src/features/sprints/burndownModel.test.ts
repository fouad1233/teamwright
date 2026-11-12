import { describe, expect, it } from 'vitest';

import type { SprintReport } from '@/shared/api/types';

import { burndownOf } from './burndownModel';

/** Four days and ten points, two days of which have come. */
const REPORT: SprintReport = {
  sprintId: 'sprint',
  unit: 'points',
  total: 10,
  days: [
    { date: '2026-11-10', remaining: 10 },
    { date: '2026-11-11', remaining: 5 },
    { date: '2026-11-12', remaining: null },
    { date: '2026-11-13', remaining: null },
  ],
};

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
      days: REPORT.days.map((day) => ({ ...day, remaining: null })),
    };

    const chart = burndownOf(notYet, SIZE, '2026-11-09');

    expect(chart?.actual).toBeNull();
    expect(chart?.remaining).toBeNull();
  });

  it('lays a sprint without work along the bottom', () => {
    const empty = { ...REPORT, total: 0, days: [{ date: '2026-11-10', remaining: 0 }] };

    expect(burndownOf(empty, SIZE, '2026-11-10')).toMatchObject({
      ideal: 'M0 50L200 50',
      actual: 'M0 50H200V50',
    });
  });

  it('draws nothing for a report without days', () => {
    expect(burndownOf({ ...REPORT, days: [] }, SIZE, '2026-11-11')).toBeNull();
  });
});
