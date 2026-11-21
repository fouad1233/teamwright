import type { Budget, BudgetScope, Coordinator, Id } from '@/shared/api/types';

import { formatTokens } from '../work/workModel';

/**
 * What the interface says about the coordinator. Every figure comes from
 * the backend; these functions only word and draw it.
 */

/** The one sentence that says what switching the coordinator on means. */
export const WHAT_IT_DOES =
  'When it is on, ready tasks start by themselves and spend your plan without a press of a button.';

/** Which tokens a budget counts, in one sentence. */
export const WHAT_A_BUDGET_COUNTS =
  'A budget counts every token in all, those read again from the cache included, as the Usage table does, once a turn has ended. A running turn is never stopped by it.';

/** Where the coordinator runs, in words. */
export function runsInWords(coordinator: Coordinator): string {
  switch (coordinator.runs) {
    case 'here':
      return `On, on this computer. At most ${coordinator.limit.toString()} ${
        coordinator.limit === 1 ? 'specialist' : 'specialists'
      } at work at once; ${coordinator.atWork.toString()} at work now.`;
    case 'elsewhere':
      return `On, on ${coordinator.runsOnName ?? 'another computer'}. It starts work there; switching it on here moves it here.`;
    case 'off':
      return 'Off. Work starts only when you start it.';
  }
}

/** The budget of a scope, if one is set. */
export function budgetOf(
  coordinator: Coordinator | undefined,
  scope: BudgetScope['scope'],
  targetId: Id,
): Budget | undefined {
  return coordinator?.budgets.find(
    (budget) => budget.scope === scope && budget.targetId === targetId,
  );
}

/** What was spent against a scope, whether or not it has a budget. */
export function spentOn(
  coordinator: Coordinator | undefined,
  scope: BudgetScope['scope'],
  targetId: Id,
): number {
  if (!coordinator) {
    return 0;
  }
  if (scope === 'project') {
    return coordinator.spent.project;
  }
  const list = scope === 'member' ? coordinator.spent.members : coordinator.spent.tasks;
  return list.find((spent) => spent.id === targetId)?.tokens ?? 0;
}

/** How full a budget is, from 0 to 1, for a meter. */
export function shareOfBudget(budget: Budget): number {
  return budget.tokens <= 0 ? 1 : Math.min(budget.used / budget.tokens, 1);
}

/** "1.2M of 2M tokens", or "1.2M used, no budget". */
export function budgetInWords(used: number, budget: Budget | undefined): string {
  return budget
    ? `${formatTokens(used)} of ${formatTokens(budget.tokens)} tokens`
    : `${formatTokens(used)} tokens used, no budget`;
}

/**
 * Reads a number of tokens the manager typed: digits with commas or
 * spaces, or a number with k or M after it ("500k", "2M", "1.5 M").
 * Nothing for text that is not one; an empty field is null.
 */
export function parseTokens(text: string): number | null | undefined {
  const trimmed = text.trim().replace(/[,\s_]/g, '');
  if (trimmed === '') {
    return null;
  }
  const match = /^(\d+(?:\.\d+)?)([kKmM]?)$/.exec(trimmed);
  if (!match) {
    return undefined;
  }
  const scale = { '': 1, k: 1_000, K: 1_000, m: 1_000_000, M: 1_000_000 }[match[2] ?? ''] ?? 1;
  return Math.round(Number(match[1]) * scale);
}
