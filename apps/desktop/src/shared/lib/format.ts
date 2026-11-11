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

/** Today's calendar date where the user is. */
export function today(now: Date = new Date()): CalendarDate {
  const year = now.getFullYear().toString().padStart(4, '0');
  const month = (now.getMonth() + 1).toString().padStart(2, '0');
  const day = now.getDate().toString().padStart(2, '0');
  return `${year}-${month}-${day}`;
}

/**
 * Writes a calendar date for people: "12 Nov", with the year added when it
 * is not the current one.
 */
export function formatDate(date: CalendarDate, now: Date = new Date()): string {
  // A calendar date has no time zone. Building it from its parts keeps it on
  // the same day everywhere, which parsing it as a moment would not.
  const [year = 0, month = 1, day = 1] = date.split('-').map(Number);
  const localDate = new Date(year, month - 1, day);
  return new Intl.DateTimeFormat(INTERFACE_LOCALE, {
    month: 'short',
    day: 'numeric',
    year: year === now.getFullYear() ? undefined : 'numeric',
  }).format(localDate);
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
