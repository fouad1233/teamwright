import { describe, expect, it } from 'vitest';

import type { Assignee, Member, Task } from '@/shared/api/types';

import { indexMembers, initialsOf, tasksOfManager, tasksOfMember } from './teamModel';

function member(id: string, name: string): Member {
  return {
    id,
    projectId: 'project',
    name,
    kind: 'agent',
    responsibility: '',
    color: 'teal',
    joinedAt: '2026-11-11T09:00:00.000Z',
    updatedAt: '2026-11-11T09:00:00.000Z',
  };
}

function task(number: number, assignee: Assignee | null): Task {
  return {
    id: `task-${number.toString()}`,
    projectId: 'project',
    number,
    title: `Task ${number.toString()}`,
    description: '',
    statusId: 'todo',
    priority: 'normal',
    assignee,
    parentId: null,
    blockedBy: [],
    estimateMinutes: null,
    startOn: null,
    dueOn: null,
    rank: number,
    createdAt: '2026-11-11T09:00:00.000Z',
    updatedAt: '2026-11-11T09:00:00.000Z',
    completedAt: null,
  };
}

describe('initialsOf', () => {
  it('takes the first letters of the first two words', () => {
    expect(initialsOf('Hardware engineer')).toBe('HE');
    expect(initialsOf('Senior firmware engineer')).toBe('SF');
  });

  it('takes the first two letters of a name that is one word', () => {
    expect(initialsOf('Dana')).toBe('DA');
    expect(initialsOf('Q')).toBe('Q');
  });

  it('writes the letters in capitals, whatever the name uses', () => {
    expect(initialsOf('video editor')).toBe('VE');
    expect(initialsOf('ömer')).toBe('ÖM');
  });

  it('ignores space around and between the words', () => {
    expect(initialsOf('  Mechanical   engineer ')).toBe('ME');
  });

  it('gives nothing for a name without letters', () => {
    expect(initialsOf('   ')).toBe('');
  });
});

describe('indexMembers', () => {
  it('finds every member by its identifier', () => {
    const hardware = member('m1', 'Hardware engineer');
    const firmware = member('m2', 'Firmware engineer');

    const byId = indexMembers([hardware, firmware]);

    expect(byId.get('m1')).toBe(hardware);
    expect(byId.get('m2')).toBe(firmware);
    expect(byId.get('m3')).toBeUndefined();
  });
});

describe('the tasks of someone', () => {
  const tasks = [
    task(1, { kind: 'manager' }),
    task(2, { kind: 'member', memberId: 'm1' }),
    task(3, { kind: 'member', memberId: 'm2' }),
    task(4, null),
    task(5, { kind: 'member', memberId: 'm1' }),
  ];

  it('are the tasks given to that member and to nobody else', () => {
    expect(tasksOfMember(tasks, 'm1').map((found) => found.number)).toEqual([2, 5]);
    expect(tasksOfMember(tasks, 'm3')).toEqual([]);
  });

  it('are, for the manager, the tasks the manager keeps', () => {
    expect(tasksOfManager(tasks).map((found) => found.number)).toEqual([1]);
  });
});
