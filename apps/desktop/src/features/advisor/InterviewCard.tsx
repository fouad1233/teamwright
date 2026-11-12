import { useState, type FormEvent } from 'react';

import type { Interview, InterviewAnswer } from '@/shared/api/types';
import { classNames } from '@/shared/lib/classNames';
import { Button } from '@/shared/ui/Button';
import { TextArea } from '@/shared/ui/Field';

import { OptionList } from '../work/OptionList';
import { answersOf, emptyAnswers, isAnswered, type AnswerDraft } from './advisorModel';
import styles from './InterviewCard.module.css';

interface InterviewCardProps {
  interview: Interview;
  /** Sends the answers. Resolves to whether they were taken. */
  onAnswer: (answers: InterviewAnswer[]) => Promise<boolean>;
}

/**
 * The questions the Advisor asks before it proposes a team and a plan. Each
 * says why it is asked. Where the Advisor offers options, one can be
 * picked. Every question leaves room for the manager's own words. They are
 * answered together, so that the manager sees all of them before answering
 * the first.
 */
export function InterviewCard({ interview, onAnswer }: InterviewCardProps) {
  const [drafts, setDrafts] = useState(() => emptyAnswers(interview.questions));
  const [isSending, setIsSending] = useState(false);
  const open = drafts.filter((draft) => !isAnswered(draft)).length;
  const canSend = !isSending && open === 0;

  function change(place: number, changes: Partial<AnswerDraft>) {
    setDrafts((current) =>
      current.map((draft, other) => (other === place ? { ...draft, ...changes } : draft)),
    );
  }

  async function send() {
    if (!canSend) {
      return;
    }
    setIsSending(true);
    const wereTaken = await onAnswer(answersOf(drafts));
    if (!wereTaken) {
      setIsSending(false);
    }
  }

  function handleSubmit(event: FormEvent) {
    event.preventDefault();
    void send();
  }

  return (
    <form className={styles.card} onSubmit={handleSubmit}>
      <p className={classNames('label', styles.asks)}>
        The Advisor asks{' '}
        {interview.questions.length === 1
          ? 'one question'
          : `${interview.questions.length.toString()} questions`}
      </p>

      <ol className={styles.questions}>
        {interview.questions.map((question, place) => {
          const draft = drafts[place] ?? { pick: undefined, note: '' };
          const number = (place + 1).toString();
          const hasOptions = question.options.length > 0;
          const answersInWords = !hasOptions || draft.pick === 'own words';
          return (
            // The questions of an interview never change, so their place identifies them.
            <li key={place} className={styles.question}>
              <span className={styles.number} aria-hidden>
                {number}
              </span>
              <div className={styles.body}>
                <h4 className={styles.text} data-selectable>
                  {question.question}
                </h4>
                {question.reason !== '' && (
                  <p className={styles.reason} data-selectable>
                    {question.reason}
                  </p>
                )}
                {hasOptions && (
                  <OptionList
                    label={`Options of question ${number}`}
                    options={question.options}
                    recommended={question.recommended}
                    pick={draft.pick}
                    onPick={(pick) => {
                      change(place, { pick });
                    }}
                  />
                )}
                <TextArea
                  className={styles.note}
                  aria-label={`Your words on question ${number}`}
                  placeholder={
                    answersInWords ? 'Your answer, in your own words…' : 'Add a note, if you like…'
                  }
                  value={draft.note}
                  rows={2}
                  onChange={(event) => {
                    change(place, { note: event.target.value });
                  }}
                />
              </div>
            </li>
          );
        })}
      </ol>

      <div className={styles.actions}>
        {open > 0 && (
          <span className={styles.left}>
            {open === 1
              ? '1 question is still open'
              : `${open.toString()} questions are still open`}
          </span>
        )}
        <Button variant="primary" type="submit" disabled={!canSend}>
          Send answers
        </Button>
      </div>
    </form>
  );
}
