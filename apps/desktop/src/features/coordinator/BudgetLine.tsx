import { useState, type CSSProperties } from 'react';

import type { BudgetScope, Coordinator } from '@/shared/api/types';
import { classNames } from '@/shared/lib/classNames';
import { Button } from '@/shared/ui/Button';
import { TextInput } from '@/shared/ui/Field';

import { formatTokens } from '../work/workModel';
import styles from './Coordinator.module.css';
import { budgetInWords, budgetOf, parseTokens, shareOfBudget, spentOn } from './coordinatorModel';

interface BudgetLineProps {
  coordinator: Coordinator | undefined;
  scope: BudgetScope;
  /** What the budget is for, before the figures: "The project", "SSN-7". */
  label: string;
  onSet: (scope: BudgetScope, tokens: number | null) => Promise<unknown>;
}

/**
 * A budget in tokens with what was spent against it, in the manner of the
 * meter of a context: a line of words over a hairline that fills. Used up,
 * it is in the color of danger. "Set" opens a field for the number.
 */
export function BudgetLine({ coordinator, scope, label, onSet }: BudgetLineProps) {
  const budget = budgetOf(coordinator, scope.scope, scope.targetId);
  const used = spentOn(coordinator, scope.scope, scope.targetId);
  const [text, setText] = useState<string | null>(null);
  const [problem, setProblem] = useState<string | null>(null);
  const isUsedUp = budget !== undefined && used >= budget.tokens;

  async function save() {
    const tokens = parseTokens(text ?? '');
    if (tokens === undefined) {
      setProblem('Write a number of tokens, such as 500k or 2M.');
      return;
    }
    try {
      await onSet(scope, tokens);
      setText(null);
      setProblem(null);
    } catch (failure) {
      setProblem(failure instanceof Error ? failure.message : String(failure));
    }
  }

  return (
    <div className={classNames(styles.budget, isUsedUp && styles.usedUp)}>
      <div className={styles.budgetLine}>
        <span className={styles.budgetLabel}>{label}</span>
        <span className={styles.amount}>{budgetInWords(used, budget)}</span>
        {text === null && (
          <Button
            variant="ghost"
            onClick={() => {
              setText(budget ? formatTokens(budget.tokens) : '');
            }}
          >
            {budget ? 'Change' : 'Set a budget'}
          </Button>
        )}
      </div>
      {budget && (
        <span
          className={styles.track}
          aria-hidden
          style={{ '--share': shareOfBudget(budget) } as CSSProperties}
        >
          <span className={styles.fill} />
        </span>
      )}
      {text !== null && (
        <form
          className={styles.budgetForm}
          onSubmit={(event) => {
            event.preventDefault();
            void save();
          }}
        >
          <TextInput
            aria-label={`Budget of ${label} in tokens`}
            value={text}
            placeholder="For example 2M"
            autoFocus
            onChange={(event) => {
              setText(event.target.value);
            }}
          />
          <Button type="submit" variant="primary">
            Save
          </Button>
          {budget && (
            <Button
              variant="ghost"
              onClick={() => {
                setText('');
                void onSet(scope, null).then(() => {
                  setText(null);
                });
              }}
            >
              Take away
            </Button>
          )}
          <Button
            variant="ghost"
            onClick={() => {
              setText(null);
              setProblem(null);
            }}
          >
            Cancel
          </Button>
        </form>
      )}
      {problem !== null && <p className={styles.problem}>{problem}</p>}
    </div>
  );
}
