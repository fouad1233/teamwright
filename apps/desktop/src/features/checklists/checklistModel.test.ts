import { describe, expect, it } from 'vitest';

import type { ChecklistItem } from '@/shared/api/types';

import { isComplete, progressInWords, progressOf, withMovedItem } from './checklistModel';

const item = (id: string, done: boolean): ChecklistItem => ({ id, text: id, done });

describe('the progress of a checklist', () => {
  it('counts the items done, and is nothing for a task without a checklist', () => {
    expect(progressOf({ checklist: [] })).toBeNull();
    expect(progressOf({ checklist: [item('a', true), item('b', false)] })).toEqual({
      done: 1,
      total: 2,
    });
  });

  it('is said in words and is complete once every item is done', () => {
    expect(progressInWords({ done: 3, total: 5 })).toBe('3 of 5 items done');
    expect(progressInWords({ done: 0, total: 1 })).toBe('0 of 1 item done');
    expect(isComplete({ done: 2, total: 2 })).toBe(true);
    expect(isComplete({ done: 1, total: 2 })).toBe(false);
  });
});

describe('moving an item', () => {
  it('puts it at a place among the others, the ends included', () => {
    const items = [item('a', false), item('b', false), item('c', false)];

    expect(withMovedItem(items, 'c', 0).map((one) => one.id)).toEqual(['c', 'a', 'b']);
    expect(withMovedItem(items, 'a', 9).map((one) => one.id)).toEqual(['b', 'c', 'a']);
    expect(withMovedItem(items, 'gone', 0)).toEqual(items);
  });
});
