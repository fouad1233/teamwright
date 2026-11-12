import type { CalendarDate, Moment, Project, Task } from '@/shared/api/types';

const MINUTES_PER_HOUR = 60;

/**
 * The language and conventions dates are written in. The interface is in
 * English, so its dates are too, whatever the language of the computer: day
 * before month, and a 24-hour clock.
 */
const INTERFACE_LOCALE = 'en-GB';

/** The name a manager knows a task by, such as `IOT-12`. */
export function taskReference(project: Pick<Project, 'key'>, task: Pick<Task, 'number'>): string {
  return `${project.key}-${task.number.toString()}`;
}

/** The calendar date of a moment, where the user is. */
export function calendarDateOf(moment: Date): CalendarDate {
  const year = moment.getFullYear().toString().padStart(4, '0');
  const month = (moment.getMonth() + 1).toString().padStart(2, '0');
  const day = moment.getDate().toString().padStart(2, '0');
  return `${year}-${month}-${day}`;
}

/**
 * The start of a calendar date where the user is.
 *
 * A calendar date has no time zone. Building it from its parts keeps it on
 * the same day everywhere, which reading it as a moment would not.
 */
export function localDateOf(date: CalendarDate): Date {
  const [year = 0, month = 1, day = 1] = date.split('-').map(Number);
  return new Date(year, month - 1, day);
}

/** Today's calendar date where the user is. */
export function today(now: Date = new Date()): CalendarDate {
  return calendarDateOf(now);
}

/**
 * Writes a calendar date for people: "12 Nov", with the year added when it
 * is not the current one.
 */
export function formatDate(date: CalendarDate, now: Date = new Date()): string {
  const localDate = localDateOf(date);
  return new Intl.DateTimeFormat(INTERFACE_LOCALE, {
    month: 'short',
    day: 'numeric',
    year: localDate.getFullYear() === now.getFullYear() ? undefined : 'numeric',
  }).format(localDate);
}

/** Writes a month for people: "November 2026". `month` counts from 1. */
export function formatMonth(year: number, month: number): string {
  return new Intl.DateTimeFormat(INTERFACE_LOCALE, { month: 'long', year: 'numeric' }).format(
    new Date(year, month - 1, 1),
  );
}

/** Writes the short name of a month: "Nov". `month` counts from 1. */
export function formatMonthShort(year: number, month: number): string {
  return new Intl.DateTimeFormat(INTERFACE_LOCALE, { month: 'short' }).format(
    new Date(year, month - 1, 1),
  );
}

/** Writes a calendar date in full: "Thursday 12 November". */
export function formatDay(date: CalendarDate): string {
  return new Intl.DateTimeFormat(INTERFACE_LOCALE, {
    weekday: 'long',
    day: 'numeric',
    month: 'long',
  }).format(localDateOf(date));
}

/**
 * Says the dates of a task in a few words: "10 Nov – 14 Nov" for a task with
 * a start and a due date, and otherwise which of the two it has.
 */
export function formatPeriod(
  startOn: CalendarDate | null,
  dueOn: CalendarDate | null,
  now: Date = new Date(),
): string {
  if (startOn !== null && dueOn !== null) {
    return startOn === dueOn
      ? formatDate(startOn, now)
      : `${formatDate(startOn, now)} – ${formatDate(dueOn, now)}`;
  }
  if (startOn !== null) {
    return `Starts ${formatDate(startOn, now)}, no due date`;
  }
  if (dueOn !== null) {
    return `Due ${formatDate(dueOn, now)}`;
  }
  return 'No dates';
}

/** Writes a moment for people, in the user's time zone: "11 Nov 2026, 10:00". */
export function formatMoment(moment: Moment): string {
  return new Intl.DateTimeFormat(INTERFACE_LOCALE, {
    dateStyle: 'medium',
    timeStyle: 'short',
  }).format(new Date(moment));
}

/** Tells whether a task should have been finished before today and was not. */
export function isOverdue(
  task: Pick<Task, 'dueOn' | 'completedAt'>,
  todayDate: CalendarDate = today(),
): boolean {
  // Dates in the form year-month-day compare correctly as text.
  return task.completedAt === null && task.dueOn !== null && task.dueOn < todayDate;
}

/** Writes a working time for people: "45m", "3h", "2h 30m". */
export function formatEstimate(minutes: number): string {
  const hours = Math.floor(minutes / MINUTES_PER_HOUR);
  const remainingMinutes = minutes % MINUTES_PER_HOUR;
  if (hours === 0) {
    return `${remainingMinutes.toString()}m`;
  }
  if (remainingMinutes === 0) {
    return `${hours.toString()}h`;
  }
  return `${hours.toString()}h ${remainingMinutes.toString()}m`;
}

/**
 * Reads a working time the way people type it: "90", "90m", "1.5h", "2h",
 * "1h 30m". A bare number means minutes.
 *
 * Returns the minutes, `null` for empty text (no estimate), or `undefined`
 * if the text cannot be understood.
 */
export function parseEstimate(text: string): number | null | undefined {
  const trimmed = text.trim().toLowerCase();
  if (trimmed === '') {
    return null;
  }

  const match = /^(?:(\d+(?:\.\d+)?)\s*h)?\s*(?:(\d+)\s*m?)?$/.exec(trimmed);
  if (!match || (match[1] === undefined && match[2] === undefined)) {
    return undefined;
  }
  const hours = Number(match[1] ?? 0);
  const minutes = Number(match[2] ?? 0);
  const total = Math.round(hours * MINUTES_PER_HOUR + minutes);
  return total > 0 ? total : null;
}
