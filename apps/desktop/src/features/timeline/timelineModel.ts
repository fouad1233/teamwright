import type { Area, Id, Member, Project, Status, Task } from '@/shared/api/types';
import type { DayNumber } from '@/shared/lib/days';

import { asScheduled } from '../gantt/ganttModel';
import { spanOf, type Span } from '../tasks/taskSpan';

/**
 * The arithmetic of the timeline: which row a task is in, which line of its
 * row its bar lies on, and which tasks of a row have no dates.
 *
 * The scale of days is the Gantt chart's (`ganttModel.ts`); this file only
 * adds what the timeline does differently: rows of people instead of rows of
 * tasks, and bars that share a row.
 */

/** The height of one line of bars within a row, in pixels. */
export const LANE_HEIGHT = 30;

/** The height of a bar, in pixels. */
export const BAR_HEIGHT = 22;

/** The space above the first line of bars of a row and below the last, in pixels. */
export const ROW_PADDING = 8;

/**
 * How many days a task with only one date keeps free on its line: the day
 * of its mark and room for its key after it.
 */
export const POINT_DAYS = 3;

/** How many tasks without dates a row lists before it offers to show the rest. */
export const UNDATED_SHOWN = 3;

/** What the rows of the timeline stand for. */
export type Grouping = 'person' | 'status' | 'area';

/** What one row of the timeline stands for. */
export type RowSubject =
  | { kind: 'manager' }
  | { kind: 'member'; member: Member }
  | { kind: 'nobody' }
  | { kind: 'status'; status: Status }
  | { kind: 'area'; area: Area }
  /** Tasks whose person, status or area is not among the others, so that none is lost. */
  | { kind: 'other'; grouping: Grouping };

/** A task with dates, and the line of its row its bar lies on. */
export interface PlacedTask {
  task: Task;
  span: Span;
  /** The line within the row, counted from 0 at the top. */
  lane: number;
}

/** One row of the timeline. */
export interface TimelineRow {
  /** Stable among the rows, for React and for remembering what is unfolded. */
  key: string;
  subject: RowSubject;
  /** The tasks with dates, in the order of the schedule. */
  placed: PlacedTask[];
  /** The tasks without any date, by number. */
  undated: Task[];
  /** How many lines of bars the row has. At least one, so that an empty row keeps its height. */
  laneCount: number;
}

/** What the rows are made from besides the tasks. */
export interface RowSources {
  project: Pick<Project, 'statuses'>;
  /** The team, in the order its members joined. */
  members: readonly Member[];
  /** The areas, in the order the manager gave them. */
  areas: readonly Area[];
}

/**
 * The last day a bar keeps its line for. A task with a start and a due date
 * keeps the days between them. A task with only one date is a mark on a day
 * with its key beside it, so it keeps a few days more.
 */
export function reachOf(span: Span): DayNumber {
  return span.kind === 'planned' ? span.lastDay : span.firstDay + POINT_DAYS - 1;
}

/** Something that takes up days on a line. */
export interface Stretch {
  id: Id;
  firstDay: DayNumber;
  lastDay: DayNumber;
}

/**
 * Lays stretches of days on lines so that no two on a line share a day: each
 * one, in the order given, goes on the highest line that is free on all its
 * days. Give them in the order they begin, and the lines fill from the top.
 */
export function layoutLanes(stretches: readonly Stretch[]): Map<Id, number> {
  /** The last day taken on each line. */
  const takenUntil: DayNumber[] = [];
  const lanes = new Map<Id, number>();
  for (const stretch of stretches) {
    const free = takenUntil.findIndex((lastDay) => lastDay < stretch.firstDay);
    const lane = free === -1 ? takenUntil.length : free;
    takenUntil[lane] = stretch.lastDay;
    lanes.set(stretch.id, lane);
  }
  return lanes;
}

/** Lays out the tasks of one row. */
function rowOf(key: string, subject: RowSubject, tasks: readonly Task[]): TimelineRow {
  const dated = tasks
    .flatMap((task) => {
      const span = spanOf(task);
      return span ? [{ task, span }] : [];
    })
    .sort((first, second) => asScheduled(first.task, second.task));
  const lanes = layoutLanes(
    dated.map(({ task, span }) => ({
      id: task.id,
      firstDay: span.firstDay,
      lastDay: reachOf(span),
    })),
  );
  const placed = dated.map(({ task, span }) => ({ task, span, lane: lanes.get(task.id) ?? 0 }));
  return {
    key,
    subject,
    placed,
    undated: tasks.filter((task) => spanOf(task) === undefined).sort((a, b) => a.number - b.number),
    laneCount: placed.reduce((count, { lane }) => Math.max(count, lane + 1), 1),
  };
}

/** The key of the row a task belongs in, for a grouping. */
function rowKeyOf(task: Task, grouping: Grouping): string {
  switch (grouping) {
    case 'person':
      if (task.assignee === null) {
        return 'nobody';
      }
      return task.assignee.kind === 'manager' ? 'manager' : `member:${task.assignee.memberId}`;
    case 'status':
      return `status:${task.statusId}`;
    case 'area':
      return `area:${task.areaId}`;
  }
}

/** The rows a grouping always has, in their order, whether or not they have tasks. */
function subjectsOf(grouping: Grouping, sources: RowSources): [string, RowSubject][] {
  switch (grouping) {
    case 'person':
      return [
        ['manager', { kind: 'manager' }],
        ...sources.members.map((member): [string, RowSubject] => [
          `member:${member.id}`,
          { kind: 'member', member },
        ]),
        ['nobody', { kind: 'nobody' }],
      ];
    case 'status':
      return sources.project.statuses.map((status) => [
        `status:${status.id}`,
        { kind: 'status', status },
      ]);
    case 'area':
      return sources.areas.map((area) => [`area:${area.id}`, { kind: 'area', area }]);
  }
}

/**
 * The rows of the timeline. Grouped by person: the manager, every member of
 * the team in the order they joined, and the tasks given to nobody, a row
 * that is left out when there are none. Grouped by status: every status of
 * the workflow, in the order of the board. Grouped by area: every area, in
 * its order. A task whose row is not among these, such as one given to a
 * member who has left, goes in a last row of its own.
 */
export function timelineRows(
  tasks: readonly Task[],
  grouping: Grouping,
  sources: RowSources,
): TimelineRow[] {
  const subjects = subjectsOf(grouping, sources);
  const known = new Set(subjects.map(([key]) => key));
  const tasksByKey = new Map<string, Task[]>();
  for (const task of tasks) {
    const ownKey = rowKeyOf(task, grouping);
    const key = known.has(ownKey) ? ownKey : 'other';
    tasksByKey.set(key, [...(tasksByKey.get(key) ?? []), task]);
  }

  const rows = subjects
    .filter(([key]) => key !== 'nobody' || tasksByKey.has(key))
    .map(([key, subject]) => rowOf(key, subject, tasksByKey.get(key) ?? []));
  const others = tasksByKey.get('other');
  if (others) {
    rows.push(rowOf('other', { kind: 'other', grouping }, others));
  }
  return rows;
}

/** The height of the lines of bars of a row, with the space above and below them, in pixels. */
export function trackHeightOf(laneCount: number): number {
  return laneCount * LANE_HEIGHT + 2 * ROW_PADDING;
}

/** Where the bar on a line of a row lies, in pixels from the top of the row. */
export function barTopOf(lane: number): number {
  return ROW_PADDING + lane * LANE_HEIGHT + (LANE_HEIGHT - BAR_HEIGHT) / 2;
}
