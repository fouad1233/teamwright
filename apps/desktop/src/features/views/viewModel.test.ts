import { describe, expect, it } from 'vitest';

import type { Member, Status, Task, View, ViewFilter } from '@/shared/api/types';

import {
  assigneeForNewTasks,
  EVERY_TASK,
  filterInWords,
  isFilterable,
  isFiltered,
  kindsToOffer,
  shownTaskIds,
  splitRow,
  viewToShow,
  withMovedView,
} from './viewModel';

const STATUSES: Status[] = [
  { id: 'todo', name: 'To do', category: 'todo', color: 'gray' },
  { id: 'shipped', name: 'Shipped', category: 'done', color: 'green' },
];

function task(id: string, more: Partial<Task> = {}): Task {
  return {
    id,
    projectId: 'p',
    areaId: 'area',
    number: 1,
    title: id,
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
    ...more,
  };
}

function view(id: string, kind: View['kind'], name = id): View {
  return { id, projectId: 'p', kind, name, filter: EVERY_TASK };
}

const ENGINEER = { kind: 'member', memberId: 'engineer' } as const;

function shown(tasks: Task[], filter: ViewFilter): string[] | null {
  const ids = shownTaskIds(tasks, STATUSES, filter);
  return ids === null ? null : [...ids].sort();
}

describe('the tasks a filter shows', () => {
  const tasks = [
    task('mine', { assignee: { kind: 'manager' } }),
    task('engineer', { assignee: ENGINEER }),
    task('nobody'),
    task('engineer-finished', { assignee: ENGINEER, statusId: 'shipped' }),
  ];

  it('shows every task without a filter', () => {
    expect(shown(tasks, EVERY_TASK)).toBeNull();
  });

  it('shows the tasks of the manager, of a member, or of nobody', () => {
    expect(shown(tasks, { assignee: { kind: 'manager' }, hidesFinished: false })).toEqual(['mine']);
    expect(shown(tasks, { assignee: ENGINEER, hidesFinished: false })).toEqual([
      'engineer',
      'engineer-finished',
    ]);
    expect(shown(tasks, { assignee: { kind: 'nobody' }, hidesFinished: false })).toEqual([
      'nobody',
    ]);
  });

  it('hides a task in a status of the category done, whatever the status is called', () => {
    expect(shown(tasks, { assignee: ENGINEER, hidesFinished: true })).toEqual(['engineer']);
    expect(shown(tasks, { assignee: { kind: 'anyone' }, hidesFinished: true })).toEqual([
      'engineer',
      'mine',
      'nobody',
    ]);
  });

  it('shows a task for one of its subtasks, with all its subtasks', () => {
    const family = [
      task('parent'),
      task('child', { parentId: 'parent', assignee: ENGINEER }),
      task('sibling', { parentId: 'parent' }),
      task('grandchild', { parentId: 'sibling' }),
      task('stranger'),
    ];

    expect(shown(family, { assignee: ENGINEER, hidesFinished: false })).toEqual([
      'child',
      'grandchild',
      'parent',
      'sibling',
    ]);
  });

  it('keeps the finished subtasks of a task that is shown', () => {
    const family = [
      task('parent', { assignee: ENGINEER }),
      task('done-child', { parentId: 'parent', statusId: 'shipped' }),
    ];

    expect(shown(family, { assignee: ENGINEER, hidesFinished: true })).toEqual([
      'done-child',
      'parent',
    ]);
  });
});

