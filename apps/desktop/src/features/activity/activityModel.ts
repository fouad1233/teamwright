import type { CalendarDate, Id, Moment, Project, ProjectChange } from '@/shared/api/types';
import { addDays } from '@/shared/lib/days';
import { calendarDateOf, formatDay, taskReference } from '@/shared/lib/format';

import { changeAfterTaskName, type HistoryNames } from '../tasks/taskHistoryModel';

/**
 * Pure functions that turn the history of a project into the log the
 * Activity view shows: which changes it is narrowed to, the entries they
 * make, the days the entries fall on, and how far back the log has been
 * read.
 */

/** What the log can be narrowed to. */
export type Narrowing = 'everything' | 'opened' | 'moved' | 'finished' | 'planned';

/** The task a change is about, as the history of the project names it. */
export type LoggedTask = ProjectChange['task'];

/**
 * One entry of the log: what happened to one task within one minute, on
 * one computer. Most entries are one change. Several changes made in one
 * go, such as a title, a priority and dates set one after the other, are
 * one entry with a part for each.
 */
export interface LogEntry {
  /** Tells the entry apart from every other entry of the log. */
  key: string;
  task: LoggedTask;
  /** When the latest of its changes happened. */
  at: Moment;
  /** The time of day where the user is, on a 24-hour clock: "14:02". */
  time: string;
  /**
   * Whether the entry above it, on the same day, has the same time. A log
   * book writes a time once and leaves it out on the lines that share it.
   */
  repeatsTime: boolean;
  /** The other computer the changes were made on, when the history names it. */
  computer: string | undefined;
  /** The changes, in the order they happened. */
  changes: readonly ProjectChange[];
}

/** The entries of one day, the newest first. */
export interface LogDay {
  date: CalendarDate;
  entries: readonly LogEntry[];
}

const MILLISECONDS_PER_MINUTE = 60 * 1000;

/**
 * Whether the log shows a change while it is narrowed.
 *
 * "Finished" means that the task went into a status of the category done
 * from one that is not, whatever the statuses are called. "Planned" is
 * everything that says when and in what order the work is to be done: the
 * dates, the estimate, the sprint, the points, and what the task waits for.
 */
export function isShownBy(narrowing: Narrowing, change: ProjectChange, project: Project): boolean {
  switch (narrowing) {
    case 'everything':
      return true;
    case 'opened':
      return change.kind === 'created';
    case 'moved':
      return change.kind === 'moved';
    case 'finished':
      return (
        change.kind === 'moved' &&
        isDone(project, change.toStatusId) &&
        !isDone(project, change.fromStatusId)
      );
    case 'planned':
      return (
        change.kind === 'rescheduled' ||
        change.kind === 'estimated' ||
        change.kind === 'planned' ||
        change.kind === 'pointed' ||
        change.kind === 'now_waits_for' ||
        change.kind === 'no_longer_waits_for'
      );
  }
}

/** Whether a status means that the work is done. A status that was removed says nothing. */
function isDone(project: Project, statusId: Id): boolean {
  return project.statuses.find((status) => status.id === statusId)?.category === 'done';
}

/**
 * The changes the log shows, of those that are loaded: the ones it is
 * narrowed to, and, while one area of the project is open, only those of
 * the tasks of that area. `taskIds` are the tasks of the open area, or
 * nothing when the whole project is shown. A task that was deleted is in
 * no area any more, so an open area leaves it out.
 */
export function changesToShow(
  changes: readonly ProjectChange[],
  narrowing: Narrowing,
  project: Project,
  taskIds: ReadonlySet<Id> | null,
): ProjectChange[] {
  return changes.filter(
    (change) =>
      (taskIds === null || taskIds.has(change.task.id)) && isShownBy(narrowing, change, project),
  );
}

