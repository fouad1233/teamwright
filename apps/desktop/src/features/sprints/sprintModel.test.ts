import { describe, expect, it } from 'vitest';

import type { Project, Sprint, Task } from '@/shared/api/types';

import {
  daysLeft,
  endAfterWeeks,
  finishedOf,
  formatDaysLeft,
  formatSprintDates,
  formatSummary,
  formatTotals,
  layoutOf,
  movedOnNote,
  openSprints,
  paceBarsOf,
  parsePoints,
  previousSprintOf,
  recordFactsOf,
  reportVersionOf,
  rowsOf,
  suggestedDates,
  totalsOf,
  withPlannedTask,
} from './sprintModel';

const PROJECT: Project = {
  id: 'project',
  key: 'SSN',
  name: 'Smart sensor node',
  description: '',
  statuses: [
    { id: 'todo', name: 'To do', category: 'todo', color: 'gray' },
    { id: 'doing', name: 'In progress', category: 'active', color: 'blue' },
    { id: 'done', name: 'Done', category: 'done', color: 'green' },
  ],
  createdAt: '2026-11-01T09:00:00.000Z',
  archivedAt: null,
};

function task(overrides: Partial<Task> & Pick<Task, 'id'>): Task {
  return {
    projectId: 'project',
    areaId: 'area',
    number: 1,
    title: 'A task',
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
    createdAt: '2026-11-01T09:00:00.000Z',
    updatedAt: '2026-11-01T09:00:00.000Z',
    completedAt: null,
    ...overrides,
  };
}

function sprint(overrides: Partial<Sprint> & Pick<Sprint, 'id'>): Sprint {
  return {
    projectId: 'project',
    name: 'Sprint 1',
    goal: '',
    startOn: null,
    endOn: null,
    state: 'planned',
    createdAt: '2026-11-01T09:00:00.000Z',
    startedAt: null,
    completedAt: null,
    summary: null,
    ...overrides,
  };
}

describe('the layout of the sprints', () => {
  it('puts the active sprint, the planned ones and the completed ones apart', () => {
    const active = sprint({ id: 'a', state: 'active' });
    const planned = sprint({ id: 'p' });
    const completed = sprint({ id: 'c', state: 'completed' });

    expect(layoutOf([active, planned, completed])).toEqual({
      active,
      planned: [planned],
      completed: [completed],
    });
  });

  it('offers only the sprints that are not completed to move a task to', () => {
    const sprints = [
      sprint({ id: 'a', state: 'active' }),
      sprint({ id: 'p' }),
      sprint({ id: 'c', state: 'completed' }),
    ];

    expect(openSprints(sprints).map((open) => open.id)).toEqual(['a', 'p']);
  });
});

describe('the rows of a sprint', () => {
  const tasks = [
    task({ id: 'done', statusId: 'done', sprintId: 's' }),
    task({ id: 'second', statusId: 'todo', rank: 2, sprintId: 's' }),
    task({ id: 'first', statusId: 'todo', rank: 1, sprintId: 's' }),
    task({ id: 'part', parentId: 'first', sprintId: 's' }),
    task({ id: 'doing', statusId: 'doing', sprintId: 's' }),
    task({ id: 'elsewhere', statusId: 'todo' }),
  ];

  it('orders the tasks by the place of their status, then by rank, and leaves subtasks out', () => {
    expect(rowsOf(PROJECT, tasks, 's').map((row) => row.id)).toEqual([
      'first',
      'second',
      'doing',
      'done',
    ]);
  });

  it('lists the backlog for no sprint', () => {
    expect(rowsOf(PROJECT, tasks, null).map((row) => row.id)).toEqual(['elsewhere']);
  });
});

