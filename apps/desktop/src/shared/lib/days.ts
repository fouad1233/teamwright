import type { CalendarDate } from '@/shared/api/types';

/**
 * Arithmetic on calendar dates.
 *
 * A calendar date has no time and no time zone, so it is counted as a whole
 * number: the days since 1 January 1970. Adding days, comparing dates and
 * finding the distance between two of them is then arithmetic on numbers,
 * and no clock change in spring or autumn can shift a date by a day.
 */

/** A calendar date as the number of days since 1 January 1970. */
export type DayNumber = number;

const MILLISECONDS_PER_DAY = 24 * 60 * 60 * 1000;
const DAYS_PER_WEEK = 7;

/** 1 January 1970 was a Thursday: three days after a Monday. */
const WEEKDAY_OF_DAY_ZERO = 3;

/** The number of a calendar date. */
export function dayNumberOf(date: CalendarDate): DayNumber {
  const [year = 1970, month = 1, day = 1] = date.split('-').map(Number);
  return Math.round(Date.UTC(year, month - 1, day) / MILLISECONDS_PER_DAY);
}

/** The calendar date with a number. */
export function calendarDateOfDay(day: DayNumber): CalendarDate {
  return new Date(day * MILLISECONDS_PER_DAY).toISOString().slice(0, 'YYYY-MM-DD'.length);
}

/** The calendar date that lies some days after another. A negative number goes back. */
export function addDays(date: CalendarDate, days: number): CalendarDate {
  return calendarDateOfDay(dayNumberOf(date) + days);
}

/** The day of the week of a day: `0` for a Monday up to `6` for a Sunday. */
export function weekdayOf(day: DayNumber): number {
  return (((day + WEEKDAY_OF_DAY_ZERO) % DAYS_PER_WEEK) + DAYS_PER_WEEK) % DAYS_PER_WEEK;
}

/** Whether a day is a Saturday or a Sunday. */
export function isWeekend(day: DayNumber): boolean {
  return weekdayOf(day) >= 5;
}

/** The Monday of the week a day is in. */
export function mondayOf(day: DayNumber): DayNumber {
  return day - weekdayOf(day);
}

/** The year, the month from 1 to 12, and the day of the month of a day. */
export function partsOfDay(day: DayNumber): { year: number; month: number; dayOfMonth: number } {
  const date = new Date(day * MILLISECONDS_PER_DAY);
  return {
    year: date.getUTCFullYear(),
    month: date.getUTCMonth() + 1,
    dayOfMonth: date.getUTCDate(),
  };
}

/** The first day of a month. `month` counts from 1 and may run past 12 or below 1. */
export function firstDayOfMonth(year: number, month: number): DayNumber {
  return Math.round(Date.UTC(year, month - 1, 1) / MILLISECONDS_PER_DAY);
}

/** The number of days of a month. `month` counts from 1. */
export function daysInMonth(year: number, month: number): number {
  return firstDayOfMonth(year, month + 1) - firstDayOfMonth(year, month);
}