/**
 * The entries of the log, the newest first, from changes in the order the
 * history gives them (the newest first).
 *
 * The changes made to one task within one minute of the clock, on one
 * computer, are one entry, even when a change of another task lies between
 * them. The entry stands where its latest change stands. Entries of the
 * very same moment, such as tasks that were opened together, follow the
 * numbers of their tasks.
 */
export function logEntries(changes: readonly ProjectChange[]): LogEntry[] {
  /** The changes of each entry while they are gathered, the newest first. */
  const gathered = new Map<string, ProjectChange[]>();
  for (const change of changes) {
    const key = [minuteOf(change.at).toString(), change.task.id, change.computer ?? ''].join('|');
    const sameEntry = gathered.get(key);
    if (sameEntry) {
      sameEntry.push(change);
    } else {
      gathered.set(key, [change]);
    }
  }

  const entries = [...gathered].flatMap(([key, newestFirst]) => {
    const latest = newestFirst[0];
    return latest === undefined
      ? []
      : {
          key,
          task: latest.task,
          at: latest.at,
          time: timeOf(latest.at),
          computer: latest.computer,
          // The history tells the newest change first. An entry tells its
          // parts in the order they happened.
          changes: [...newestFirst].reverse(),
        };
  });
  entries.sort(
    (one, other) =>
      Date.parse(other.at) - Date.parse(one.at) || one.task.number - other.task.number,
  );

  return entries.map((entry, place) => {
    const above = entries[place - 1];
    return {
      ...entry,
      repeatsTime:
        above !== undefined && above.time === entry.time && dayOf(above.at) === dayOf(entry.at),
    };
  });
}

/** The entries of the log by the day they happened on, where the user is, the newest day first. */
export function logDays(entries: readonly LogEntry[]): LogDay[] {
  const days: { date: CalendarDate; entries: LogEntry[] }[] = [];
  for (const entry of entries) {
    const date = dayOf(entry.at);
    const day = days.at(-1);
    if (day?.date === date) {
      day.entries.push(entry);
    } else {
      days.push({ date, entries: [entry] });
    }
  }
  return days;
}

/**
 * An entry of the log as one sentence, for a place that has no room for
 * its parts on lines of their own, such as the latest activity on the
 * dashboard: the title of the task and what happened to it, in the words
 * of the history of a task. "Draw the schematic was opened, was given to
 * you and had its estimate set to 3h."
 */
export function entryInWords(entry: Pick<LogEntry, 'task' | 'changes'>, names: HistoryNames) {
  const parts = entry.changes.map((change) => changeAfterTaskName(change, names));
  const last = parts.pop();
  const said = parts.length === 0 ? (last ?? '') : `${parts.join(', ')} and ${last ?? ''}`;
  return `${entry.task.title} ${said}.`;
}

/**
 * When an entry happened, said plainly for a short list: "Today, 14:02",
 * "Yesterday, 09:10", "Thursday 12 November, 16:45". The day is called as
 * the log heads it (`dayHeading`).
 */
export function whenInWords(entry: Pick<LogEntry, 'at' | 'time'>, today: CalendarDate): string {
  return `${dayHeading(dayOf(entry.at), today).name}, ${entry.time}`;
}

/** What a day is called at the head of its entries. */
export interface DayHeading {
  /** "Today", "Yesterday", or the date in full: "Thursday 12 November". */
  name: string;
  /** The date in full, beside a day that is called by a word. Nothing beside a date. */
  date: string | null;
}

/**
 * What a day of the log is called. The last two days are called by a word,
 * with their date beside it. A day of another year says its year.
 */
export function dayHeading(date: CalendarDate, today: CalendarDate): DayHeading {
  const inFull = dateInFull(date, today);
  if (date === today) {
    return { name: 'Today', date: inFull };
  }
  if (date === addDays(today, -1)) {
    return { name: 'Yesterday', date: inFull };
  }
  return { name: inFull, date: null };
}

