import { describe, expect, it } from 'vitest';

import type { Task } from '@/shared/api/types';

import {
  indexById,
  indexSubtasks,
  mainTasksOfStatus,
  outlineRows,
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

describe('mainTasksOfStatus', () => {
  it('leaves out the tasks that are part of another task', () => {
    const tasks = [
      task({ id: 'parent', rank: 10 }),
      task({ id: 'part', rank: 20, parentId: 'parent' }),
      task({ id: 'other', rank: 30 }),
    ];

    expect(ids(mainTasksOfStatus(tasks, 'todo'))).toEqual(['parent', 'other']);
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

  it('does not count subtasks when it places a task among the tasks of a status', () => {
    const withPart = [...tasks, task({ id: 'part', rank: 512, parentId: 'a' })];

    const moved = withMovedTask(withPart, 'x', 'todo', 1);

    expect(ids(mainTasksOfStatus(moved, 'todo'))).toEqual(['a', 'x', 'b', 'c']);
  });

  it('places a subtask among the other subtasks of its parent', () => {
    const parts = [
      task({ id: 'parent' }),
      task({ id: 'first', rank: 10, parentId: 'parent' }),
      task({ id: 'second', rank: 20, parentId: 'parent' }),
      task({ id: 'third', rank: 30, parentId: 'parent' }),
    ];

    const moved = withMovedTask(parts, 'third', 'todo', 0);
    const ofParent = tasksOfStatus(moved, 'todo').filter((part) => part.parentId === 'parent');

    expect(ids(ofParent)).toEqual(['third', 'first', 'second']);
  });

  it('changes nothing for a task that is not in the list', () => {
    expect(withMovedTask(tasks, 'missing', 'todo', 0)).toEqual(tasks);
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

describe('indexSubtasks', () => {
  it('lists the direct subtasks of every parent by number', () => {
    const tasks = [
      task({ id: 'parent', number: 1 }),
      task({ id: 'later', number: 3, parentId: 'parent' }),
      task({ id: 'earlier', number: 2, parentId: 'parent' }),
      task({ id: 'grandchild', number: 4, parentId: 'earlier' }),
      task({ id: 'alone', number: 5 }),
    ];

    const subtasksByParent = indexSubtasks(tasks);

    expect(ids(subtasksByParent.get('parent') ?? [])).toEqual(['earlier', 'later']);
    expect(ids(subtasksByParent.get('earlier') ?? [])).toEqual(['grandchild']);
    expect(subtasksByParent.has('alone')).toBe(false);
  });
});

describe('outlineRows', () => {
  const parent = task({ id: 'parent', number: 1 });
  const part = task({ id: 'part', number: 2, parentId: 'parent' });
  const partOfPart = task({ id: 'part of part', number: 3, parentId: 'part' });
  const alone = task({ id: 'alone', number: 4 });
  const subtasksByParent = indexSubtasks([parent, part, partOfPart, alone]);

  function outline(open: string[]): string[] {
    return outlineRows([parent, alone], subtasksByParent, (taskId) => open.includes(taskId)).map(
      (row) => `${'  '.repeat(row.depth)}${row.task.id}${row.isOpen ? ' (open)' : ''}`,
    );
  }

  it('shows only the tasks themselves while they are closed', () => {
    expect(outline([])).toEqual(['parent', 'alone']);
  });

  it('puts the subtasks of an open task right after it, one step in', () => {
    expect(outline(['parent'])).toEqual(['parent (open)', '  part', 'alone']);
  });

  it('follows open tasks to any depth', () => {
    expect(outline(['parent', 'part'])).toEqual([
      'parent (open)',
      '  part (open)',
      '    part of part',
      'alone',
    ]);
  });

  it('does not call a task open that has no subtasks', () => {
    expect(outline(['alone'])).toEqual(['parent', 'alone']);
  });

  it('tells every row how many subtasks its task has', () => {
    const rows = outlineRows([parent, alone], subtasksByParent, () => false);

    expect(rows.map((row) => row.subtasks.length)).toEqual([1, 0]);
  });
});
