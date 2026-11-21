import { describe, expect, it } from 'vitest';

import type { Member, Project, Task, TaskChange } from '@/shared/api/types';

import { changeAfterTaskName, changeInWords, type HistoryNames } from './taskHistoryModel';

const project: Project = {
  id: 'project',
  key: 'SSN',
  name: 'Smart sensor node',
  description: '',
  statuses: [
    { id: 'todo', name: 'To do', category: 'todo', color: 'gray' },
    { id: 'doing', name: 'In progress', category: 'active', color: 'blue' },
  ],
  createdAt: '2026-11-11T09:00:00.000Z',
  archivedAt: null,
};

const engineer: Member = {
  id: 'engineer',
  projectId: 'project',
  name: 'Hardware engineer',
  kind: 'agent',
  responsibility: '',
  model: null,
  effort: null,
  instructions: '',
  color: 'teal',
  joinedAt: '2026-11-11T09:00:00.000Z',
  updatedAt: '2026-11-11T09:00:00.000Z',
};

const pins = { id: 'pins', number: 3 } as Task;

const names: HistoryNames = { project, tasks: [pins], members: [engineer] };

const at = '2026-11-11T09:00:00.000Z';

/** A change without its moment. Written this way, it holds for each kind of change in turn. */
type WithoutMoment<Change> = Change extends unknown ? Omit<Change, 'at'> : never;

function words(change: WithoutMoment<TaskChange>): string {
  return changeInWords({ at, ...change }, names);
}

describe('changeInWords', () => {
  it('says that a task was opened, renamed or described', () => {
    expect(words({ kind: 'created' })).toBe('Opened');
    expect(words({ kind: 'renamed', from: 'Draw it', to: 'Draw the schematic' })).toBe(
      'Renamed from "Draw it"',
    );
    expect(words({ kind: 'described' })).toBe('Description changed');
  });

  it('names the statuses a task moved between', () => {
    expect(words({ kind: 'moved', fromStatusId: 'todo', toStatusId: 'doing' })).toBe(
      'Moved from To do to In progress',
    );
  });

  it('says so when a status is gone by now', () => {
    expect(words({ kind: 'moved', fromStatusId: 'removed', toStatusId: 'doing' })).toBe(
      'Moved from a status that was removed to In progress',
    );
  });

  it('names who a task was given to', () => {
    expect(
      words({ kind: 'assigned', from: null, to: { kind: 'member', memberId: 'engineer' } }),
    ).toBe('Given to Hardware engineer');
    expect(words({ kind: 'assigned', from: null, to: { kind: 'manager' } })).toBe('Given to you');
    expect(words({ kind: 'assigned', from: { kind: 'manager' }, to: null })).toBe(
      'Given to nobody',
    );
    expect(words({ kind: 'assigned', from: null, to: { kind: 'member', memberId: 'gone' } })).toBe(
      'Given to a member who has left the team',
    );
  });

  it('names the priorities', () => {
    expect(words({ kind: 'prioritized', from: 'normal', to: 'urgent' })).toBe(
      'Priority changed from Normal to Urgent',
    );
  });

  it('says whether an estimate was set, changed or cleared', () => {
    expect(words({ kind: 'estimated', from: null, to: 180 })).toBe('Estimate set to 3h');
    expect(words({ kind: 'estimated', from: 120, to: 150 })).toBe(
      'Estimate changed from 2h to 2h 30m',
    );
    expect(words({ kind: 'estimated', from: 120, to: null })).toBe('Estimate cleared');
  });

  it('says what the dates are now', () => {
    const none = { startOn: null, dueOn: null };
    const inThisYear = (date: string) => `${new Date().getFullYear().toString()}-${date}`;

    expect(
      words({
        kind: 'rescheduled',
        from: none,
        to: { startOn: inThisYear('11-10'), dueOn: inThisYear('11-14') },
      }),
    ).toBe('Planned from 10 Nov to 14 Nov');
    expect(
      words({ kind: 'rescheduled', from: none, to: { startOn: null, dueOn: inThisYear('11-14') } }),
    ).toBe('Due date set to 14 Nov');
    expect(
      words({ kind: 'rescheduled', from: none, to: { startOn: inThisYear('11-10'), dueOn: null } }),
    ).toBe('Start set to 10 Nov');
    expect(
      words({ kind: 'rescheduled', from: { startOn: null, dueOn: inThisYear('11-14') }, to: none }),
    ).toBe('Dates cleared');
  });

  it('names the tasks a task waits for, or says that one is gone', () => {
    expect(words({ kind: 'now_waits_for', taskId: 'pins' })).toBe('Now waits for SSN-3');
    expect(words({ kind: 'no_longer_waits_for', taskId: 'pins' })).toBe(
      'No longer waits for SSN-3',
    );
    expect(words({ kind: 'now_waits_for', taskId: 'gone' })).toBe(
      'Now waits for a task that was removed',
    );
  });
});

