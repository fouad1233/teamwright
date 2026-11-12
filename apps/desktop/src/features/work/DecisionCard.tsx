import { useId, useState, type FormEvent } from 'react';

import type { DecisionRequest, NewDecisionAnswer } from '@/shared/api/types';
import { classNames } from '@/shared/lib/classNames';
import { Button } from '@/shared/ui/Button';

import styles from './DecisionCard.module.css';

/** What the manager has picked so far: one of the options, or to answer in words alone. */
type Pick = number | 'own words' | undefined;

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
 * one thing there that carries the brass.
 */
export function DecisionCard({ decision, askedBy, onDecide }: DecisionCardProps) {
  const name = useId();
  const [pick, setPick] = useState<Pick>(undefined);
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

      <div className={styles.options} role="radiogroup" aria-label="Options">
        {decision.options.map((option, place) => (
          <label
            // The options of a question never change, so their place identifies them.
            key={place}
            className={classNames(styles.option, pick === place && styles.picked)}
          >
            <input
              type="radio"
              name={name}
              className={styles.radio}
              checked={pick === place}
              onChange={() => {
                setPick(place);
              }}
            />
            <span className={styles.optionLabel}>
              {option.label}
              {decision.recommended === place && (
                <span className={classNames('label', styles.recommended)}>Recommended</span>
              )}
            </span>
            {option.consequence !== '' && (
              <span className={styles.consequence}>{option.consequence}</span>
            )}
          </label>
        ))}
        <label className={classNames(styles.option, pick === 'own words' && styles.picked)}>
          <input
            type="radio"
            name={name}
            className={styles.radio}
            checked={pick === 'own words'}
            onChange={() => {
              setPick('own words');
            }}
          />
          <span className={styles.optionLabel}>Something else</span>
          <span className={styles.consequence}>Say below what you want instead.</span>
        </label>
      </div>

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
