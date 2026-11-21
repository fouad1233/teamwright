import type { CalendarDate, Project, Task } from '@/shared/api/types';
import { partsOfDay, dayNumberOf } from '@/shared/lib/days';
import { formatDay } from '@/shared/lib/format';

import {
  isInWork,
  isLate,
  shareOf,
  thisWeek,
  type DashboardCounts,
} from '../dashboard/dashboardModel';

/**
 * Pure functions for the report: what the view keeps, and the few ways the
 * sheet arranges what the dashboard works out. No figure is computed here a
 * second way: the counts, the sentence, the week, the lateness and the
 * shares are the dashboard's own (`dashboardModel.ts`).
 */

/** The parts of the sheet, in the order they are printed. */
export const PARTS = [
  'introduction',
  'standing',
  'progress',
  'sprint',
  'finished',
  'inWork',
  'next',
  'attention',
  'team',
  'usage',
] as const;

export type Part = (typeof PARTS)[number];

/** What each part is called, beside the sheet and as its heading on it. */
export const PART_LABELS: Record<Part, string> = {
  introduction: 'Introduction',
  standing: 'Where it stands',
  progress: 'Progress',
  sprint: 'The sprint',
  finished: 'Finished this week',
  inWork: 'In work',
  next: 'Coming next',
  attention: 'Late or waiting',
  team: 'The team',
  usage: 'What the team used',
};

/** The longest introduction the view keeps, in characters. */
export const INTRODUCTION_MAX_LENGTH = 2000;

/**
 * The two sheets a report view can show: the page of how the project
 * stands now, made from the dashboard's figures, and the report of the
 * whole project from its start (decision 0042), gathered by the backend.
 */
export type Sheet = 'now' | 'whole';

/** What each sheet is called in the control beside it. */
export const SHEET_LABELS: Record<Sheet, string> = {
  now: 'Now',
  whole: 'Whole project',
};

/**
 * What a report view keeps: the manager's introduction, which parts are
 * left out, and which sheet it shows.
 */
export interface ReportSettings {
  introduction: string;
  /** The parts left out, in the order of `PARTS`. A part the view does not name shows. */
  hidden: Part[];
  /** The sheet the view shows. Content written before there were two reads as `now`. */
  sheet: Sheet;
}

/**
 * A new report shows every part but the usage: what the specialists spent
 * is the manager's own business more often than the reader's.
 */
export const NEW_REPORT: ReportSettings = { introduction: '', hidden: ['usage'], sheet: 'now' };

function isPart(value: unknown): value is Part {
  return typeof value === 'string' && (PARTS as readonly string[]).includes(value);
}

/**
 * Reads what a view keeps. Content that is missing, empty or damaged reads
 * as a new report; a part it does not know is dropped, so content written
 * by a newer version still reads, and a part added later shows.
 */
export function parseReport(content: string | null): ReportSettings {
  if (content === null || content.trim() === '') {
    return NEW_REPORT;
  }
  let raw: unknown;
  try {
    raw = JSON.parse(content);
  } catch {
    return NEW_REPORT;
  }
  if (typeof raw !== 'object' || raw === null || Array.isArray(raw)) {
    return NEW_REPORT;
  }
  const { introduction, hidden, sheet } = raw as {
    introduction?: unknown;
    hidden?: unknown;
    sheet?: unknown;
  };
  const named = Array.isArray(hidden) ? hidden.filter(isPart) : NEW_REPORT.hidden;
  return {
    introduction:
      typeof introduction === 'string' ? introduction.slice(0, INTRODUCTION_MAX_LENGTH) : '',
    hidden: PARTS.filter((part) => named.includes(part)),
    sheet: sheet === 'whole' ? 'whole' : 'now',
  };
}

/** Writes what a view keeps, as `parseReport` reads it. */
export function serializeReport(settings: ReportSettings): string {
  return JSON.stringify({
    introduction: settings.introduction,
    hidden: settings.hidden,
    sheet: settings.sheet,
  });
}

/** Whether a part is on the sheet. */
export function shows(settings: ReportSettings, part: Part): boolean {
  return !settings.hidden.includes(part);
}

/** The report with a part shown if it was left out, and left out if it was shown. */
export function togglePart(settings: ReportSettings, part: Part): ReportSettings {
  const hidden = shows(settings, part)
    ? [...settings.hidden, part]
    : settings.hidden.filter((each) => each !== part);
  return { ...settings, hidden: PARTS.filter((each) => hidden.includes(each)) };
}

/** The report with a new introduction, cut to the longest that is kept. */
export function withIntroduction(settings: ReportSettings, introduction: string): ReportSettings {
  return { ...settings, introduction: introduction.slice(0, INTRODUCTION_MAX_LENGTH) };
}

/** The report showing another sheet. */
export function withSheet(settings: ReportSettings, sheet: Sheet): ReportSettings {
  return settings.sheet === sheet ? settings : { ...settings, sheet };
}

/** The date under the name: "Monday 16 November 2026". */
export function dateLine(today: CalendarDate): string {
  return `${formatDay(today)} ${partsOfDay(dayNumberOf(today)).year.toString()}`;
}

/** The one bar of progress: how much of it is finished, and how much is in work after that. */
export interface ProgressBar {
  finished: number;
  inWork: number;
}

/** The shares of the bar of progress, from the dashboard's counts. */
export function progressBar(counts: Pick<DashboardCounts, 'total' | 'finished' | 'inWork'>) {
  return {
    finished: shareOf(counts.finished, counts.total),
    inWork: shareOf(counts.inWork, counts.total),
  } satisfies ProgressBar;
}

/** The tasks in work, in the order of their keys. */
export function tasksInWork(project: Project, tasks: readonly Task[]): Task[] {
  return tasks.filter((task) => isInWork(project, task)).sort((a, b) => a.number - b.number);
}

/**
 * What comes next: the tasks of this week that are due by Sunday and not
 * yet late, the one due first first. The late ones are in a part of their
 * own.
 */
export function comingNext(project: Project, tasks: readonly Task[], today: CalendarDate): Task[] {
  return thisWeek(project, tasks, today).due.filter((task) => !isLate(project, task, today));
}

/** The longest list a part prints, so that the sheet stays on one or two pages. */
export const LIST_LENGTH = 8;

/** The first lines of a list, and how many more there are. */
export function firstLines<Item>(
  items: readonly Item[],
  length: number = LIST_LENGTH,
): { lines: Item[]; more: number } {
  return { lines: items.slice(0, length), more: Math.max(0, items.length - length) };
}

/**
 * The line under what is late, written for a reader who is not the
 * manager: "Nothing is late. 2 things wait for the manager."
 */
export function attentionInWords(late: number, waiting: number): string {
  const lateWords = late === 0 ? 'Nothing is late.' : null;
  const waitingWords =
    waiting === 0
      ? 'Nothing waits for the manager.'
      : `${waiting.toString()} ${waiting === 1 ? 'thing waits' : 'things wait'} for the manager.`;
  return lateWords === null ? waitingWords : `${lateWords} ${waitingWords}`;
}

/** "And 4 more." under a list that was cut. */
export function moreInWords(more: number): string {
  return `And ${more.toString()} more.`;
}