/** A date for a heading: "Thursday 12 November", with the year if it is not this one. */
function dateInFull(date: CalendarDate, today: CalendarDate): string {
  const year = yearOf(date);
  return year === yearOf(today) ? formatDay(date) : `${formatDay(date)} ${year}`;
}

/** The pages of the history loaded so far, the newest first, each the newest change first. */
export type HistoryPages = readonly (readonly ProjectChange[])[];

/** The moment of the oldest change on some pages, or nothing if they hold none. */
export function oldestMomentOf(pages: HistoryPages): Moment | null {
  return pages.flat().at(-1)?.at ?? null;
}

/** How far back the reader wants the log. */
export interface ReadingBack {
  /**
   * The moment the log was read back to when it was last shown in full.
   * Nothing before the first reading.
   */
  to: Moment | null;
  /** Whether the reader has asked for what is earlier than that. */
  andEarlier: boolean;
}

/**
 * Whether the pages loaded so far reach as far back as the reader wants.
 * If not, the next page is to be asked for. `hasMore` says whether a page
 * may follow the last one, and `fullPage` how many changes a page is asked
 * for.
 *
 * The history is asked for again from its first page whenever a task
 * changes. The reading then goes on until it is back where the reader was,
 * so that the log does not lose its end under the reader's eyes.
 *
 * A page shorter than a full one does not end the reading either: the
 * history cuts a page short rather than split the changes of one moment,
 * and the rest of that moment is on the next page. Only a full page, or
 * the beginning of the history, is a place to stop.
 */
export function isReadFarEnough(
  pages: HistoryPages,
  hasMore: boolean,
  fullPage: number,
  back: ReadingBack,
): boolean {
  if (!hasMore) {
    return true;
  }
  const oldest = oldestMomentOf(pages);
  if (oldest === null || (pages.at(-1)?.length ?? 0) < fullPage) {
    return false;
  }
  if (back.to === null) {
    return true;
  }
  const beyond = Date.parse(back.to) - Date.parse(oldest);
  return back.andEarlier ? beyond > 0 : beyond >= 0;
}

/**
 * How many characters the longest key of a task has, so that the keys of
 * the log stand in one column: six for "SSN-21". `numbers` are the numbers
 * of the tasks the log may name.
 */
export function longestKey(project: Pick<Project, 'key'>, numbers: Iterable<number>): number {
  let highest = 0;
  for (const number of numbers) {
    highest = Math.max(highest, number);
  }
  return taskReference(project, { number: highest }).length;
}

/**
 * Says in a sentence that the log has nothing to show, once the whole
 * history has been read. Narrowed to everything, that can only be said of
 * one area of a project: a project without any history has no log yet.
 */
export function nothingInWords(narrowing: Narrowing): string {
  switch (narrowing) {
    case 'everything':
      return 'Nothing has happened to the tasks of this area yet.';
    case 'opened':
      return 'No task has been opened yet.';
    case 'moved':
      return 'No task has moved to another status yet.';
    case 'finished':
      return 'No task has been finished yet.';
    case 'planned':
      // A task that was opened with dates is planned, and its opening does
      // not say so. What can be said for certain is that no plan changed.
      return 'The plan has not changed yet: no dates, no estimate, nothing a task waits for.';
  }
}

/** The minute of the clock a moment lies in, counted from 1970. */
function minuteOf(moment: Moment): number {
  return Math.floor(Date.parse(moment) / MILLISECONDS_PER_MINUTE);
}

/** The calendar date of a moment where the user is. */
function dayOf(moment: Moment): CalendarDate {
  return calendarDateOf(new Date(moment));
}

function yearOf(date: CalendarDate): string {
  return date.slice(0, 'YYYY'.length);
}

/** The time of day of a moment where the user is, on a 24-hour clock: "09:05". */
function timeOf(moment: Moment): string {
  const date = new Date(moment);
  const twoDigits = (value: number) => value.toString().padStart(2, '0');
  return `${twoDigits(date.getHours())}:${twoDigits(date.getMinutes())}`;
}
