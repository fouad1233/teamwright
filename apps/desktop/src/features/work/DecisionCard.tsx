import { useState, type FormEvent } from 'react';

import type { DecisionRequest, NewDecisionAnswer } from '@/shared/api/types';
import { classNames } from '@/shared/lib/classNames';
import { Button } from '@/shared/ui/Button';

import styles from './DecisionCard.module.css';
import { OptionList, type OptionPick } from './OptionList';

interface DecisionCardProps {
  decision: DecisionRequest;
  /** The name of the specialist that asks. */
  askedBy: string;
  /** Sends the answer. Resolves to whether it was taken. */
  onDecide: (answer: NewDecisionAnswer) => Promise<boolean>;
}

/**
 * A question that is the manager's to answer: what is asked and why, the
 * options with what each leads to, and room for words of the manager's own.
 *
 * It is the one thing on its page that waits for the manager, so it is the
 * one thing there that carries the gold, and the gilded sun of decision
 * 0015.
 */
export function DecisionCard({ decision, askedBy, onDecide }: DecisionCardProps) {
  const [pick, setPick] = useState<OptionPick | undefined>(undefined);
  const [note, setNote] = useState('');
  const [isSending, setIsSending] = useState(false);
  const saysSomething = note.trim() !== '';
  // An option is an answer by itself. Words alone are one too. Nothing
  // picked and nothing said is not.
  const canDecide = !isSending && (typeof pick === 'number' || saysSomething);

  async function decide() {
    if (!canDecide) {
      return;
    }
    setIsSending(true);
    const wasTaken = await onDecide({ option: typeof pick === 'number' ? pick : null, note });
    if (!wasTaken) {
      setIsSending(false);
    }
  }

  function handleSubmit(event: FormEvent) {
    event.preventDefault();
    void decide();
  }

  return (
    <form className={styles.card} onSubmit={handleSubmit}>
      <p className={classNames('label', styles.asks)}>{askedBy} asks you to decide</p>
      <h4 className={styles.question} data-selectable>
        {decision.question}
      </h4>
      {decision.reason !== '' && (
        <p className={styles.reason} data-selectable>
          {decision.reason}
        </p>
      )}

      <OptionList
        label="Options"
        options={decision.options}
        recommended={decision.recommended}
        pick={pick}
        onPick={setPick}
      />

      <textarea
        className={styles.note}
        aria-label="Your words"
        placeholder={
          pick === 'own words' ? 'What you decide, in your own words…' : 'Add a note, if you like…'
        }
        value={note}
        onChange={(event) => {
          setNote(event.target.value);
        }}
        rows={2}
      />

      <div className={styles.actions}>
        <Button variant="primary" type="submit" disabled={!canDecide}>
          Decide
        </Button>
      </div>
    </form>
  );
}
