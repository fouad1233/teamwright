import type { Assignee, Id, Member, Project, Task } from '@/shared/api/types';
import { dayNumberOf, isWeekend, mondayOf, type DayNumber } from '@/shared/lib/days';

/**
 * The arithmetic of the workload: which days a task's estimate is spread
 * over, how many hours of it fall into a week or a day, what a person can
 * take in that time, and what the view keeps of its own.
 *
 * Everything here is a pure function of the tasks, the team, today and the
 * content of the view, so that it can be tested without an interface.
 */

/** How many weeks the view shows, week by week. */
export const WEEKS_SHOWN = 8;

/** How many weeks the view shows when it shows each day. */
export const WEEKS_SHOWN_BY_DAY = 4;

/** The hours a day a person can work until the manager says otherwise. */
export const DEFAULT_HOURS_PER_DAY = 6;

/** The fewest and the most hours a day the manager can give a person. */
export const LEAST_HOURS_PER_DAY = 0.5;
export const MOST_HOURS_PER_DAY = 24;

const DAYS_PER_WEEK = 7;
const MINUTES_PER_HOUR = 60;

/** Whether the columns are weeks or days. */
export type Scale = 'weeks' | 'days';

/** What the view keeps of its own, in `View.content`. */
export interface WorkloadContent {
  scale: Scale;
  /**
   * The hours a day each person can work, by the key of their row
   * (`manager`, or `member:<id>` for a person on the team). A person who is
   * not here can work the default.
   */
  hoursPerDay: Record<string, number>;
}

/** The content of a view that has never been changed. */
export const EMPTY_CONTENT: WorkloadContent = { scale: 'weeks', hoursPerDay: {} };

/**
 * Reads what the view keeps. The content may be missing, empty, not JSON,
 * or written by an older or newer version: whatever cannot be understood is
 * left out, and nothing is ever thrown.
 */
export function parseContent(text: string | null): WorkloadContent {
  if (text === null || text.trim() === '') {
    return EMPTY_CONTENT;
  }
  let raw: unknown;
  try {
    raw = JSON.parse(text);
  } catch {
    return EMPTY_CONTENT;
  }
  if (typeof raw !== 'object' || raw === null) {
    return EMPTY_CONTENT;
  }
  const record = raw as Record<string, unknown>;
  const scale: Scale = record.scale === 'days' ? 'days' : 'weeks';
  const hoursPerDay: Record<string, number> = {};
  const given = record.hoursPerDay;
  if (typeof given === 'object' && given !== null) {
    for (const [key, value] of Object.entries(given as Record<string, unknown>)) {
      if (typeof value === 'number' && isHoursPerDay(value)) {
        hoursPerDay[key] = value;
      }
    }
  }
  return { scale, hoursPerDay };
}

/** Writes what the view keeps. */
export function serializeContent(content: WorkloadContent): string {
  return JSON.stringify(content);
}

/** Whether a number of hours a day can be given to a person. */
export function isHoursPerDay(hours: number): boolean {
  return Number.isFinite(hours) && hours >= LEAST_HOURS_PER_DAY && hours <= MOST_HOURS_PER_DAY;
}

/**
 * Reads the hours a day the manager typed: "6", "7.5", "7,5", "6h". Returns
 * nothing for anything else, or for a number out of bounds.
 */
export function parseHoursPerDay(text: string): number | undefined {
  const match = /^\s*(\d+(?:[.,]\d+)?)\s*h?\s*$/i.exec(text);
  if (!match?.[1]) {
    return undefined;
  }
  const hours = Math.round(Number(match[1].replace(',', '.')) * 4) / 4;
  return isHoursPerDay(hours) ? hours : undefined;
}

/**
 * Writes a number of minutes as hours, to a tenth: "6", "7.5", "0.3". A
 * little work never reads as nothing.
 */
export function formatHours(minutes: number): string {
  if (minutes <= 0) {
    return '0';
  }
  const tenths = Math.max(Math.round((minutes / MINUTES_PER_HOUR) * 10), 1);
  return (tenths / 10).toString();
}

