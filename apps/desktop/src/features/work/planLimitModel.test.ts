import { describe, expect, it } from 'vitest';

import type { PlanLimit } from '@/shared/api/types';
import {
  planLimitLine,
  resetInWords,
  startWarning,
  tryAgainHint,
  usedInWords,
  windowName,
  windowsToShow,
} from './planLimitModel';

/** A moment on the clocks of this computer, whatever its zone. */
function local(day: number, hour: number, minute: number): Date {
  return new Date(2026, 10, day, hour, minute);
}

function reached(resetsAt: Date | null): PlanLimit {
  return {
    reached: {
      at: local(21, 20, 15).toISOString(),
      window: 'five_hour',
      resetsAt: resetsAt === null ? null : resetsAt.toISOString(),
    },
    windows: [],
    reportedAt: null,
  };
}

describe('the words of the limit of the Claude plan', () => {
  const now = local(21, 20, 15);

  it('says the one plain line with the time it resets', () => {
    expect(planLimitLine(reached(local(21, 22, 40)), now)).toBe(
      'The limit of your Claude plan is reached. It resets at 22:40.',
    );
    expect(planLimitLine(reached(null), now)).toBe(
      'The limit of your Claude plan is reached. When it resets is not known.',
    );
    expect(planLimitLine({ reached: null, windows: [], reportedAt: null }, now)).toBeNull();
    expect(planLimitLine(undefined, now)).toBeNull();
  });

  it('words a reset today, tomorrow and later', () => {
    expect(resetInWords(local(21, 22, 40).toISOString(), now)).toBe('at 22:40');
    expect(resetInWords(local(22, 0, 5).toISOString(), now)).toBe('tomorrow at 00:05');
    expect(resetInWords(local(25, 10, 0).toISOString(), now)).toBe('on 25 November at 10:00');
  });

  it('warns a manager who starts work before the reset, and says nothing otherwise', () => {
    expect(startWarning(reached(local(21, 22, 40)), now)).toBe(
      'The limit of your Claude plan is reached until 22:40. Work started before then will most likely stop at once.',
    );
    expect(startWarning(reached(local(25, 10, 0)), now)).toContain('until 25 November at 10:00.');
    expect(startWarning(reached(null), now)).toContain('if it goes through, the limit has reset');
    expect(startWarning(undefined, now)).toBeNull();
    expect(tryAgainHint(reached(local(21, 22, 40)))).toBe(
      'Before then it will most likely stop again at once.',
    );
    expect(tryAgainHint(undefined)).toBeNull();
  });

  it('shows the five hours and the week, unknown where nothing was reported', () => {
    const windows = windowsToShow({
      reached: null,
      windows: [
        { kind: 'seven_day_opus', used: 0.5, resetsAt: null, status: 'allowed' },
        { kind: 'seven_day', used: 0.214, resetsAt: null, status: 'allowed' },
      ],
      reportedAt: null,
    });
    expect(windows.map((window) => windowName(window.kind))).toEqual([
      'Five hours',
      'Week',
      'Week opus',
    ]);
    expect(windows.map(usedInWords)).toEqual(['unknown', '21 %', '50 %']);
  });
});
