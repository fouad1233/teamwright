import { describe, expect, it } from 'vitest';

import type { Status, Task } from '@/shared/api/types';

import { arrangementOf, locate, moveCard, statusUnder, type Arrangement } from './boardModel';

const statuses: Status[] = [
  { id: 'todo', name: 'To do', category: 'todo', color: 'gray' },
  { id: 'doing', name: 'In progress', category: 'active', color: 'blue' },
  { id: 'done', name: 'Done', category: 'done', color: 'green' },
];

function arrangement(columns: Record<string, string[]>): Arrangement {
  return new Map(Object.entries(columns));
}

function columns(arranged: Arrangement): Record<string, readonly string[]> {
  return Object.fromEntries(arranged);
}

const board = arrangement({ todo: ['a', 'b', 'c'], doing: ['x', 'y'], done: [] });

describe('arrangementOf', () => {
  it('lists the tasks of each status by rank and keeps empty statuses', () => {
    const tasks = [
      { id: 'second', statusId: 'todo', rank: 20, number: 2, parentId: null },
      { id: 'first', statusId: 'todo', rank: 10, number: 1, parentId: null },
      { id: 'working', statusId: 'doing', rank: 0, number: 3, parentId: null },
    ] as Task[];

    expect(columns(arrangementOf(statuses, tasks))).toEqual({
      todo: ['first', 'second'],
      doing: ['working'],
      done: [],
    });
  });

  it('gives a subtask no place of its own, whatever its status', () => {
    const tasks = [
      { id: 'parent', statusId: 'todo', rank: 10, number: 1, parentId: null },
      { id: 'part', statusId: 'doing', rank: 0, number: 2, parentId: 'parent' },
    ] as Task[];

    expect(columns(arrangementOf(statuses, tasks))).toEqual({
      todo: ['parent'],
      doing: [],
      done: [],
    });
  });
});

describe('locate', () => {
  it('finds the status and the position of a card', () => {
    expect(locate(board, 'y')).toEqual({ statusId: 'doing', index: 1 });
  });

  it('finds nothing for a card that is not on the board', () => {
    expect(locate(board, 'missing')).toBeUndefined();
  });
});

describe('statusUnder', () => {
  it('is the status itself when the target is a status', () => {
    expect(statusUnder(board, 'done')).toBe('done');
  });

  it('is the status of the card when the target is a card', () => {
    expect(statusUnder(board, 'x')).toBe('doing');
  });
});

describe('moveCard', () => {
  it('puts a card dropped on a card of another status in that card’s place', () => {
    const moved = moveCard(board, 'a', 'y');

    expect(columns(moved)).toEqual({ todo: ['b', 'c'], doing: ['x', 'a', 'y'], done: [] });
  });

  it('puts a card dropped on a status at the bottom of it', () => {
    const moved = moveCard(board, 'a', 'doing');

    expect(columns(moved).doing).toEqual(['x', 'y', 'a']);
  });

  it('puts a card dropped on an empty status into it', () => {
    const moved = moveCard(board, 'x', 'done');

    expect(columns(moved)).toEqual({ todo: ['a', 'b', 'c'], doing: ['y'], done: ['x'] });
  });

  it('moves a card down within its status to below the card it is dropped on', () => {
    const moved = moveCard(board, 'a', 'c');

    expect(columns(moved).todo).toEqual(['b', 'c', 'a']);
  });

  it('moves a card up within its status to above the card it is dropped on', () => {
    const moved = moveCard(board, 'c', 'a');

    expect(columns(moved).todo).toEqual(['c', 'a', 'b']);
  });

  it('changes nothing when a card is dropped on itself', () => {
    expect(moveCard(board, 'b', 'b')).toBe(board);
  });

  it('changes nothing when the target is not on the board', () => {
    expect(moveCard(board, 'b', 'missing')).toBe(board);
  });

  it('leaves the original arrangement untouched', () => {
    moveCard(board, 'a', 'doing');

    expect(columns(board).todo).toEqual(['a', 'b', 'c']);
  });
});
