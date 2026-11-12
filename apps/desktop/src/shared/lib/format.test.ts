import { describe, expect, it } from 'vitest';

import {
  formatDate,
  formatDay,
  formatEstimate,
  formatMonth,
  formatMonthShort,
  formatPeriod,
  isOverdue,
  parseEstimate,
  taskReference,
  today,
} from './format';

describe('taskReference', () => {
  it('joins the project key and the task number', () => {
    expect(taskReference({ key: 'IOT' }, { number: 12 })).toBe('IOT-12');
  });
});

describe('today', () => {
  it('writes the local date as year-month-day with leading zeros', () => {
    expect(today(new Date(2026, 2, 9, 23, 59))).toBe('2026-03-09');
  });
});

describe('formatDate', () => {
  const inNovember2026 = new Date(2026, 10, 11);

  it('writes a date of the current year as day and month', () => {
    expect(formatDate('2026-11-17', inNovember2026)).toBe('17 Nov');
  });

  it('adds the year to a date of another year', () => {
    expect(formatDate('2027-01-05', inNovember2026)).toBe('5 Jan 2027');
  });
});

describe('formatMonth and formatMonthShort', () => {
  it('write a month with its year, or as a short name', () => {
    expect(formatMonth(2026, 11)).toBe('November 2026');
    expect(formatMonthShort(2026, 12)).toBe('Dec');
  });
});

describe('formatDay', () => {
  it('writes a date in full, with the day of the week', () => {
    expect(formatDay('2026-11-12')).toBe('Thursday 12 November');
  });
});

describe('formatPeriod', () => {
  const inNovember2026 = new Date(2026, 10, 11);

  it('writes both dates of a planned task', () => {
    expect(formatPeriod('2026-11-10', '2026-11-14', inNovember2026)).toBe('10 Nov – 14 Nov');
  });

  it('writes one date when the task starts and ends on the same day', () => {
    expect(formatPeriod('2026-11-10', '2026-11-10', inNovember2026)).toBe('10 Nov');
  });

  it('says which date a task with one date has', () => {
    expect(formatPeriod('2026-11-10', null, inNovember2026)).toBe('Starts 10 Nov, no due date');
    expect(formatPeriod(null, '2026-11-14', inNovember2026)).toBe('Due 14 Nov');
  });

  it('says so when a task has no dates', () => {
    expect(formatPeriod(null, null)).toBe('No dates');
  });
});

describe('isOverdue', () => {
  it('is true for an unfinished task that was due before today', () => {
    expect(isOverdue({ dueOn: '2026-11-10', completedAt: null }, '2026-11-11')).toBe(true);
  });

  it('is false on the due date itself', () => {
    expect(isOverdue({ dueOn: '2026-11-11', completedAt: null }, '2026-11-11')).toBe(false);
  });

  it('is false for a finished task', () => {
    const finished = { dueOn: '2026-11-10', completedAt: '2026-11-12T08:00:00.000Z' };

    expect(isOverdue(finished, '2026-11-11')).toBe(false);
  });

  it('is false for a task without a due date', () => {
    expect(isOverdue({ dueOn: null, completedAt: null }, '2026-11-11')).toBe(false);
  });
});

describe('formatEstimate', () => {
  it('writes minutes below an hour as minutes', () => {
    expect(formatEstimate(45)).toBe('45m');
  });

  it('writes whole hours without minutes', () => {
    expect(formatEstimate(180)).toBe('3h');
  });

  it('writes hours and minutes together', () => {
    expect(formatEstimate(150)).toBe('2h 30m');
  });
});

describe('parseEstimate', () => {
  it('reads a bare number as minutes', () => {
    expect(parseEstimate('90')).toBe(90);
  });

  it('reads minutes, hours and both', () => {
    expect(parseEstimate('45m')).toBe(45);
    expect(parseEstimate('2h')).toBe(120);
    expect(parseEstimate('1h 30m')).toBe(90);
    expect(parseEstimate('1h30')).toBe(90);
  });

  it('reads fractions of an hour', () => {
    expect(parseEstimate('1.5h')).toBe(90);
  });

  it('ignores case and surrounding space', () => {
    expect(parseEstimate('  2H 15M ')).toBe(135);
  });

  it('reads empty text and zero as no estimate', () => {
    expect(parseEstimate('   ')).toBeNull();
    expect(parseEstimate('0')).toBeNull();
  });

  it('does not understand anything else', () => {
    expect(parseEstimate('soon')).toBeUndefined();
    expect(parseEstimate('2 days')).toBeUndefined();
  });

  it('reads back what formatEstimate writes', () => {
    for (const minutes of [5, 45, 60, 150, 600]) {
      expect(parseEstimate(formatEstimate(minutes))).toBe(minutes);
    }
  });
});
