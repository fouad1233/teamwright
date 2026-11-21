import { describe, expect, it } from 'vitest';

import type { Member, Status, Task } from '@/shared/api/types';
import { dayNumberOf } from '@/shared/lib/days';

import {
  DEFAULT_HOURS_PER_DAY,
  EMPTY_CONTENT,
  WEEKS_SHOWN,
  WEEKS_SHOWN_BY_DAY,
  columnsOf,
  fillOf,
  formatHours,
  largestUnboundedCell,
  layingOf,
  parseContent,
  parseHoursPerDay,
  serializeContent,
  unlaidSentence,
  workloadOf,
  type Laying,
  type WorkloadContent,
  type WorkloadSources,
} from './workloadModel';

/** A Wednesday. The Monday of its week is 16 November 2026. */
const TODAY = dayNumberOf('2026-11-18');
const MONDAY = dayNumberOf('2026-11-16');

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

function member(id: string, kind: Member['kind'] = 'agent'): Member {
  return {
    id,
    projectId: 'project',
    name: id,
    kind,
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
  { id: 'done', name: 'Done', category: 'done', color: 'green' },
];

function sources(
  everyTask: readonly Task[],
  overrides: Partial<WorkloadSources> = {},
): WorkloadSources {
  return {
    project: { statuses },
    members: [member('firmware'), member('drafter', 'human')],
    everyTask,
    content: EMPTY_CONTENT,
    today: TODAY,
    ...overrides,
  };
}

const HOUR = 60;

describe('the content of the view', () => {
  it('falls back to an empty page for anything it cannot read', () => {
    for (const text of [null, '', '  ', 'not json', '42', 'null', '[]', '{"scale": 3}']) {
      expect(parseContent(text)).toEqual(EMPTY_CONTENT);
    }
  });

  it('keeps the hours that make sense and leaves out the rest', () => {
    const content = parseContent(
      JSON.stringify({
        scale: 'days',
        hoursPerDay: { manager: 7.5, 'member:a': 'six', 'member:b': 0, 'member:c': 99 },
        somethingNewer: true,
      }),
    );
    expect(content).toEqual({ scale: 'days', hoursPerDay: { manager: 7.5 } });
  });

  it('reads back what it writes', () => {
    const content: WorkloadContent = { scale: 'days', hoursPerDay: { manager: 4 } };
    expect(parseContent(serializeContent(content))).toEqual(content);
  });
});

describe('hours', () => {
  it('reads the hours a day the way they are typed', () => {
    expect(parseHoursPerDay('6')).toBe(6);
    expect(parseHoursPerDay(' 7,5 ')).toBe(7.5);
    expect(parseHoursPerDay('8h')).toBe(8);
    expect(parseHoursPerDay('0')).toBeUndefined();
    expect(parseHoursPerDay('25')).toBeUndefined();
    expect(parseHoursPerDay('six')).toBeUndefined();
  });

  it('writes minutes as hours to a tenth, and a little work never as nothing', () => {
    expect(formatHours(0)).toBe('0');
    expect(formatHours(6 * HOUR)).toBe('6');
    expect(formatHours(96)).toBe('1.6');
    expect(formatHours(1)).toBe('0.1');
  });
});

describe('the columns', () => {
  it('are eight weeks from the Monday of this week', () => {
    const columns = columnsOf('weeks', TODAY);
    expect(columns).toHaveLength(WEEKS_SHOWN);
    expect(columns[0]).toMatchObject({ firstDay: MONDAY, lastDay: MONDAY + 6, holdsToday: true });
    expect(columns[1]?.firstDay).toBe(MONDAY + 7);
  });

  it('begin with next week on a Sunday, when no work is left in this one', () => {
    const sunday = dayNumberOf('2026-11-22');
    const columns = columnsOf('weeks', sunday);
    expect(columns[0]).toMatchObject({ firstDay: MONDAY + 7, holdsToday: false, isPast: false });
  });

  it('are the days of four weeks, with the past and the weekends marked', () => {
    const columns = columnsOf('days', TODAY);
    expect(columns).toHaveLength(WEEKS_SHOWN_BY_DAY * 7);
    expect(columns[0]).toMatchObject({ isPast: true, isRestDay: false });
    expect(columns[2]).toMatchObject({ holdsToday: true, isPast: false });
    expect(columns[5]?.isRestDay).toBe(true);
  });
});

describe('laying a task on the days', () => {
  it('spreads the estimate evenly over the working days from start to due', () => {
    // Thursday to the Tuesday after: four working days.
    const laying = layingOf(
      { estimateMinutes: 8 * HOUR, startOn: '2026-11-19', dueOn: '2026-11-24' },
      TODAY,
    ) as Laying;
    expect(laying.days).toHaveLength(4);
    expect(laying.minutesPerDay).toBe(2 * HOUR);
    expect(laying.isLate).toBe(false);
  });

  it('starts today when the task has no start or began earlier', () => {
    const noStart = layingOf(
      { estimateMinutes: 3 * HOUR, startOn: null, dueOn: '2026-11-20' },
      TODAY,
    );
    const earlier = layingOf(
      { estimateMinutes: 3 * HOUR, startOn: '2026-11-02', dueOn: '2026-11-20' },
      TODAY,
    );
    expect((noStart as Laying).days[0]).toBe(TODAY);
    expect(earlier).toEqual(noStart);
  });

  it('lays a late task whole on today', () => {
    expect(
      layingOf({ estimateMinutes: 5 * HOUR, startOn: null, dueOn: '2026-11-10' }, TODAY),
    ).toEqual({ days: [TODAY], minutesPerDay: 5 * HOUR, isLate: true });
  });

  it('uses the weekend only for a task that lies on nothing else', () => {
    const laying = layingOf(
      { estimateMinutes: 4 * HOUR, startOn: '2026-11-21', dueOn: '2026-11-22' },
      TODAY,
    ) as Laying;
    expect(laying.days).toHaveLength(2);
    expect(laying.minutesPerDay).toBe(2 * HOUR);
  });

  it('says why a task cannot be laid', () => {
    expect(layingOf({ estimateMinutes: null, startOn: null, dueOn: '2026-11-20' }, TODAY)).toBe(
      'no estimate',
    );
    expect(layingOf({ estimateMinutes: 60, startOn: '2026-11-20', dueOn: null }, TODAY)).toBe(
      'no due date',
    );
  });
});

describe('the workload', () => {
  it('has a row for the manager and every member, and adds nobody only when it has work', () => {
    const tasks = [
      task({ id: 'mine', assignee: { kind: 'manager' }, estimateMinutes: 60, dueOn: '2026-11-18' }),
    ];
    expect(workloadOf(tasks, sources(tasks)).rows.map((row) => row.key)).toEqual([
      'manager',
      'member:firmware',
      'member:drafter',
    ]);

    const withNobody = [...tasks, task({ id: 'free', estimateMinutes: 60, dueOn: '2026-11-18' })];
    expect(workloadOf(withNobody, sources(withNobody)).rows.at(-1)?.key).toBe('nobody');
  });

  it('puts the hours of a task into the weeks its days fall in', () => {
    // Four working days, two in this week and two in the next.
    const tasks = [
      task({
        id: 'board',
        assignee: { kind: 'member', memberId: 'firmware' },
        estimateMinutes: 8 * HOUR,
        startOn: '2026-11-19',
        dueOn: '2026-11-24',
      }),
    ];
    const row = workloadOf(tasks, sources(tasks)).rows[1];
    expect(row?.cells[0]?.minutes).toBe(4 * HOUR);
    expect(row?.cells[1]?.minutes).toBe(4 * HOUR);
    expect(row?.cells[0]?.shares[0]?.task.id).toBe('board');
    expect(row?.totalMinutes).toBe(8 * HOUR);
  });

  it('gives a working day to the manager and to people, and none to a specialist', () => {
    const tasks: Task[] = [];
    const content: WorkloadContent = { scale: 'weeks', hoursPerDay: { 'member:drafter': 4 } };
    const [manager, specialist, person] = workloadOf(tasks, sources(tasks, { content })).rows;
    // Wednesday to Friday are left of this week: three working days.
    expect(manager?.hoursPerDay).toBe(DEFAULT_HOURS_PER_DAY);
    expect(manager?.cells[0]?.capacityMinutes).toBe(3 * DEFAULT_HOURS_PER_DAY * HOUR);
    expect(manager?.cells[1]?.capacityMinutes).toBe(5 * DEFAULT_HOURS_PER_DAY * HOUR);
    expect(person?.cells[1]?.capacityMinutes).toBe(5 * 4 * HOUR);
    expect(specialist?.hoursPerDay).toBeNull();
    expect(specialist?.cells[0]?.capacityMinutes).toBeNull();
    expect(specialist?.totalCapacityMinutes).toBeNull();
  });

  it('leaves out finished tasks, and counts a task with estimated parts through its parts', () => {
    const tasks = [
      task({ id: 'whole', number: 1, estimateMinutes: 8 * HOUR, dueOn: '2026-11-20' }),
      task({
        id: 'part',
        number: 2,
        parentId: 'whole',
        estimateMinutes: 3 * HOUR,
        dueOn: '2026-11-20',
      }),
      task({
        id: 'over',
        number: 3,
        statusId: 'done',
        estimateMinutes: 9 * HOUR,
        dueOn: '2026-11-20',
      }),
    ];
    const workload = workloadOf(tasks, sources(tasks));
    expect(workload.rows.at(-1)?.totalMinutes).toBe(3 * HOUR);
    expect(workload.unlaid).toEqual([]);
  });

  it('lists the tasks that cannot be laid, by number', () => {
    const tasks = [
      task({ id: 'b', number: 4, estimateMinutes: 60 }),
      task({ id: 'a', number: 2, dueOn: '2026-11-20' }),
    ];
    expect(workloadOf(tasks, sources(tasks)).unlaid).toEqual([
      { task: tasks[1], reason: 'no estimate' },
      { task: tasks[0], reason: 'no due date' },
    ]);
  });

  it('keeps a task of someone who has left in a row of its own', () => {
    const tasks = [
      task({
        id: 'orphan',
        assignee: { kind: 'member', memberId: 'gone' },
        estimateMinutes: 60,
        dueOn: '2026-11-18',
      }),
    ];
    expect(workloadOf(tasks, sources(tasks)).rows.at(-1)?.subject).toEqual({ kind: 'other' });
  });
});

describe('the bars', () => {
  it('fill against the capacity, and past it when there is too much', () => {
    expect(fillOf({ minutes: 90, capacityMinutes: 180 })).toBe(0.5);
    expect(fillOf({ minutes: 240, capacityMinutes: 180 })).toBeGreaterThan(1);
    expect(fillOf({ minutes: 60, capacityMinutes: 0 })).toBe(Infinity);
    expect(fillOf({ minutes: 0, capacityMinutes: 0 })).toBe(0);
    expect(fillOf({ minutes: 60, capacityMinutes: null })).toBe(0);
  });

  it('of the specialists are measured against the fullest cell among them', () => {
    const tasks = [
      task({
        id: 'a',
        assignee: { kind: 'member', memberId: 'firmware' },
        estimateMinutes: 5 * HOUR,
        dueOn: '2026-11-18',
      }),
      task({
        id: 'b',
        assignee: { kind: 'manager' },
        estimateMinutes: 9 * HOUR,
        dueOn: '2026-11-18',
      }),
    ];
    expect(largestUnboundedCell(workloadOf(tasks, sources(tasks)).rows)).toBe(5 * HOUR);
  });
});

describe('the sentence about the tasks that cannot be laid', () => {
  it('counts each reason', () => {
    expect(unlaidSentence([])).toBeNull();
    expect(
      unlaidSentence([
        ...Array<{ reason: 'no estimate' }>(4).fill({ reason: 'no estimate' }),
        ...Array<{ reason: 'no due date' }>(3).fill({ reason: 'no due date' }),
      ]),
    ).toBe('7 tasks cannot be laid on the days: 4 have no estimate, 3 no due date.');
    expect(unlaidSentence([{ reason: 'no due date' }])).toBe(
      '1 task cannot be laid on the days: it has no due date.',
    );
    expect(unlaidSentence([{ reason: 'no estimate' }, { reason: 'no estimate' }])).toBe(
      '2 tasks cannot be laid on the days: none of them have an estimate.',
    );
  });
});
