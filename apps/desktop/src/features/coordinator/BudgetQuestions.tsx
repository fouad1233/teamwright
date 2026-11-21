import { useQueryClient } from '@tanstack/react-query';

import { useBackend } from '@/shared/api/BackendProvider';
import type { BudgetQuestion } from '@/shared/api/types';
import { formatMoment } from '@/shared/lib/format';
import { Button } from '@/shared/ui/Button';

import { formatTokens } from '../work/workModel';
import styles from './Coordinator.module.css';
import { BUDGET_QUESTIONS_KEY, coordinatorKey } from './coordinatorQueries';

interface BudgetQuestionsProps {
  questions: readonly BudgetQuestion[];
}

/**
 * The questions the coordinator asks in the inbox when a budget is used up,
 * like the decision requests of the specialists: what it is about, and the
 * choices that make sense, raising it by half or by all of it again, or
 * leaving the work against it stopped.
 */
export function BudgetQuestions({ questions }: BudgetQuestionsProps) {
  const backend = useBackend();
  const queryClient = useQueryClient();

  async function answer(question: BudgetQuestion, raiseBy: number | null) {
    await backend.answerBudget(
      question.projectId,
      { scope: question.scope, targetId: question.targetId },
      raiseBy,
    );
    await queryClient.invalidateQueries({ queryKey: BUDGET_QUESTIONS_KEY });
    await queryClient.invalidateQueries({ queryKey: coordinatorKey(question.projectId) });
  }

  return (
    <ol className={styles.questions}>
      {questions.map((question) => (
        <li key={`${question.projectId}-${question.targetId}`} className={styles.question}>
          <p className={styles.questionAbout}>
            <span className={styles.subject}>{question.projectName}</span>
            <span className={styles.moment}>{formatMoment(question.askedAt)}</span>
          </p>
          <p className={styles.questionText}>
            The coordinator stopped: {question.name} is used up, {formatTokens(question.used)} of{' '}
            {formatTokens(question.tokens)} tokens. Raise it, or leave the work against it stopped?
          </p>
          <div className={styles.answers}>
            {question.raises.map((raise) => (
              <Button
                key={raise}
                variant="secondary"
                onClick={() => {
                  void answer(question, raise);
                }}
              >
                Raise by {formatTokens(raise)}
              </Button>
            ))}
            <Button
              variant="ghost"
              onClick={() => {
                void answer(question, null);
              }}
            >
              Leave it stopped
            </Button>
          </div>
        </li>
      ))}
    </ol>
  );
}
