import { describe, expect, it } from 'vitest';

import type { Coordinator } from '@/shared/api/types';

import { budgetInWords, parseTokens, runsInWords, shareOfBudget } from './coordinatorModel';

const COORDINATOR: Coordinator = {
  projectId: 'p',
  runs: 'here',
  runsOnName: null,
  switchedAt: null,
  limit: 2,
  leastLimit: 1,
  mostLimit: 8,
  atWork: 1,
  budgets: [],
  spent: { project: 0, members: [], tasks: [] },
  waiting: [],
  entries: [],
};

describe('the words of the coordinator', () => {
  it('reads the numbers of tokens a manager types', () => {
    expect(parseTokens('2M')).toBe(2_000_000);
    expect(parseTokens('500k')).toBe(500_000);
    expect(parseTokens('1.5 M')).toBe(1_500_000);
    expect(parseTokens('1,000,000')).toBe(1_000_000);
    expect(parseTokens('  ')).toBeNull();
    expect(parseTokens('lots')).toBeUndefined();
  });

  it('says where it runs', () => {
    expect(runsInWords(COORDINATOR)).toBe(
      'On, on this computer. At most 2 specialists at work at once; 1 at work now.',
    );
    expect(runsInWords({ ...COORDINATOR, runs: 'elsewhere', runsOnName: 'Desk' })).toMatch(
      /^On, on Desk\./,
    );
    expect(runsInWords({ ...COORDINATOR, runs: 'off' })).toBe(
      'Off. Work starts only when you start it.',
    );
  });

  it('draws a budget against what was spent, full at most', () => {
    const budget = {
      scope: 'project' as const,
      targetId: 'p',
      tokens: 2_000_000,
      used: 3_000_000,
      askedAt: null,
      raises: [1_000_000, 2_000_000] as [number, number],
    };
    expect(shareOfBudget(budget)).toBe(1);
    expect(budgetInWords(1_200_000, budget)).toBe('1.2M of 2M tokens');
    expect(budgetInWords(1_200_000, undefined)).toBe('1.2M tokens used, no budget');
  });
});
