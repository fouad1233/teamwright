import { describe, expect, it } from 'vitest';

import {
  addDays,
  calendarDateOfDay,
  dayNumberOf,
  daysInMonth,
  firstDayOfMonth,
  isWeekend,
  mondayOf,
  partsOfDay,
  weekdayOf,
} from './days';

describe('dayNumberOf and calendarDateOfDay', () => {
  it('count the days since 1 January 1970', () => {
    expect(dayNumberOf('1970-01-01')).toBe(0);
    expect(dayNumberOf('1970-01-02')).toBe(1);
  });

  it('undo each other', () => {
    for (const date of ['2026-11-12', '2024-02-29', '1999-12-31', '2027-01-01']) {
      expect(calendarDateOfDay(dayNumberOf(date))).toBe(date);
    }
  });

  it('put consecutive dates one apart across the change of the clocks', () => {
    // In many places the clocks go back in the last days of October.
    expect(dayNumberOf('2026-10-26') - dayNumberOf('2026-10-25')).toBe(1);
    expect(dayNumberOf('2026-03-30') - dayNumberOf('2026-03-29')).toBe(1);
  });
});

describe('addDays', () => {
  it('goes forward across the end of a month and of a year', () => {
    expect(addDays('2026-11-28', 5)).toBe('2026-12-03');
    expect(addDays('2026-12-31', 1)).toBe('2027-01-01');
  });

  it('goes back with a negative number', () => {
    expect(addDays('2026-03-01', -1)).toBe('2026-02-28');
  });
});

describe('weekdayOf', () => {
  it('is 0 for a Monday and 6 for a Sunday', () => {
    // 9 November 2026 is a Monday.
    expect(weekdayOf(dayNumberOf('2026-11-09'))).toBe(0);
    expect(weekdayOf(dayNumberOf('2026-11-12'))).toBe(3);
    expect(weekdayOf(dayNumberOf('2026-11-15'))).toBe(6);
  });

  it('also works before 1970', () => {
    // 29 December 1969 was a Monday.
    expect(weekdayOf(dayNumberOf('1969-12-29'))).toBe(0);
  });
});

describe('isWeekend', () => {
  it('is true for Saturday and Sunday only', () => {
    const friday = dayNumberOf('2026-11-13');

    expect([0, 1, 2, 3].map((offset) => isWeekend(friday + offset))).toEqual([
      false,
      true,
      true,
      false,
    ]);
  });
});

describe('mondayOf', () => {
  it('is the Monday on or before a day', () => {
    expect(calendarDateOfDay(mondayOf(dayNumberOf('2026-11-12')))).toBe('2026-11-09');
    expect(calendarDateOfDay(mondayOf(dayNumberOf('2026-11-09')))).toBe('2026-11-09');
    expect(calendarDateOfDay(mondayOf(dayNumberOf('2026-11-15')))).toBe('2026-11-09');
  });
});

describe('partsOfDay', () => {
  it('names the year, the month and the day of the month', () => {
    expect(partsOfDay(dayNumberOf('2026-11-12'))).toEqual({
      year: 2026,
      month: 11,
      dayOfMonth: 12,
    });
  });
});

describe('firstDayOfMonth and daysInMonth', () => {
  it('know the length of every kind of month', () => {
    expect(daysInMonth(2026, 11)).toBe(30);
    expect(daysInMonth(2026, 12)).toBe(31);
    expect(daysInMonth(2026, 2)).toBe(28);
    expect(daysInMonth(2024, 2)).toBe(29);
  });

  it('accept a month past December as a month of the next year', () => {
    expect(calendarDateOfDay(firstDayOfMonth(2026, 13))).toBe('2027-01-01');
    expect(calendarDateOfDay(firstDayOfMonth(2026, 0))).toBe('2025-12-01');
  });
});
