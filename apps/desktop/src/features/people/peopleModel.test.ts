import { describe, expect, it } from 'vitest';

import type { Member, Status, Task } from '@/shared/api/types';

import {
  FINISHED_SHOWN,
  byDueDate,
  columnKeyOf,
  dropOutcome,
  figuresInWords,
  isSameAssignee,
  neighbourColumn,
  peopleColumns,
  withAssignee,
} from './peopleModel';

/** A task with only the fields a test cares about filled in. */
function task(overrides: Partial<Task> & Pick<Task, 'id'>): Task {
  return {
    projectId: 'project',
    areaId: 'general',
    number: 1,
    title: overrides.id,
    description: '',
    statusId: 'todo',
    priority: 'normal',
    assignee: null,
    parentId: null,
    blockedBy: [],
    tagIds: [],
    checklist: [],
    fieldValues: {},
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

function member(id: string, name: string): Member {
  return {
    id,
    projectId: 'project',
    name,
    kind: 'agent',
    responsibility: '',
    model: null,
    effort: null,
    instructions: '',
    color: 'blue',
    joinedAt: '2026-11-11T09:00:00.000Z',
    updatedAt: '2026-11-11T09:00:00.000Z',
  };
}

const statuses: Status[] = [
  { id: 'todo', name: 'To do', category: 'todo', color: 'gray' },
  { id: 'doing', name: 'Doing', category: 'active', color: 'blue' },
  { id: 'checking', name: 'Checking', category: 'review', color: 'purple' },
  { id: 'done', name: 'Done', category: 'done', color: 'green' },
];

const project = { statuses };
const members = [member('ada', 'Ada'), member('ben', 'Ben')];
const toAda = { kind: 'member', memberId: 'ada' } as const;
const toManager = { kind: 'manager' } as const;

describe('peopleColumns', () => {
  it('has the manager, every member in order, and nobody, even without tasks', () => {
    const columns = peopleColumns(project, [], members);
    expect(columns.map((column) => column.key)).toEqual([
      'manager',
      'member:ada',
      'member:ben',
      'nobody',
    ]);
    expect(columns.every((column) => column.figures.open === 0)).toBe(true);
  });

  it('puts every task in the column of whoever it is given to', () => {
    const columns = peopleColumns(
      project,
      [
        task({ id: 'a', assignee: toAda }),
        task({ id: 'm', assignee: toManager }),
        task({ id: 'n' }),
      ],
      members,
    );
    const ids = (key: string) =>
      columns.find((column) => column.key === key)?.next.map((given) => given.id);
    expect(ids('member:ada')).toEqual(['a']);
    expect(ids('manager')).toEqual(['m']);
    expect(ids('nobody')).toEqual(['n']);
    expect(ids('member:ben')).toEqual([]);
  });

  it('keeps tasks of somebody who left in a last column that receives nothing', () => {
    const columns = peopleColumns(
      project,
      [task({ id: 'gone', assignee: { kind: 'member', memberId: 'left' } })],
      members,
    );
    const last = columns.at(-1);
    expect(last?.key).toBe('other');
    expect(last?.receives).toBeUndefined();
    expect(last?.next.map((given) => given.id)).toEqual(['gone']);
  });

  it('follows the category of a status, never its name', () => {
    const [ada] = peopleColumns(
      project,
      [
        task({ id: 'new', statusId: 'todo', assignee: toAda }),
        task({ id: 'busy', statusId: 'doing', assignee: toAda }),
        task({ id: 'checked', statusId: 'checking', assignee: toAda }),
        task({ id: 'over', statusId: 'done', assignee: toAda }),
      ],
      [members[0] as Member],
    ).filter((column) => column.key === 'member:ada');
    expect(ada?.figures).toEqual({ open: 3, inWork: 2, finished: 1 });
    expect(ada?.inWork.map((given) => given.id)).toEqual(['busy', 'checked']);
    expect(ada?.next.map((given) => given.id)).toEqual(['new']);
    expect(ada?.finishedLately.map((given) => given.id)).toEqual(['over']);
  });

  it('counts a task in a status that is gone as not begun', () => {
    const [manager] = peopleColumns(
      project,
      [task({ id: 'lost', statusId: 'removed', assignee: toManager })],
      [],
    );
    expect(manager?.figures).toEqual({ open: 1, inWork: 0, finished: 0 });
    expect(manager?.next.map((given) => given.id)).toEqual(['lost']);
  });

  it('shows the tasks finished last, newest first, and counts them all', () => {
    const finished = Array.from({ length: FINISHED_SHOWN + 2 }, (_, index) =>
      task({
        id: `f${index.toString()}`,
        number: index + 1,
        statusId: 'done',
        assignee: toManager,
        completedAt: `2026-11-0${(index + 1).toString()}T09:00:00.000Z`,
      }),
    );
    const [manager] = peopleColumns(project, finished, []);
    expect(manager?.figures.finished).toBe(FINISHED_SHOWN + 2);
    expect(manager?.finishedLately.map((given) => given.id)).toEqual([
      'f6',
      'f5',
      'f4',
      'f3',
      'f2',
    ]);
  });
});

describe('byDueDate', () => {
  it('puts the earliest due first, those without a date last, and ties by number', () => {
    const sorted = [
      task({ id: 'none', number: 1 }),
      task({ id: 'late', number: 2, dueOn: '2026-11-20' }),
      task({ id: 'soon', number: 5, dueOn: '2026-11-14' }),
      task({ id: 'soon-too', number: 3, dueOn: '2026-11-14' }),
      task({ id: 'none-too', number: 0 }),
    ].sort(byDueDate);
    expect(sorted.map((given) => given.id)).toEqual([
      'soon-too',
      'soon',
      'late',
      'none-too',
      'none',
    ]);
  });
});

describe('giving a task to somebody else', () => {
  it('tells the columns of the ways of giving a task apart', () => {
    expect(columnKeyOf(null)).toBe('nobody');
    expect(columnKeyOf(toManager)).toBe('manager');
    expect(columnKeyOf(toAda)).toBe('member:ada');
    expect(isSameAssignee(toAda, { kind: 'member', memberId: 'ada' })).toBe(true);
    expect(isSameAssignee(toAda, null)).toBe(false);
  });

  it('gives a task dropped on another column to that column', () => {
    expect(dropOutcome({ assignee: toAda }, { receives: toManager })).toEqual({
      assignee: toManager,
    });
    expect(dropOutcome({ assignee: toAda }, { receives: null })).toEqual({ assignee: null });
  });

  it('changes nothing for a drop on its own column or on one that receives nothing', () => {
    expect(dropOutcome({ assignee: toAda }, { receives: toAda })).toBeUndefined();
    expect(dropOutcome({ assignee: null }, { receives: null })).toBeUndefined();
    expect(dropOutcome({ assignee: toAda }, {})).toBeUndefined();
    expect(dropOutcome({ assignee: toAda }, undefined)).toBeUndefined();
  });

  it('shows the task with its new assignee and leaves the others alone', () => {
    const tasks = [task({ id: 'a', assignee: toAda }), task({ id: 'b', assignee: toAda })];
    const changed = withAssignee(tasks, 'a', null);
    expect(changed[0]?.assignee).toBeNull();
    expect(changed[1]).toBe(tasks[1]);
  });
});

describe('neighbourColumn', () => {
  const columns = [
    { id: 'a', left: 0, width: 240 },
    { id: 'b', left: 240, width: 240 },
    { id: 'c', left: 480, width: 240 },
  ];

  it('finds the nearest column on either side, never the one the point is in', () => {
    expect(neighbourColumn(columns, 360, 'right')?.id).toBe('c');
    expect(neighbourColumn(columns, 360, 'left')?.id).toBe('a');
  });

  it('finds nothing beyond the first and the last column', () => {
    expect(neighbourColumn(columns, 600, 'right')).toBeUndefined();
    expect(neighbourColumn(columns, 120, 'left')).toBeUndefined();
  });

  it('does not depend on the order the columns are given in', () => {
    expect(neighbourColumn([...columns].reverse(), 120, 'right')?.id).toBe('b');
  });
});

describe('figuresInWords', () => {
  it('says the three figures, or that there are no tasks', () => {
    expect(figuresInWords({ open: 3, inWork: 1, finished: 2 })).toBe(
      '3 open, 1 in work, 2 finished',
    );
    expect(figuresInWords({ open: 0, inWork: 0, finished: 0 })).toBe('No tasks');
  });
});
