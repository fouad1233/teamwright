import { describe, expect, it } from 'vitest';

import type { Task } from '@/shared/api/types';

import {
  indexById,
  subtasksOf,
  tasksOfStatus,
  unfinishedBlockers,
  withMovedTask,
  withTask,
} from './taskModel';

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
    startOn: null,
    dueOn: null,
    rank: 0,
    createdAt: '2026-11-11T09:00:00.000Z',
    updatedAt: '2026-11-11T09:00:00.000Z',
    completedAt: null,
    ...overrides,
  };
}

function ids(tasks: Task[]): string[] {
  return tasks.map((listed) => listed.id);
}

describe('tasksOfStatus', () => {
  it('returns the tasks of the status ordered by rank', () => {
    const tasks = [
      task({ id: 'c', rank: 30 }),
      task({ id: 'a', rank: 10 }),
      task({ id: 'elsewhere', statusId: 'done', rank: 0 }),
      task({ id: 'b', rank: 20 }),
    ];

    expect(ids(tasksOfStatus(tasks, 'todo'))).toEqual(['a', 'b', 'c']);
  });

  it('orders tasks with equal ranks by their number', () => {
    const tasks = [task({ id: 'second', number: 2 }), task({ id: 'first', number: 1 })];

    expect(ids(tasksOfStatus(tasks, 'todo'))).toEqual(['first', 'second']);
  });
});

describe('withTask', () => {
  it('replaces a task that is already in the list', () => {
    const tasks = [task({ id: 'a', title: 'Old' }), task({ id: 'b' })];

    const changed = withTask(tasks, task({ id: 'a', title: 'New' }));

    expect(changed.map((listed) => listed.title)).toEqual(['New', 'b']);
  });

  it('adds a task that is not in the list', () => {
    const changed = withTask([task({ id: 'a' })], task({ id: 'b' }));

    expect(ids(changed)).toEqual(['a', 'b']);
  });

  it('leaves the original list untouched', () => {
    const tasks = [task({ id: 'a', title: 'Old' })];

    withTask(tasks, task({ id: 'a', title: 'New' }));

    expect(tasks[0]?.title).toBe('Old');
  });
});

describe('withMovedTask', () => {
  const tasks = [
    task({ id: 'a', rank: 0 }),
    task({ id: 'b', rank: 1024 }),
    task({ id: 'c', rank: 2048 }),
    task({ id: 'x', statusId: 'doing', rank: 0 }),
  ];

  it('moves a task to the top of another status', () => {
    const moved = withMovedTask(tasks, 'x', 'todo', 0);

    expect(ids(tasksOfStatus(moved, 'todo'))).toEqual(['x', 'a', 'b', 'c']);
    expect(tasksOfStatus(moved, 'doing')).toEqual([]);
  });

  it('moves a task between two tasks of another status', () => {
    const moved = withMovedTask(tasks, 'x', 'todo', 2);

    expect(ids(tasksOfStatus(moved, 'todo'))).toEqual(['a', 'b', 'x', 'c']);
  });

  it('treats a position past the end as the bottom', () => {
    const moved = withMovedTask(tasks, 'x', 'todo', 99);

    expect(ids(tasksOfStatus(moved, 'todo'))).toEqual(['a', 'b', 'c', 'x']);
  });

  it('reorders a task within its own status', () => {
    const moved = withMovedTask(tasks, 'c', 'todo', 0);

    expect(ids(tasksOfStatus(moved, 'todo'))).toEqual(['c', 'a', 'b']);
  });

  it('moves a task into an empty status', () => {
    const moved = withMovedTask(tasks, 'a', 'done', 0);

    expect(ids(tasksOfStatus(moved, 'done'))).toEqual(['a']);
  });
});

describe('unfinishedBlockers', () => {
  it('lists the blockers that are not complete', () => {
    const finished = task({ id: 'finished', completedAt: '2026-11-11T10:00:00.000Z' });
    const open = task({ id: 'open' });
    const waiting = task({ id: 'waiting', blockedBy: ['finished', 'open', 'deleted'] });

    const blockers = unfinishedBlockers(waiting, indexById([finished, open, waiting]));

    expect(ids(blockers)).toEqual(['open']);
  });
});

describe('subtasksOf', () => {
  it('lists the direct subtasks by number', () => {
    const tasks = [
      task({ id: 'parent', number: 1 }),
      task({ id: 'later', number: 3, parentId: 'parent' }),
      task({ id: 'earlier', number: 2, parentId: 'parent' }),
      task({ id: 'grandchild', number: 4, parentId: 'earlier' }),
    ];

    expect(ids(subtasksOf(tasks, 'parent'))).toEqual(['earlier', 'later']);
  });
});
