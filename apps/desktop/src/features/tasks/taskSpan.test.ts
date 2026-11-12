import { describe, expect, it } from 'vitest';

import { dayNumberOf } from '@/shared/lib/days';

import { spanOf } from './taskSpan';

describe('spanOf', () => {
  it('runs from the start to the due date of a planned task', () => {
    expect(spanOf({ startOn: '2026-11-10', dueOn: '2026-11-14' })).toEqual({
      firstDay: dayNumberOf('2026-11-10'),
      lastDay: dayNumberOf('2026-11-14'),
      kind: 'planned',
    });
  });

  it('is one open day for a task that only has a start', () => {
    const day = dayNumberOf('2026-11-10');

    expect(spanOf({ startOn: '2026-11-10', dueOn: null })).toEqual({
      firstDay: day,
      lastDay: day,
      kind: 'open',
    });
  });

  it('is the day of the deadline for a task that only has a due date', () => {
    const day = dayNumberOf('2026-11-14');

    expect(spanOf({ startOn: null, dueOn: '2026-11-14' })).toEqual({
      firstDay: day,
      lastDay: day,
      kind: 'deadline',
    });
  });

  it('is nothing for a task without any date', () => {
    expect(spanOf({ startOn: null, dueOn: null })).toBeUndefined();
  });
});
