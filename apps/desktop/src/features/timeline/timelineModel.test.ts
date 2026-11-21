import { describe, expect, it } from 'vitest';

import type { Area, Member, Status, Task } from '@/shared/api/types';

import { spanOf } from '../tasks/taskSpan';

import {
  BAR_HEIGHT,
  LANE_HEIGHT,
  POINT_DAYS,
  ROW_PADDING,
  barTopOf,
  layoutLanes,
  reachOf,
  timelineRows,
  trackHeightOf,
  type RowSources,
} from './timelineModel';

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
  { id: 'done', name: 'Done', category: 'done', color: 'green' },
];

const areas: Area[] = [
  { id: 'general', projectId: 'project', name: 'General' },
  { id: 'firmware', projectId: 'project', name: 'Firmware' },
];

const sources: RowSources = {
  project: { statuses },
  members: [member('ada', 'Ada'), member('ben', 'Ben')],
  areas,
};

const toAda = { kind: 'member', memberId: 'ada' } as const;

describe('layoutLanes', () => {
  it('puts stretches that do not share a day on one line', () => {
    const lanes = layoutLanes([
      { id: 'a', firstDay: 1, lastDay: 3 },
      { id: 'b', firstDay: 4, lastDay: 6 },
    ]);
    expect([...lanes]).toEqual([
      ['a', 0],
      ['b', 0],
    ]);
  });

  it('puts a stretch that shares a day with another on the next line', () => {
    const lanes = layoutLanes([
      { id: 'a', firstDay: 1, lastDay: 3 },
      { id: 'b', firstDay: 3, lastDay: 5 },
      { id: 'c', firstDay: 2, lastDay: 2 },
    ]);
    expect(lanes.get('a')).toBe(0);
    expect(lanes.get('b')).toBe(1);
    expect(lanes.get('c')).toBe(2);
  });

  it('goes back to the highest line that has come free', () => {
    const lanes = layoutLanes([
      { id: 'a', firstDay: 1, lastDay: 2 },
      { id: 'b', firstDay: 1, lastDay: 9 },
      { id: 'c', firstDay: 5, lastDay: 6 },
    ]);
    expect(lanes.get('c')).toBe(0);
  });

  it('lays out nothing as nothing', () => {
    expect(layoutLanes([]).size).toBe(0);
  });
});

describe('reachOf', () => {
  it('keeps the days of a planned task', () => {
    const span = spanOf({ startOn: '2026-11-10', dueOn: '2026-11-12' });
    expect(span && reachOf(span)).toBe(span && span.lastDay);
  });

  it('keeps room for the key after a task with one date', () => {
    const deadline = spanOf({ startOn: null, dueOn: '2026-11-12' });
    const open = spanOf({ startOn: '2026-11-12', dueOn: null });
    expect(deadline && reachOf(deadline)).toBe(deadline && deadline.firstDay + POINT_DAYS - 1);
    expect(open && reachOf(open)).toBe(open && open.firstDay + POINT_DAYS - 1);
  });
});

