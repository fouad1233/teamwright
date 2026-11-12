import { describe, expect, it } from 'vitest';

import type { Status } from '@/shared/api/types';

import {
  categoryLabel,
  colorName,
  isTheOnlyOneOfItsKind,
  suggestedStatus,
  withMovedStatus,
} from './workflowModel';

const todo: Status = { id: 'todo', name: 'To do', category: 'todo', color: 'gray' };
const doing: Status = { id: 'doing', name: 'In progress', category: 'active', color: 'blue' };
const testing: Status = { id: 'testing', name: 'In test', category: 'review', color: 'amber' };
const done: Status = { id: 'done', name: 'Done', category: 'done', color: 'green' };
const workflow = [todo, doing, testing, done];

function ids(statuses: Status[]): string[] {
  return statuses.map((status) => status.id);
}

describe('categoryLabel and colorName', () => {
  it('name a category and a pigment in plain words', () => {
    expect(categoryLabel('todo')).toBe('Not started');
    expect(categoryLabel('review')).toBe('In review');
    expect(colorName('teal')).toBe('Teal');
  });
});

describe('withMovedStatus', () => {
  it('moves a status to another place', () => {
    expect(ids(withMovedStatus(workflow, 'testing', 1))).toEqual([
      'todo',
      'testing',
      'doing',
      'done',
    ]);
  });

  it('treats a place past the end as the last place', () => {
    expect(ids(withMovedStatus(workflow, 'todo', 99))).toEqual([
      'doing',
      'testing',
      'done',
      'todo',
    ]);
  });

  it('changes nothing for a status the workflow does not have', () => {
    expect(ids(withMovedStatus(workflow, 'missing', 0))).toEqual(ids(workflow));
  });

  it('leaves the original list untouched', () => {
    withMovedStatus(workflow, 'done', 0);

    expect(ids(workflow)).toEqual(['todo', 'doing', 'testing', 'done']);
  });
});

describe('suggestedStatus', () => {
  it('starts a new status as a step of the work, in a pigment nobody uses', () => {
    expect(suggestedStatus(workflow)).toEqual({
      name: 'New status',
      category: 'active',
      color: 'purple',
    });
  });

  it('numbers the name when it is taken, whatever the capitals', () => {
    const withNew = [...workflow, { ...doing, id: 'new', name: 'new STATUS' }];

    expect(suggestedStatus(withNew).name).toBe('New status 2');
  });

  it('falls back to blue when every pigment for a step of the work is in use', () => {
    const colorful = (['blue', 'purple', 'teal', 'green', 'amber', 'pink'] as const).map(
      (color): Status => ({ id: color, name: color, category: 'active', color }),
    );

    expect(suggestedStatus(colorful).color).toBe('blue');
  });
});

describe('isTheOnlyOneOfItsKind', () => {
  it('is true for the only status in which work has not started, and the only done one', () => {
    expect(isTheOnlyOneOfItsKind(workflow, todo)).toBe(true);
    expect(isTheOnlyOneOfItsKind(workflow, done)).toBe(true);
  });

  it('is false for a step of the work, even if it is the only one', () => {
    expect(isTheOnlyOneOfItsKind(workflow, doing)).toBe(false);
    expect(isTheOnlyOneOfItsKind(workflow, testing)).toBe(false);
  });

  it('is false once there is a second status of the kind', () => {
    const withBacklog = [{ ...todo, id: 'backlog', name: 'Backlog' }, ...workflow];

    expect(isTheOnlyOneOfItsKind(withBacklog, todo)).toBe(false);
  });
});