describe('a filter in words', () => {
  const members = [{ id: 'engineer', name: 'Firmware engineer' } as Member];

  it('says whose tasks are shown, and whether the finished ones are left out', () => {
    expect(filterInWords(EVERY_TASK, members)).toBeNull();
    expect(filterInWords({ assignee: ENGINEER, hidesFinished: false }, members)).toBe(
      'Shows the tasks of Firmware engineer. New ones go to them',
    );
    expect(filterInWords({ assignee: { kind: 'manager' }, hidesFinished: true }, members)).toBe(
      'Shows your unfinished tasks. New ones are yours',
    );
    expect(filterInWords({ assignee: { kind: 'nobody' }, hidesFinished: false }, members)).toBe(
      'Shows the tasks given to nobody',
    );
    expect(filterInWords({ assignee: { kind: 'anyone' }, hidesFinished: true }, members)).toBe(
      'Hides the finished tasks',
    );
  });

  it('does not name a member it does not know', () => {
    expect(
      filterInWords(
        { assignee: { kind: 'member', memberId: 'gone' }, hidesFinished: false },
        members,
      ),
    ).toBe('Shows the tasks of a member who has left the team');
  });
});

describe('who a task opened in a view is given to', () => {
  it('goes to the member or to the manager the filter shows', () => {
    expect(assigneeForNewTasks({ assignee: ENGINEER, hidesFinished: false })).toEqual(ENGINEER);
    expect(assigneeForNewTasks({ assignee: { kind: 'manager' }, hidesFinished: true })).toEqual({
      kind: 'manager',
    });
  });

  it('goes to nobody in a view of everybody or of nobody', () => {
    expect(assigneeForNewTasks(EVERY_TASK)).toBeUndefined();
    expect(
      assigneeForNewTasks({ assignee: { kind: 'nobody' }, hidesFinished: false }),
    ).toBeUndefined();
  });
});

describe('the kinds and the views', () => {
  it('offers a backlog and a dashboard only while the project has none', () => {
    const views = [view('a', 'board'), view('b', 'backlog')];

    expect(kindsToOffer(['board', 'backlog', 'list', 'dashboard'], views)).toEqual([
      'board',
      'list',
      'dashboard',
    ]);
  });

  it('filters boards, lists, Gantt charts and calendars only', () => {
    expect(isFilterable('board')).toBe(true);
    expect(isFilterable('calendar')).toBe(true);
    expect(isFilterable('backlog')).toBe(false);
    expect(isFilterable('dashboard')).toBe(false);
  });

  it('tells a filter that leaves something out', () => {
    expect(isFiltered(EVERY_TASK)).toBe(false);
    expect(isFiltered({ assignee: { kind: 'anyone' }, hidesFinished: true })).toBe(true);
  });

  it('shows the view asked for, or the first one when it is gone', () => {
    const views = [view('a', 'board'), view('b', 'list')];

    expect(viewToShow(views, 'b')?.id).toBe('b');
    expect(viewToShow(views, 'gone')?.id).toBe('a');
    expect(viewToShow(views, null)?.id).toBe('a');
    expect(viewToShow([], 'a')).toBeUndefined();
  });

  it('moves a view to a place, as the backend does', () => {
    const views = [view('a', 'board'), view('b', 'list'), view('c', 'gantt')];

    expect(withMovedView(views, 'c', 0).map((moved) => moved.id)).toEqual(['c', 'a', 'b']);
    expect(withMovedView(views, 'a', 9).map((moved) => moved.id)).toEqual(['b', 'c', 'a']);
  });
});

describe('the row of tabs', () => {
  const measures = { widths: [80, 80, 80, 80, 80], gap: 10, moreWidth: 60 };

  it('shows every tab that fits', () => {
    expect(splitRow({ ...measures, available: 440, openIndex: 0 })).toEqual({
      shown: [0, 1, 2, 3, 4],
      hidden: [],
    });
  });

  it('puts the tabs that do not fit behind the button for the rest', () => {
    expect(splitRow({ ...measures, available: 300, openIndex: 0 })).toEqual({
      shown: [0, 1],
      hidden: [2, 3, 4],
    });
  });

  it('keeps the open view in the row, in the place of the last tab that fits', () => {
    expect(splitRow({ ...measures, available: 300, openIndex: 4 })).toEqual({
      shown: [0, 4],
      hidden: [1, 2, 3],
    });
  });

  it('shows the open view even when nothing else fits', () => {
    expect(splitRow({ ...measures, available: 100, openIndex: 3 })).toEqual({
      shown: [3],
      hidden: [0, 1, 2, 4],
    });
  });
});