describe('finished tasks', () => {
  it('are those in a status of the category done, whatever it is called', () => {
    const project = {
      ...PROJECT,
      statuses: [
        { id: 'finished', name: 'Finished', category: 'review' as const, color: 'amber' as const },
        { id: 'accepted', name: 'Accepted', category: 'done' as const, color: 'green' as const },
      ],
    };
    const tasks = [
      task({ id: 'checked', statusId: 'finished' }),
      task({ id: 'taken', statusId: 'accepted' }),
    ];

    expect(finishedOf(project, tasks).map((each) => each.id)).toEqual(['taken']);
  });
});

describe('totals', () => {
  it('counts the tasks and adds up their points', () => {
    const totals = totalsOf([
      task({ id: 'a', points: 3 }),
      task({ id: 'b', points: 5 }),
      task({ id: 'c' }),
    ]);

    expect(totals).toEqual({ tasks: 3, points: 8, withoutPoints: 1 });
    expect(formatTotals(totals)).toBe('3 tasks · 8 points');
  });

  it('says nothing of points when no task has any', () => {
    expect(formatTotals(totalsOf([task({ id: 'a' })]))).toBe('1 task');
  });

  it('says what a completed sprint did of what it took on', () => {
    expect(formatSummary({ unit: 'points', committed: 10, completed: 5 })).toBe('5 of 10 points');
    expect(formatSummary({ unit: 'tasks', committed: 1, completed: 1 })).toBe('1 of 1 task');
  });
});

describe('the dates of a sprint in words', () => {
  const now = new Date(2026, 10, 12);

  it('says both dates, or what there is of them', () => {
    expect(formatSprintDates({ startOn: '2026-11-10', endOn: '2026-11-23' }, now)).toBe(
      '10 Nov – 23 Nov',
    );
    expect(formatSprintDates({ startOn: '2026-11-10', endOn: null }, now)).toBe('From 10 Nov');
    expect(formatSprintDates({ startOn: null, endOn: null }, now)).toBe('No dates yet');
  });
});

describe('the days left', () => {
  it('counts the days from today to the last day', () => {
    expect(daysLeft('2026-11-20', '2026-11-11')).toBe(9);
  });

  it('says them in words, on the last day and after it too', () => {
    expect(formatDaysLeft(9)).toBe('9 days left');
    expect(formatDaysLeft(1)).toBe('1 day left');
    expect(formatDaysLeft(0)).toBe('Last day');
    expect(formatDaysLeft(-2)).toBe('Over by 2 days');
  });
});

describe('the dates of a sprint to start', () => {
  it('offers today and two weeks later', () => {
    expect(suggestedDates(sprint({ id: 's' }), '2026-11-12')).toEqual({
      startOn: '2026-11-12',
      endOn: '2026-11-26',
    });
  });

  it('keeps the dates the sprint was planned with', () => {
    const planned = sprint({ id: 's', startOn: '2026-11-16', endOn: '2026-11-20' });

    expect(suggestedDates(planned, '2026-11-12')).toEqual({
      startOn: '2026-11-16',
      endOn: '2026-11-20',
    });
  });

  it('ends a sprint some weeks after it starts', () => {
    expect(endAfterWeeks('2026-11-12', 1)).toBe('2026-11-19');
    expect(endAfterWeeks('2026-11-12', 4)).toBe('2026-12-10');
  });
});

describe('planning a task on screen', () => {
  it('moves the task and its subtasks, to any depth, and nothing else', () => {
    const tasks = [
      task({ id: 'parent' }),
      task({ id: 'child', parentId: 'parent' }),
      task({ id: 'grandchild', parentId: 'child' }),
      task({ id: 'other' }),
    ];

    const planned = withPlannedTask(tasks, 'parent', 's');

    expect(planned.map((each) => each.sprintId)).toEqual(['s', 's', 's', null]);
  });
});

