import { describe, expect, it } from 'vitest';

import type { Task } from '@/shared/api/types';
import { calendarDateOfDay, dayNumberOf } from '@/shared/lib/days';

import { addMonths, lanesUsed, layoutWeek, monthOf, tasksOn, weeksOfMonth } from './calendarModel';

/** A task with only the fields a test cares about filled in. */
function task(overrides: Partial<Task> & Pick<Task, 'id'>): Task {
  return {
    projectId: 'project',
    number: 1,
    title: overrides.id,
    description: '',
    statusId: 'todo',
    priority: 'normal',
    assignee: null,
    parentId: null,
    blockedBy: [],
    estimateMinutes: null,
    sprintId: null,
    points: null,
    startOn: null,
    dueOn: null,
    rank: 0,
    createdAt: '2026-11-11T09:00:00.000Z',
    updatedAt: '2026-11-11T09:00:00.000Z',
    completedAt: null,
    ...overrides,
  };
}

/** The week from Monday 9 to Sunday 15 November 2026. */
const week = Array.from({ length: 7 }, (_, weekday) => dayNumberOf('2026-11-09') + weekday);

describe('monthOf and addMonths', () => {
  it('name the month of a day', () => {
    expect(monthOf(dayNumberOf('2026-11-12'))).toEqual({ year: 2026, month: 11 });
  });

  it('go forward and back across the end of a year', () => {
    expect(addMonths({ year: 2026, month: 11 }, 2)).toEqual({ year: 2027, month: 1 });
    expect(addMonths({ year: 2026, month: 1 }, -1)).toEqual({ year: 2025, month: 12 });
  });
});

describe('weeksOfMonth', () => {
  it('shows six weeks from the Monday on or before the first of the month', () => {
    // 1 November 2026 is a Sunday.
    const weeks = weeksOfMonth({ year: 2026, month: 11 });

    expect(weeks).toHaveLength(6);
    expect(weeks.every((days) => days.length === 7)).toBe(true);
    expect(calendarDateOfDay(weeks[0]?.[0] ?? 0)).toBe('2026-10-26');
    expect(calendarDateOfDay(weeks[5]?.[6] ?? 0)).toBe('2026-12-06');
  });

  it('begins with the first of the month when that is a Monday', () => {
    // 1 June 2026 is a Monday.
    const weeks = weeksOfMonth({ year: 2026, month: 6 });

    expect(calendarDateOfDay(weeks[0]?.[0] ?? 0)).toBe('2026-06-01');
  });
});

describe('layoutWeek', () => {
  it('puts a task of several days on one line across them', () => {
    const tasks = [task({ id: 'three days', startOn: '2026-11-10', dueOn: '2026-11-12' })];

    const { pieces } = layoutWeek(week, tasks, 3);

    expect(pieces).toEqual([
      {
        task: tasks[0],
        firstColumn: 1,
        columnCount: 3,
        lane: 0,
        beginsHere: true,
        endsHere: true,
      },
    ]);
  });

  it('puts a task with one date on its day', () => {
    const tasks = [task({ id: 'deadline', dueOn: '2026-11-13' })];

    const [piece] = layoutWeek(week, tasks, 3).pieces;

    expect(piece).toMatchObject({ firstColumn: 4, columnCount: 1 });
  });

  it('lets two tasks share a line when they do not meet', () => {
    const tasks = [
      task({ id: 'early', number: 1, startOn: '2026-11-09', dueOn: '2026-11-10' }),
      task({ id: 'late', number: 2, startOn: '2026-11-12', dueOn: '2026-11-13' }),
    ];

    const lanes = layoutWeek(week, tasks, 3).pieces.map((piece) => piece.lane);

    expect(lanes).toEqual([0, 0]);
  });

  it('gives tasks that meet lines of their own, the earlier and longer one on top', () => {
    const tasks = [
      task({ id: 'short', number: 1, startOn: '2026-11-10', dueOn: '2026-11-10' }),
      task({ id: 'long', number: 2, startOn: '2026-11-10', dueOn: '2026-11-13' }),
      task({ id: 'first', number: 3, startOn: '2026-11-09', dueOn: '2026-11-10' }),
    ];

    const lanes = Object.fromEntries(
      layoutWeek(week, tasks, 3).pieces.map((piece) => [piece.task.id, piece.lane]),
    );

    expect(lanes).toEqual({ first: 0, long: 1, short: 2 });
  });

  it('cuts a task at the edges of the week and says that it goes on', () => {
    const tasks = [task({ id: 'long', startOn: '2026-11-05', dueOn: '2026-11-20' })];

    const [piece] = layoutWeek(week, tasks, 3).pieces;

    expect(piece).toMatchObject({
      firstColumn: 0,
      columnCount: 7,
      beginsHere: false,
      endsHere: false,
    });
  });

  it('leaves out tasks of other weeks and tasks without a date', () => {
    const tasks = [
      task({ id: 'before', dueOn: '2026-11-08' }),
      task({ id: 'after', startOn: '2026-11-16', dueOn: '2026-11-17' }),
      task({ id: 'undated' }),
    ];

    expect(layoutWeek(week, tasks, 3).pieces).toEqual([]);
  });

  it('lists a task that finds no line with each of its days', () => {
    const tasks = [
      task({ id: 'a', number: 1, startOn: '2026-11-10', dueOn: '2026-11-12' }),
      task({ id: 'b', number: 2, startOn: '2026-11-10', dueOn: '2026-11-12' }),
      task({ id: 'c', number: 3, startOn: '2026-11-11', dueOn: '2026-11-12' }),
    ];

    const { pieces, crowdedOut } = layoutWeek(week, tasks, 2);

    expect(pieces.map((piece) => piece.task.id)).toEqual(['a', 'b']);
    expect(crowdedOut.map((hidden) => hidden.map((listed) => listed.id))).toEqual([
      [],
      [],
      ['c'],
      ['c'],
      [],
      [],
      [],
    ]);
  });
});

describe('tasksOn', () => {
  it('lists the tasks that take up a day, the earlier and longer ones first', () => {
    const tasks = [
      task({ id: 'deadline', number: 1, dueOn: '2026-11-12' }),
      task({ id: 'long', number: 2, startOn: '2026-11-09', dueOn: '2026-11-13' }),
      task({ id: 'elsewhere', number: 3, startOn: '2026-11-13', dueOn: '2026-11-14' }),
      task({ id: 'undated', number: 4 }),
    ];

    const listed = tasksOn(dayNumberOf('2026-11-12'), tasks).map((found) => found.id);

    expect(listed).toEqual(['long', 'deadline']);
  });
});

describe('lanesUsed', () => {
  it('counts the lines the pieces of a week are on', () => {
    const tasks = [
      task({ id: 'a', number: 1, startOn: '2026-11-10', dueOn: '2026-11-12' }),
      task({ id: 'b', number: 2, startOn: '2026-11-10', dueOn: '2026-11-12' }),
      task({ id: 'c', number: 3, startOn: '2026-11-11', dueOn: '2026-11-12' }),
    ];

    expect(lanesUsed(layoutWeek(week, tasks, 6).pieces, 2)).toBe(3);
  });

  it('is never fewer than the fewest lines a week shows', () => {
    expect(lanesUsed([], 2)).toBe(2);
  });
});