/** One column of the table: a week, or a day. */
export interface Column {
  key: string;
  kind: 'week' | 'day';
  firstDay: DayNumber;
  lastDay: DayNumber;
  /** For a day: whether it is a Saturday or a Sunday. */
  isRestDay: boolean;
  /** Whether the column holds today. */
  holdsToday: boolean;
  /** Whether every day of the column is over. */
  isPast: boolean;
}

/**
 * The first day work can be done: today, or the Monday after it when today
 * is a Saturday or a Sunday.
 */
export function nextWorkingDay(today: DayNumber): DayNumber {
  let day = today;
  while (isWeekend(day)) {
    day += 1;
  }
  return day;
}

/**
 * The columns of a scale, from the Monday of the week in which work can
 * next be done. The days before that are over.
 */
export function columnsOf(scale: Scale, today: DayNumber): Column[] {
  const start = nextWorkingDay(today);
  const monday = mondayOf(start);
  if (scale === 'weeks') {
    return Array.from({ length: WEEKS_SHOWN }, (_, index) => {
      const firstDay = monday + index * DAYS_PER_WEEK;
      const lastDay = firstDay + DAYS_PER_WEEK - 1;
      return {
        key: `week:${firstDay.toString()}`,
        kind: 'week',
        firstDay,
        lastDay,
        isRestDay: false,
        holdsToday: firstDay <= today && today <= lastDay,
        isPast: lastDay < start,
      };
    });
  }
  return Array.from({ length: WEEKS_SHOWN_BY_DAY * DAYS_PER_WEEK }, (_, index) => {
    const day = monday + index;
    return {
      key: `day:${day.toString()}`,
      kind: 'day',
      firstDay: day,
      lastDay: day,
      isRestDay: isWeekend(day),
      holdsToday: day === today,
      isPast: day < start,
    };
  });
}

/** Why a task cannot be laid on the days. */
export type Unlaid = 'no estimate' | 'no due date';

/** How a task's estimate lies on the days. */
export interface Laying {
  /** The days the estimate is spread over, evenly. */
  days: DayNumber[];
  /** The minutes that fall on each of those days. */
  minutesPerDay: number;
  /** Whether the task was due before the first day, so that all of it lies on that day. */
  isLate: boolean;
}

/**
 * Spreads the estimate of a task over its working days: from its start, or
 * from `today` (the first day work can be done) if it starts earlier or has
 * no start, up to its due date. Work that is not finished cannot be done in
 * the past, so a task that is late lies whole on that first day. Saturdays
 * and Sundays carry nothing, unless the task lies on nothing else.
 */
export function layingOf(
  task: Pick<Task, 'estimateMinutes' | 'startOn' | 'dueOn'>,
  today: DayNumber,
): Laying | Unlaid {
  if (task.estimateMinutes === null || task.estimateMinutes <= 0) {
    return 'no estimate';
  }
  if (task.dueOn === null) {
    return 'no due date';
  }
  const lastDay = dayNumberOf(task.dueOn);
  if (lastDay < today) {
    return { days: [today], minutesPerDay: task.estimateMinutes, isLate: true };
  }
  const firstDay = Math.max(task.startOn === null ? today : dayNumberOf(task.startOn), today);
  const everyDay = Array.from({ length: lastDay - firstDay + 1 }, (_, index) => firstDay + index);
  const workingDays = everyDay.filter((day) => !isWeekend(day));
  const days = workingDays.length > 0 ? workingDays : everyDay;
  return { days, minutesPerDay: task.estimateMinutes / days.length, isLate: false };
}

/** Who or what a row stands for. */
export type RowSubject =
  | { kind: 'manager' }
  | { kind: 'member'; member: Member }
  | { kind: 'nobody' }
  /** Tasks given to someone who is no longer on the team, so that none is lost. */
  | { kind: 'other' };

/** The part of a task that falls into a cell. */
export interface Share {
  task: Task;
  minutes: number;
  isLate: boolean;
}

