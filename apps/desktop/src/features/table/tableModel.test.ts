import { describe, expect, it } from 'vitest';

import type { CustomField, Task } from '@/shared/api/types';

import {
  COLUMNS,
  DEFAULT_SETTINGS,
  assigneeOf,
  assigneeValue,
  describeSort,
  fieldColumnId,
  formatHours,
  nextCell,
  parseSettings,
  serializeSettings,
  shownColumns,
  tableRows,
  totalsOf,
  withColumnToggled,
  withSortBy,
  type SortContext,
  type TableSort,
} from './tableModel';

/** A task with only the fields a test cares about filled in. */
function task(overrides: Partial<Task> & Pick<Task, 'id'>): Task {
  return {
    projectId: 'project',
    areaId: 'area',
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

const context: SortContext = {
  statusOrder: new Map([
    ['todo', 0],
    ['doing', 1],
    ['done', 2],
  ]),
  sprintOrder: new Map([
    ['first', 0],
    ['second', 1],
  ]),
  areaOrder: new Map([
    ['area', 0],
    ['other', 1],
  ]),
  assigneeName: (assignee) => (assignee.kind === 'manager' ? 'You' : assignee.memberId),
};

function idsOf(tasks: readonly Task[], sort: TableSort | null) {
  return tableRows(tasks, sort, context).map((row) => row.task.id);
}

describe('parseSettings', () => {
  it('falls back to every column, unsorted, for missing, empty or unreadable content', () => {
    expect(parseSettings(null)).toEqual(DEFAULT_SETTINGS);
    expect(parseSettings('')).toEqual(DEFAULT_SETTINGS);
    expect(parseSettings('{not json')).toEqual(DEFAULT_SETTINGS);
    expect(parseSettings('42')).toEqual(DEFAULT_SETTINGS);
    expect(parseSettings('null')).toEqual(DEFAULT_SETTINGS);
    expect(DEFAULT_SETTINGS.columns).toHaveLength(COLUMNS.length);
  });

  it('reads back what was written', () => {
    const settings = {
      columns: ['reference', 'title', 'status', 'due'] as const,
      sort: { column: 'due', direction: 'descending' } as const,
    };
    const written = serializeSettings({ columns: [...settings.columns], sort: settings.sort });
    expect(parseSettings(written)).toEqual({ columns: [...settings.columns], sort: settings.sort });
  });

  it('drops what it does not know, keeps the fixed columns and puts them in order', () => {
    const parsed = parseSettings(
      JSON.stringify({ columns: ['due', 'colour', 'status', 'due', 7], sort: 'by due' }),
    );
    expect(parsed).toEqual({ columns: ['reference', 'title', 'status', 'due'], sort: null });
  });

  it('forgets a sort by a column or a direction it does not know', () => {
    expect(parseSettings('{"sort":{"column":"mood","direction":"ascending"}}').sort).toBeNull();
    expect(parseSettings('{"sort":{"column":"due","direction":"sideways"}}').sort).toBeNull();
  });

  it('shows every column when the content names none', () => {
    expect(parseSettings('{"sort":null}').columns).toEqual(DEFAULT_SETTINGS.columns);
  });
});

describe('the columns', () => {
  it('can be hidden and shown again, but the task and its title stay', () => {
    const hidden = withColumnToggled(DEFAULT_SETTINGS, 'priority');
    expect(hidden.columns).not.toContain('priority');
    expect(withColumnToggled(hidden, 'priority').columns).toEqual(DEFAULT_SETTINGS.columns);
    expect(withColumnToggled(DEFAULT_SETTINGS, 'title').columns).toContain('title');
    expect(shownColumns(hidden).map((column) => column.id)).toEqual(hidden.columns);
  });

  it('forget a sort by a column that is hidden', () => {
    const sorted = withSortBy(DEFAULT_SETTINGS, 'due');
    expect(withColumnToggled(sorted, 'due').sort).toBeNull();
    expect(withColumnToggled(sorted, 'start').sort).toEqual(sorted.sort);
  });
});

describe('withSortBy', () => {
  it('sorts by a column from its first value on, and turns the order on a second press', () => {
    const once = withSortBy(DEFAULT_SETTINGS, 'points');
    expect(once.sort).toEqual({ column: 'points', direction: 'ascending' });
    expect(withSortBy(once, 'points').sort).toEqual({ column: 'points', direction: 'descending' });
    expect(withSortBy(withSortBy(once, 'points'), 'points').sort?.direction).toBe('ascending');
    expect(withSortBy(once, 'due').sort).toEqual({ column: 'due', direction: 'ascending' });
  });
});

describe('describeSort', () => {
  it('says how the rows are sorted in words that suit the column', () => {
    expect(describeSort({ column: 'due', direction: 'ascending' })).toBe('Due, earliest first');
    expect(describeSort({ column: 'priority', direction: 'ascending' })).toBe(
      'Priority, most urgent first',
    );
    expect(describeSort({ column: 'title', direction: 'descending' })).toBe('Title, Z to A');
  });
});

describe('tableRows', () => {
  it('keeps the arranged order without a sort: by status, then by rank', () => {
    const tasks = [
      task({ id: 'done', statusId: 'done', rank: 0 }),
      task({ id: 'second', statusId: 'todo', rank: 2 }),
      task({ id: 'first', statusId: 'todo', rank: 1 }),
      task({ id: 'doing', statusId: 'doing', rank: 0 }),
    ];
    expect(idsOf(tasks, null)).toEqual(['first', 'second', 'doing', 'done']);
  });

  it('puts every subtask under its task, set in, and sorts the subtasks the same way', () => {
    const tasks = [
      task({ id: 'b', number: 2 }),
      task({ id: 'a', number: 1 }),
      task({ id: 'a2', number: 4, parentId: 'a' }),
      task({ id: 'a1', number: 3, parentId: 'a' }),
      task({ id: 'a1x', number: 5, parentId: 'a1' }),
    ];
    const rows = tableRows(tasks, { column: 'reference', direction: 'ascending' }, context);
    expect(rows.map((row) => [row.task.id, row.depth])).toEqual([
      ['a', 0],
      ['a1', 1],
      ['a1x', 2],
      ['a2', 1],
      ['b', 0],
    ]);
    expect(idsOf(tasks, { column: 'reference', direction: 'descending' })).toEqual([
      'b',
      'a',
      'a2',
      'a1',
      'a1x',
    ]);
  });

  it('lets a subtask whose task is not shown stand on its own', () => {
    const tasks = [task({ id: 'orphan', parentId: 'hidden' })];
    expect(tableRows(tasks, null, context)).toEqual([{ task: tasks[0], depth: 0 }]);
  });

  it('puts empty cells last, whichever way the column is sorted', () => {
    const tasks = [
      task({ id: 'none', number: 1 }),
      task({ id: 'late', number: 2, dueOn: '2026-12-01' }),
      task({ id: 'soon', number: 3, dueOn: '2026-11-20' }),
    ];
    expect(idsOf(tasks, { column: 'due', direction: 'ascending' })).toEqual([
      'soon',
      'late',
      'none',
    ]);
    expect(idsOf(tasks, { column: 'due', direction: 'descending' })).toEqual([
      'late',
      'soon',
      'none',
    ]);
  });

  it('sorts titles as people read them, numbers within words included', () => {
    const tasks = [
      task({ id: 'x', title: 'Step 10' }),
      task({ id: 'y', title: 'step 9' }),
      task({ id: 'z', title: 'Assemble' }),
    ];
    expect(idsOf(tasks, { column: 'title', direction: 'ascending' })).toEqual(['z', 'y', 'x']);
  });

  it('sorts priorities from the most urgent first', () => {
    const tasks = [
      task({ id: 'low', priority: 'low' }),
      task({ id: 'urgent', priority: 'urgent' }),
      task({ id: 'normal', priority: 'normal' }),
      task({ id: 'high', priority: 'high' }),
    ];
    expect(idsOf(tasks, { column: 'priority', direction: 'ascending' })).toEqual([
      'urgent',
      'high',
      'normal',
      'low',
    ]);
  });

  it('sorts statuses, sprints and areas by their place, and people by name', () => {
    const tasks = [
      task({ id: 'p', statusId: 'done', sprintId: 'second', areaId: 'other' }),
      task({ id: 'q', statusId: 'todo', sprintId: null, assignee: { kind: 'manager' } }),
      task({
        id: 'r',
        statusId: 'doing',
        sprintId: 'first',
        assignee: { kind: 'member', memberId: 'Ada' },
      }),
    ];
    expect(idsOf(tasks, { column: 'status', direction: 'ascending' })).toEqual(['q', 'r', 'p']);
    expect(idsOf(tasks, { column: 'sprint', direction: 'ascending' })).toEqual(['r', 'p', 'q']);
    expect(idsOf(tasks, { column: 'area', direction: 'descending' })).toEqual(['p', 'q', 'r']);
    expect(idsOf(tasks, { column: 'assignee', direction: 'ascending' })).toEqual(['r', 'q', 'p']);
  });

  it('sorts by how many tasks a task waits for, the free ones last', () => {
    const tasks = [
      task({ id: 'free' }),
      task({ id: 'two', blockedBy: ['a', 'b'] }),
      task({ id: 'one', blockedBy: ['a'] }),
    ];
    expect(idsOf(tasks, { column: 'waitsFor', direction: 'descending' })).toEqual([
      'two',
      'one',
      'free',
    ]);
  });
});

describe('totalsOf', () => {
  it('counts every row and sums points and estimates', () => {
    const totals = totalsOf([
      task({ id: 'a', points: 3, estimateMinutes: 90 }),
      task({ id: 'b', points: 5 }),
      task({ id: 'c', estimateMinutes: 30, parentId: 'a' }),
    ]);
    expect(totals).toEqual({
      tasks: 3,
      points: 8,
      minutes: 120,
      withoutPoints: 1,
      withoutEstimate: 1,
    });
  });

  it('writes hours to a tenth', () => {
    expect(formatHours(120)).toBe('2h');
    expect(formatHours(90)).toBe('1.5h');
    expect(formatHours(100)).toBe('1.7h');
    expect(formatHours(0)).toBe('0h');
  });
});

describe('nextCell', () => {
  const size = { rows: 30, columns: 6 };

  it('moves by one with the arrows and stops at the edges', () => {
    expect(nextCell({ row: 0, column: 0 }, 'ArrowDown', false, size)).toEqual({
      row: 1,
      column: 0,
    });
    expect(nextCell({ row: 0, column: 0 }, 'ArrowUp', false, size)).toEqual({ row: 0, column: 0 });
    expect(nextCell({ row: 4, column: 5 }, 'ArrowRight', false, size)).toEqual({
      row: 4,
      column: 5,
    });
    expect(nextCell({ row: 4, column: 5 }, 'ArrowLeft', false, size)).toEqual({
      row: 4,
      column: 4,
    });
  });

  it('goes to the ends of a row with Home and End, and of the table with Ctrl', () => {
    expect(nextCell({ row: 4, column: 3 }, 'Home', false, size)).toEqual({ row: 4, column: 0 });
    expect(nextCell({ row: 4, column: 3 }, 'End', false, size)).toEqual({ row: 4, column: 5 });
    expect(nextCell({ row: 4, column: 3 }, 'Home', true, size)).toEqual({ row: 0, column: 0 });
    expect(nextCell({ row: 4, column: 3 }, 'End', true, size)).toEqual({ row: 29, column: 5 });
  });

  it('moves ten rows with the page keys', () => {
    expect(nextCell({ row: 4, column: 2 }, 'PageDown', false, size)).toEqual({
      row: 14,
      column: 2,
    });
    expect(nextCell({ row: 4, column: 2 }, 'PageUp', false, size)).toEqual({ row: 0, column: 2 });
  });

  it('does not move for other keys, or in an empty table', () => {
    expect(nextCell({ row: 0, column: 0 }, 'Enter', false, size)).toBeNull();
    expect(nextCell({ row: 0, column: 0 }, 'ArrowDown', false, { rows: 0, columns: 6 })).toBeNull();
  });
});

describe('the assignee menu', () => {
  it('turns an assignee into a value and back', () => {
    for (const assignee of [
      null,
      { kind: 'manager' as const },
      { kind: 'member' as const, memberId: 'm-1' },
    ]) {
      expect(assigneeOf(assigneeValue(assignee))).toEqual(assignee);
    }
  });
});

describe('the columns of custom fields', () => {
  const cost: CustomField = {
    id: 'cost',
    projectId: 'p',
    name: 'Unit cost',
    kind: 'number',
    unit: '€',
    position: 1,
    choices: [],
  };
  const supplier: CustomField = {
    ...cost,
    id: 'supplier',
    name: 'Supplier',
    kind: 'choice',
    position: 0,
  };
  const fields = [cost, supplier];

  it('shows every field after the other columns until the manager chooses', () => {
    const ids = shownColumns(DEFAULT_SETTINGS, fields).map((column) => column.id);
    expect(ids.slice(-3)).toEqual(['waitsFor', fieldColumnId('supplier'), fieldColumnId('cost')]);
    const kept = parseSettings('{"version":1,"columns":["reference","title","due"],"sort":null}');
    expect(shownColumns(kept, fields).map((column) => column.id)).toEqual([
      'reference',
      'title',
      'due',
      fieldColumnId('supplier'),
      fieldColumnId('cost'),
    ]);
  });

  it('keeps the fields chosen once one is turned off, and a new field stays hidden', () => {
    const hidden = withColumnToggled(DEFAULT_SETTINGS, fieldColumnId('cost'), fields);
    const read = parseSettings(serializeSettings(hidden));
    const later = [...fields, { ...cost, id: 'new', name: 'New', position: 2 }];
    expect(read.fieldsChosen).toBe(true);
    expect(shownColumns(read, later).map((column) => column.id)).toContain(
      fieldColumnId('supplier'),
    );
    expect(shownColumns(read, later).map((column) => column.id)).not.toContain(
      fieldColumnId('cost'),
    );
    expect(shownColumns(read, later).map((column) => column.id)).not.toContain(
      fieldColumnId('new'),
    );
  });

  it('sorts by a field and says so', () => {
    const tasks = [
      task({ id: 'cheap', fieldValues: { cost: { kind: 'number', number: 2 } } }),
      task({ id: 'none' }),
      task({ id: 'dear', fieldValues: { cost: { kind: 'number', number: 40 } } }),
    ];
    const sort: TableSort = { column: fieldColumnId('cost'), direction: 'descending' };
    expect(tableRows(tasks, sort, { ...context, fields }).map((row) => row.task.id)).toEqual([
      'dear',
      'cheap',
      'none',
    ]);
    expect(describeSort(sort, fields)).toBe('Unit cost, largest first');
    expect(describeSort(sort)).toBe('A field that was removed, last to first');
  });
});
