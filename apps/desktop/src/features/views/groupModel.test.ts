import { describe, expect, it } from 'vitest';

import type { GroupKey, Sprint, Task, TaskGroup } from '@/shared/api/types';

import {
  canDropOn,
  dropEffect,
  groupId,
  groupName,
  isGroupable,
  withCardMoved,
  type GroupNames,
} from './groupModel';

function task(tagIds: string[] = []): Task {
  return { id: 'task', tagIds } as Task;
}

const NAMES: GroupNames = {
  statuses: [],
  members: [{ id: 'm1', name: 'Hardware engineer' } as GroupNames['members'][number]],
  areas: [{ id: 'a1', projectId: 'p', name: 'Firmware' }],
  sprints: [],
  tags: [{ id: 't1', projectId: 'p', name: 'EMC', color: 'blue' }],
};

describe('what a group is called', () => {
  it('names each kind of group', () => {
    const named = (key: GroupKey) => groupName(key, NAMES);

    expect(named({ kind: 'manager' })).toBe('You');
    expect(named({ kind: 'member', id: 'm1' })).toBe('Hardware engineer');
    expect(named({ kind: 'nobody' })).toBe('Unassigned');
    expect(named({ kind: 'priority', priority: 'urgent' })).toBe('Urgent');
    expect(named({ kind: 'area', id: 'a1' })).toBe('Firmware');
    expect(named({ kind: 'noArea' })).toBe('No area');
    expect(named({ kind: 'tag', id: 't1' })).toBe('EMC');
    expect(named({ kind: 'noTag' })).toBe('No tag');
    expect(named({ kind: 'backlog' })).toBe('Backlog');
  });

  it('gives every group an identifier of its own', () => {
    const keys: GroupKey[] = [
      { kind: 'status', id: 'x' },
      { kind: 'manager' },
      { kind: 'member', id: 'x' },
      { kind: 'nobody' },
      { kind: 'priority', priority: 'low' },
      { kind: 'area', id: 'x' },
      { kind: 'noArea' },
      { kind: 'tag', id: 'x' },
      { kind: 'noTag' },
      { kind: 'sprint', id: 'x' },
      { kind: 'backlog' },
    ];

    expect(new Set(keys.map(groupId)).size).toBe(keys.length);
  });

  it('can group only a board and a list', () => {
    expect(isGroupable('board')).toBe(true);
    expect(isGroupable('list')).toBe(true);
    expect(isGroupable('gantt')).toBe(false);
  });
});

describe('which groups take a card', () => {
  const over = { id: 's1', state: 'completed' } as Sprint;
  const planned = { id: 's2', state: 'planned' } as Sprint;

  it('takes none for a sprint that is over or an area that is gone', () => {
    expect(canDropOn({ kind: 'sprint', id: 's1' }, [over, planned])).toBe(false);
    expect(canDropOn({ kind: 'noArea' }, [])).toBe(false);
  });

  it('takes a card for every other group', () => {
    expect(canDropOn({ kind: 'sprint', id: 's2' }, [over, planned])).toBe(true);
    expect(canDropOn({ kind: 'backlog' }, [])).toBe(true);
    expect(canDropOn({ kind: 'noTag' }, [])).toBe(true);
    expect(canDropOn({ kind: 'nobody' }, [])).toBe(true);
  });
});

describe('what dropping a card on a group changes', () => {
  it('gives the task to the one of the lane', () => {
    expect(dropEffect(task(), { kind: 'nobody' }, { kind: 'manager' })).toEqual({
      kind: 'assign',
      assignee: { kind: 'manager' },
    });
    expect(dropEffect(task(), { kind: 'manager' }, { kind: 'member', id: 'm1' })).toEqual({
      kind: 'assign',
      assignee: { kind: 'member', memberId: 'm1' },
    });
    expect(dropEffect(task(), { kind: 'manager' }, { kind: 'nobody' })).toEqual({
      kind: 'assign',
      assignee: null,
    });
  });

  it('sets the priority, the area and the sprint of the lane, and the backlog is no sprint', () => {
    const from: GroupKey = { kind: 'priority', priority: 'normal' };
    expect(dropEffect(task(), from, { kind: 'priority', priority: 'urgent' })).toEqual({
      kind: 'prioritize',
      priority: 'urgent',
    });
    expect(dropEffect(task(), { kind: 'area', id: 'a' }, { kind: 'area', id: 'b' })).toEqual({
      kind: 'move-to-area',
      areaId: 'b',
    });
    expect(dropEffect(task(), { kind: 'backlog' }, { kind: 'sprint', id: 's' })).toEqual({
      kind: 'plan',
      sprintId: 's',
    });
    expect(dropEffect(task(), { kind: 'sprint', id: 's' }, { kind: 'backlog' })).toEqual({
      kind: 'plan',
      sprintId: null,
    });
  });

  it('swaps the tag of the lane it came from for the tag of the lane it went to', () => {
    expect(dropEffect(task(['a']), { kind: 'tag', id: 'a' }, { kind: 'tag', id: 'b' })).toEqual({
      kind: 'tags',
      put: 'b',
      takeOff: 'a',
    });
  });

  it('does not put a tag on a task that has it already', () => {
    expect(
      dropEffect(task(['a', 'b']), { kind: 'tag', id: 'a' }, { kind: 'tag', id: 'b' }),
    ).toEqual({ kind: 'tags', put: null, takeOff: 'a' });
  });

  it('only puts a tag on from "No tag", and only takes one off to it', () => {
    expect(dropEffect(task(), { kind: 'noTag' }, { kind: 'tag', id: 'b' })).toEqual({
      kind: 'tags',
      put: 'b',
      takeOff: null,
    });
    expect(dropEffect(task(['a']), { kind: 'tag', id: 'a' }, { kind: 'noTag' })).toEqual({
      kind: 'tags',
      put: null,
      takeOff: 'a',
    });
  });

  it('changes nothing for a drop on the same lane or on a lane that is not offered', () => {
    expect(dropEffect(task(), { kind: 'manager' }, { kind: 'manager' })).toBeUndefined();
    expect(dropEffect(task(), { kind: 'area', id: 'a' }, { kind: 'noArea' })).toBeUndefined();
    expect(
      dropEffect(task(), { kind: 'status', id: 'a' }, { kind: 'status', id: 'b' }),
    ).toBeUndefined();
  });
});

describe('the groups once a card is dropped', () => {
  const groups: TaskGroup[] = [
    { key: { kind: 'manager' }, taskIds: ['task', 'other'] },
    { key: { kind: 'nobody' }, taskIds: [] },
  ];

  it('takes the card out of its lane and puts it at the bottom of the other', () => {
    const moved = withCardMoved(groups, 'task', { kind: 'manager' }, { kind: 'nobody' });

    expect(moved.map((group) => group.taskIds)).toEqual([['other'], ['task']]);
  });

  it('does not put a card in a lane it is in already', () => {
    const tagged: TaskGroup[] = [
      { key: { kind: 'tag', id: 'a' }, taskIds: ['task'] },
      { key: { kind: 'tag', id: 'b' }, taskIds: ['task'] },
    ];

    const moved = withCardMoved(tagged, 'task', { kind: 'tag', id: 'a' }, { kind: 'tag', id: 'b' });

    expect(moved.map((group) => group.taskIds)).toEqual([[], ['task']]);
  });
});
