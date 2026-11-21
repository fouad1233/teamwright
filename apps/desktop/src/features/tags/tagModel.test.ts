import { describe, expect, it } from 'vitest';

import type { Tag } from '@/shared/api/types';

import {
  fittingTags,
  makesNewTag,
  matchingTags,
  tagsOf,
  taskCountsByTag,
  withTagSet,
} from './tagModel';

const tag = (id: string, name: string): Tag => ({ id, projectId: 'p', name, color: 'blue' });

const TAGS = [tag('cost', 'Cost'), tag('emc', 'EMC'), tag('power', 'Power')];

describe('the tags of a task', () => {
  it('are those it carries, in the order of the tags of the project', () => {
    expect(tagsOf({ tagIds: ['power', 'gone', 'cost'] }, TAGS).map((one) => one.id)).toEqual([
      'cost',
      'power',
    ]);
  });

  it('all fit in a row that has room for them, and the rest make a count', () => {
    expect(fittingTags(TAGS, 3)).toEqual({ shown: TAGS, more: 0 });
    expect(fittingTags(TAGS, 2)).toEqual({ shown: TAGS.slice(0, 1), more: 2 });
  });

  it('are put on and taken off once', () => {
    const task = { tagIds: ['emc'] };

    expect(withTagSet(task, 'cost', true).tagIds).toEqual(['emc', 'cost']);
    expect(withTagSet(task, 'emc', true).tagIds).toEqual(['emc']);
    expect(withTagSet(task, 'emc', false).tagIds).toEqual([]);
  });
});

describe('typing the name of a tag', () => {
  it('narrows the tags by a part of their name, whatever the capitals', () => {
    expect(matchingTags(TAGS, ' o').map((one) => one.name)).toEqual(['Cost', 'Power']);
    expect(matchingTags(TAGS, '')).toHaveLength(3);
  });

  it('offers to make a tag only for a name no tag has', () => {
    expect(makesNewTag(TAGS, 'emc ')).toBe(false);
    expect(makesNewTag(TAGS, '   ')).toBe(false);
    expect(makesNewTag(TAGS, 'Waiting for parts')).toBe(true);
  });
});

describe('the tasks of a tag', () => {
  it('are counted by tag', () => {
    const counts = taskCountsByTag([{ tagIds: ['emc', 'cost'] }, { tagIds: ['emc'] }]);

    expect(counts.get('emc')).toBe(2);
    expect(counts.get('cost')).toBe(1);
    expect(counts.get('power')).toBeUndefined();
  });
});