describe('timelineRows', () => {
  it('has a row for the manager and every member, in the order they joined', () => {
    const rows = timelineRows([], 'person', sources);
    expect(rows.map((row) => row.key)).toEqual(['manager', 'member:ada', 'member:ben']);
    expect(rows.every((row) => row.laneCount === 1)).toBe(true);
  });

  it('adds a last row for the tasks given to nobody, only when there are some', () => {
    const rows = timelineRows([task({ id: 'loose' })], 'person', sources);
    expect(rows.map((row) => row.key)).toEqual(['manager', 'member:ada', 'member:ben', 'nobody']);
    expect(rows[3]?.undated.map(({ id }) => id)).toEqual(['loose']);
  });

  it('puts each task in the row of its person', () => {
    const rows = timelineRows(
      [
        task({ id: 'mine', assignee: { kind: 'manager' }, dueOn: '2026-11-12' }),
        task({ id: 'hers', assignee: toAda, startOn: '2026-11-10', dueOn: '2026-11-11' }),
      ],
      'person',
      sources,
    );
    expect(rows[0]?.placed.map(({ task: { id } }) => id)).toEqual(['mine']);
    expect(rows[1]?.placed.map(({ task: { id } }) => id)).toEqual(['hers']);
    expect(rows[2]?.placed).toEqual([]);
  });

  it('stacks the tasks of a person that overlap in time', () => {
    const [, ada] = timelineRows(
      [
        task({ id: 'long', assignee: toAda, startOn: '2026-11-09', dueOn: '2026-11-13' }),
        task({ id: 'inside', assignee: toAda, startOn: '2026-11-10', dueOn: '2026-11-11' }),
        task({ id: 'after', assignee: toAda, startOn: '2026-11-14', dueOn: '2026-11-15' }),
      ],
      'person',
      sources,
    );
    expect(ada?.placed.map(({ task: { id }, lane }) => [id, lane])).toEqual([
      ['long', 0],
      ['inside', 1],
      ['after', 0],
    ]);
    expect(ada?.laneCount).toBe(2);
  });

  it('keeps room after a mark, so that its key is not covered', () => {
    const [, ada] = timelineRows(
      [
        task({ id: 'mark', assignee: toAda, dueOn: '2026-11-10' }),
        task({ id: 'next', assignee: toAda, startOn: '2026-11-11', dueOn: '2026-11-12' }),
      ],
      'person',
      sources,
    );
    expect(ada?.placed.map(({ lane }) => lane)).toEqual([0, 1]);
  });

  it('lists the tasks of a row without dates by number', () => {
    const [, ada] = timelineRows(
      [
        task({ id: 'later', number: 9, assignee: toAda }),
        task({ id: 'sooner', number: 2, assignee: toAda }),
        task({ id: 'dated', number: 1, assignee: toAda, dueOn: '2026-11-12' }),
      ],
      'person',
      sources,
    );
    expect(ada?.undated.map(({ id }) => id)).toEqual(['sooner', 'later']);
  });

  it('gives a task of a member who has left a row of its own at the end', () => {
    const rows = timelineRows(
      [task({ id: 'orphan', assignee: { kind: 'member', memberId: 'gone' } })],
      'person',
      sources,
    );
    expect(rows.at(-1)?.key).toBe('other');
    expect(rows.at(-1)?.subject).toEqual({ kind: 'other', grouping: 'person' });
  });

  it('groups by status in the order of the board, every status a row', () => {
    const rows = timelineRows(
      [task({ id: 'busy', statusId: 'doing', dueOn: '2026-11-12' })],
      'status',
      sources,
    );
    expect(rows.map((row) => row.key)).toEqual(['status:todo', 'status:doing', 'status:done']);
    expect(rows[1]?.placed.map(({ task: { id } }) => id)).toEqual(['busy']);
  });

  it('groups by area in the order of the areas', () => {
    const rows = timelineRows(
      [task({ id: 'code', areaId: 'firmware' }), task({ id: 'lost', areaId: 'gone' })],
      'area',
      sources,
    );
    expect(rows.map((row) => row.key)).toEqual(['area:general', 'area:firmware', 'other']);
    expect(rows[1]?.undated.map(({ id }) => id)).toEqual(['code']);
  });
});

describe('the measures of a row', () => {
  it('is as high as its lines and the space around them', () => {
    expect(trackHeightOf(1)).toBe(LANE_HEIGHT + 2 * ROW_PADDING);
    expect(trackHeightOf(3)).toBe(3 * LANE_HEIGHT + 2 * ROW_PADDING);
  });

  it('sets a bar in the middle of its line', () => {
    expect(barTopOf(0)).toBe(ROW_PADDING + (LANE_HEIGHT - BAR_HEIGHT) / 2);
    expect(barTopOf(2) - barTopOf(1)).toBe(LANE_HEIGHT);
  });
});