/** One cell: the hours of a person in a week or a day. */
export interface Cell {
  column: Column;
  minutes: number;
  /** What the person can take in the cell, in minutes. Nothing for someone without a working day. */
  capacityMinutes: number | null;
  /** The tasks behind the figure, the largest share first. */
  shares: Share[];
}

/** One row of the table. */
export interface WorkloadRow {
  key: string;
  subject: RowSubject;
  /** The hours a day the person can work. Nothing for a specialist, for nobody, and for the rest. */
  hoursPerDay: number | null;
  cells: Cell[];
  /** The minutes of all the cells. */
  totalMinutes: number;
  /** What the person can take in all the cells. */
  totalCapacityMinutes: number | null;
}

/** The whole table. */
export interface Workload {
  columns: Column[];
  rows: WorkloadRow[];
  /** The tasks that cannot be laid on the days, in the order of their numbers. */
  unlaid: { task: Task; reason: Unlaid }[];
}

/** What the table is made from besides the tasks. */
export interface WorkloadSources {
  project: Pick<Project, 'statuses'>;
  /** The team, in the order its members joined. */
  members: readonly Member[];
  /** Every task of the project, to know which tasks have parts of their own. */
  everyTask: readonly Task[];
  content: WorkloadContent;
  today: DayNumber;
}

/** The key of the row of an assignee. */
export function rowKeyOf(assignee: Assignee | null): string {
  if (assignee === null) {
    return 'nobody';
  }
  return assignee.kind === 'manager' ? 'manager' : `member:${assignee.memberId}`;
}

/** The hours a day a row can work, or nothing for a row without a working day. */
function hoursPerDayOf(key: string, subject: RowSubject, content: WorkloadContent): number | null {
  const hasWorkingDay =
    subject.kind === 'manager' || (subject.kind === 'member' && subject.member.kind === 'human');
  return hasWorkingDay ? (content.hoursPerDay[key] ?? DEFAULT_HOURS_PER_DAY) : null;
}

/** The working days of a column that are not over. */
function workingDaysLeft(column: Column, today: DayNumber): number {
  let count = 0;
  for (let day = Math.max(column.firstDay, today); day <= column.lastDay; day += 1) {
    if (!isWeekend(day)) {
      count += 1;
    }
  }
  return count;
}

/**
 * The tasks that are counted. A finished task lies on nobody. A task whose
 * parts carry estimates of their own is counted through its parts, so that
 * the same work is not counted twice.
 */
function countedTasks(tasks: readonly Task[], sources: WorkloadSources): Task[] {
  const finished = new Set(
    sources.project.statuses
      .filter((status) => status.category === 'done')
      .map((status) => status.id),
  );
  const splitIntoParts = new Set<Id>();
  for (const task of sources.everyTask) {
    if (task.parentId !== null && task.estimateMinutes !== null && task.estimateMinutes > 0) {
      splitIntoParts.add(task.parentId);
    }
  }
  return tasks.filter((task) => !finished.has(task.statusId) && !splitIntoParts.has(task.id));
}

/**
 * The workload: a row for the manager, one for every member of the team in
 * the order they joined, then one for the tasks given to nobody and one for
 * tasks given to someone who has left, each only when it has work. A column
 * for every week or day from the Monday of this week on.
 */
