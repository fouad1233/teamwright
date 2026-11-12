import { describe, expect, it } from 'vitest';

import { isStaleAfterChanges } from '../sync/syncModel';
import { areasKey } from './areaQueries';

describe('the areas when another computer changed a project', () => {
  it('asks again for the areas of the projects it changed, and only those', () => {
    expect(isStaleAfterChanges(areasKey('p'), ['p'])).toBe(true);
    expect(isStaleAfterChanges(areasKey('q'), ['p'])).toBe(false);
  });
});
