import { describe, expect, it } from 'vitest';

import { isStaleAfterChanges } from '../sync/syncModel';
import { viewsKey } from './viewQueries';

describe('the views when another computer changed a project', () => {
  it('asks again for the views of the projects it changed, and only those', () => {
    expect(isStaleAfterChanges(viewsKey('p'), ['p'])).toBe(true);
    expect(isStaleAfterChanges(viewsKey('q'), ['p'])).toBe(false);
  });
});