describe('changeAfterTaskName', () => {
  /** What follows the name of the task: "SSN-4 Draw the schematic …". */
  function after(change: WithoutMoment<TaskChange>): string {
    return changeAfterTaskName({ at, ...change }, names);
  }

  const none = { startOn: null, dueOn: null };
  const inThisYear = (date: string) => `${new Date().getFullYear().toString()}-${date}`;

  it('says what was done to the task', () => {
    expect(after({ kind: 'created' })).toBe('was opened');
    expect(after({ kind: 'renamed', from: 'Draw it', to: 'Draw the schematic' })).toBe(
      'was renamed from "Draw it"',
    );
    expect(
      after({ kind: 'assigned', from: null, to: { kind: 'member', memberId: 'engineer' } }),
    ).toBe('was given to Hardware engineer');
    expect(after({ kind: 'assigned', from: null, to: { kind: 'manager' } })).toBe(
      'was given to you',
    );
    expect(after({ kind: 'assigned', from: { kind: 'manager' }, to: null })).toBe(
      'was given to nobody',
    );
    expect(
      after({
        kind: 'rescheduled',
        from: none,
        to: { startOn: inThisYear('11-10'), dueOn: inThisYear('11-14') },
      }),
    ).toBe('was planned from 10 Nov to 14 Nov');
  });

  it('says what was done to a part of the task', () => {
    expect(after({ kind: 'described' })).toBe('had its description changed');
    expect(after({ kind: 'prioritized', from: 'normal', to: 'urgent' })).toBe(
      'had its priority changed from Normal to Urgent',
    );
    expect(after({ kind: 'estimated', from: null, to: 180 })).toBe('had its estimate set to 3h');
    expect(after({ kind: 'estimated', from: 120, to: 150 })).toBe(
      'had its estimate changed from 2h to 2h 30m',
    );
    expect(after({ kind: 'estimated', from: 120, to: null })).toBe('had its estimate cleared');
    expect(
      after({ kind: 'rescheduled', from: none, to: { startOn: null, dueOn: inThisYear('11-14') } }),
    ).toBe('had its due date set to 14 Nov');
    expect(
      after({ kind: 'rescheduled', from: none, to: { startOn: inThisYear('11-10'), dueOn: null } }),
    ).toBe('had its start set to 10 Nov');
    expect(
      after({ kind: 'rescheduled', from: { startOn: null, dueOn: inThisYear('11-14') }, to: none }),
    ).toBe('had its dates cleared');
  });

  it('says what the task does without a word between', () => {
    expect(after({ kind: 'moved', fromStatusId: 'todo', toStatusId: 'doing' })).toBe(
      'moved from To do to In progress',
    );
    expect(after({ kind: 'moved', fromStatusId: 'removed', toStatusId: 'doing' })).toBe(
      'moved from a status that was removed to In progress',
    );
    expect(after({ kind: 'now_waits_for', taskId: 'pins' })).toBe('now waits for SSN-3');
    expect(after({ kind: 'no_longer_waits_for', taskId: 'gone' })).toBe(
      'no longer waits for a task that was removed',
    );
  });

  it('ends in the words the history of the task lists', () => {
    const changes: WithoutMoment<TaskChange>[] = [
      { kind: 'created' },
      { kind: 'described' },
      { kind: 'moved', fromStatusId: 'todo', toStatusId: 'doing' },
      { kind: 'estimated', from: null, to: 180 },
      { kind: 'now_waits_for', taskId: 'pins' },
    ];
    for (const change of changes) {
      const listed = words(change);
      const withoutCapital = listed.charAt(0).toLowerCase() + listed.slice(1);
      expect(after(change).endsWith(withoutCapital)).toBe(true);
    }
  });
});