export function workloadOf(tasks: readonly Task[], sources: WorkloadSources): Workload {
  const { content } = sources;
  const columns = columnsOf(content.scale, sources.today);
  // On a Saturday or a Sunday, the work that waits begins on Monday.
  const today = nextWorkingDay(sources.today);
  const subjects: [string, RowSubject][] = [
    ['manager', { kind: 'manager' }],
    ...sources.members.map((member): [string, RowSubject] => [
      `member:${member.id}`,
      { kind: 'member', member },
    ]),
  ];
  const known = new Set(subjects.map(([key]) => key));

  const laidByKey = new Map<string, { task: Task; laying: Laying }[]>();
  const unlaid: Workload['unlaid'] = [];
  for (const task of countedTasks(tasks, sources)) {
    const laying = layingOf(task, today);
    if (typeof laying === 'string') {
      unlaid.push({ task, reason: laying });
      continue;
    }
    const ownKey = rowKeyOf(task.assignee);
    const key = ownKey === 'nobody' || known.has(ownKey) ? ownKey : 'other';
    laidByKey.set(key, [...(laidByKey.get(key) ?? []), { task, laying }]);
  }
  unlaid.sort((first, second) => first.task.number - second.task.number);

  const allSubjects: [string, RowSubject][] = [
    ...subjects,
    ...(laidByKey.has('nobody') ? [['nobody', { kind: 'nobody' }] as [string, RowSubject]] : []),
    ...(laidByKey.has('other') ? [['other', { kind: 'other' }] as [string, RowSubject]] : []),
  ];

  const rows = allSubjects.map(([key, subject]): WorkloadRow => {
    const hoursPerDay = hoursPerDayOf(key, subject, content);
    const laid = laidByKey.get(key) ?? [];
    const cells = columns.map((column): Cell => {
      const shares: Share[] = [];
      for (const { task, laying } of laid) {
        const daysInside = laying.days.filter(
          (day) => day >= column.firstDay && day <= column.lastDay,
        ).length;
        if (daysInside > 0) {
          shares.push({ task, minutes: daysInside * laying.minutesPerDay, isLate: laying.isLate });
        }
      }
      shares.sort(
        (first, second) => second.minutes - first.minutes || first.task.number - second.task.number,
      );
      return {
        column,
        minutes: shares.reduce((sum, share) => sum + share.minutes, 0),
        capacityMinutes:
          hoursPerDay === null
            ? null
            : workingDaysLeft(column, today) * hoursPerDay * MINUTES_PER_HOUR,
        shares,
      };
    });
    return {
      key,
      subject,
      hoursPerDay,
      cells,
      totalMinutes: cells.reduce((sum, cell) => sum + cell.minutes, 0),
      totalCapacityMinutes:
        hoursPerDay === null
          ? null
          : cells.reduce((sum, cell) => sum + (cell.capacityMinutes ?? 0), 0),
    };
  });

  return { columns, rows, unlaid };
}

/** How full a cell is, from 0, for its bar. Above 1 the person has more than they can take. */
export function fillOf(cell: Pick<Cell, 'minutes' | 'capacityMinutes'>): number {
  if (cell.capacityMinutes === null) {
    return 0;
  }
  if (cell.capacityMinutes === 0) {
    return cell.minutes > 0 ? Infinity : 0;
  }
  return cell.minutes / cell.capacityMinutes;
}

/**
 * The most minutes in any cell of the rows without a working day: the
 * measure their bars are drawn against, since they have no limit of their own.
 */
export function largestUnboundedCell(rows: readonly WorkloadRow[]): number {
  let largest = 0;
  for (const row of rows) {
    if (row.hoursPerDay === null) {
      for (const cell of row.cells) {
        largest = Math.max(largest, cell.minutes);
      }
    }
  }
  return largest;
}

/**
 * The sentence under the table about the tasks that cannot be laid on the
 * days: "7 tasks cannot be laid on the days: 4 have no estimate, 3 no due
 * date." Nothing when every task can be laid.
 */
export function unlaidSentence(unlaid: readonly { reason: Unlaid }[]): string | null {
  if (unlaid.length === 0) {
    return null;
  }
  const noEstimate = unlaid.filter(({ reason }) => reason === 'no estimate').length;
  const noDueDate = unlaid.length - noEstimate;
  const lead =
    unlaid.length === 1
      ? '1 task cannot be laid on the days'
      : `${unlaid.length.toString()} tasks cannot be laid on the days`;
  const has = (count: number) => (count === 1 ? 'has' : 'have');
  const reasons =
    noEstimate > 0 && noDueDate > 0
      ? `${noEstimate.toString()} ${has(noEstimate)} no estimate, ${noDueDate.toString()} no due date`
      : noEstimate > 0
        ? unlaid.length === 1
          ? 'it has no estimate'
          : `none of them have an estimate`
        : unlaid.length === 1
          ? 'it has no due date'
          : `none of them have a due date`;
  return `${lead}: ${reasons}.`;
}