describe('the pace from sprint to sprint', () => {
  it('measures every completed sprint against the largest of the same unit', () => {
    const first = sprint({
      id: 'first',
      state: 'completed',
      summary: { unit: 'points', committed: 20, completed: 10 },
    });
    const second = sprint({
      id: 'second',
      state: 'completed',
      summary: { unit: 'points', committed: 10, completed: 10 },
    });
    const counted = sprint({
      id: 'counted',
      state: 'completed',
      summary: { unit: 'tasks', committed: 4, completed: 2 },
    });

    const bars = paceBarsOf([first, second, counted]);

    expect(bars.map((bar) => [bar.committedShare, bar.completedShare])).toEqual([
      [1, 0.5],
      [0.5, 0.5],
      [1, 0.5],
    ]);
  });
});

describe('the version of a report', () => {
  it('changes when a task of the sprint changes or leaves it', () => {
    const tasks = [
      task({ id: 'a', sprintId: 's', updatedAt: '2026-11-02T09:00:00.000Z' }),
      task({ id: 'b', sprintId: 's' }),
    ];
    const before = reportVersionOf(tasks, 's');

    const changed = reportVersionOf(
      tasks.map((each) =>
        each.id === 'b' ? { ...each, updatedAt: '2026-11-03T09:00:00.000Z' } : each,
      ),
      's',
    );
    const left = reportVersionOf(tasks.slice(0, 1), 's');

    expect(changed).not.toBe(before);
    expect(left).not.toBe(before);
  });
});

describe('what a completed sprint did not finish', () => {
  it('says that the rest moved on, in the unit of the sprint', () => {
    expect(movedOnNote({ unit: 'points', committed: 10, completed: 5 })).toBe(
      'It finished 5 of 10 points. The rest moved on when the sprint was completed.',
    );
  });

  it('says nothing when everything was finished', () => {
    expect(movedOnNote({ unit: 'tasks', committed: 3, completed: 3 })).toBeNull();
  });
});

describe('the facts of a completed sprint', () => {
  const completed = sprint({
    id: 'c',
    state: 'completed',
    startOn: '2026-10-26',
    endOn: '2026-11-08',
    startedAt: '2026-10-26T09:00:00.000Z',
    completedAt: '2026-11-08T17:00:00.000Z',
    summary: { unit: 'points', committed: 10, completed: 1 },
  });
  const now = new Date('2026-11-12T12:00:00');

  it('lists the dates, the moments and the work, in the order they happened', () => {
    const facts = recordFactsOf(completed, now);

    expect(facts.map((fact) => fact.label)).toEqual([
      'Dates',
      'Started',
      'Completed',
      'Took on',
      'Finished',
    ]);
    expect(facts[0]?.value).toBe(formatSprintDates(completed, now));
    expect(facts[3]?.value).toBe('10 points');
    expect(facts[4]?.value).toBe('1 point');
  });

  it('leaves out what is not known', () => {
    const facts = recordFactsOf(sprint({ id: 'x', state: 'completed' }), now);

    expect(facts.map((fact) => fact.label)).toEqual(['Dates']);
  });
});

describe('the sprint the board shows', () => {
  const completed = [sprint({ id: 'a', state: 'completed' })];

  it('is the completed sprint a scope asks for', () => {
    expect(previousSprintOf({ kind: 'previous', sprintId: 'a' }, completed)?.id).toBe('a');
  });

  it('is nothing for the other scopes, or for a sprint that is gone', () => {
    expect(previousSprintOf({ kind: 'all' }, completed)).toBeUndefined();
    expect(previousSprintOf({ kind: 'current' }, completed)).toBeUndefined();
    expect(previousSprintOf({ kind: 'previous', sprintId: 'gone' }, completed)).toBeUndefined();
  });
});

describe('points as they are typed', () => {
  it('reads digits as a number and an empty field as no points', () => {
    expect(parsePoints(' 8 ')).toBe(8);
    expect(parsePoints('')).toBeNull();
  });

  it('does not understand anything else', () => {
    expect(parsePoints('2.5')).toBeUndefined();
    expect(parsePoints('a few')).toBeUndefined();
  });
});
