import { useState, type CSSProperties, type FormEvent } from 'react';

import type { DecisionRequest, NewDecisionAnswer } from '@/shared/api/types';
import { classNames } from '@/shared/lib/classNames';
import { Button } from '@/shared/ui/Button';

import styles from './DecisionCard.module.css';
import { OptionList, type OptionPick } from './OptionList';
import { appearsAt, waitsForManager } from './waitsForManager';

interface DecisionCardProps {
  decision: DecisionRequest;
  /** The name of the specialist that asks. */
  askedBy: string;
  /** Sends the answer. Resolves to whether it was taken. */
  onDecide: (answer: NewDecisionAnswer) => Promise<boolean>;
  /**
   * The name of the computer the question is answered on, when it is not
   * this one. The question and its options can then be read here, and
   * nothing can be picked or sent.
   */
  answeredOn?: string;
}

/**
 * A question that is the manager's to answer: what is asked and why, the
 * options with what each leads to, and room for words of the manager's own.
 *
 * It is the one thing on its page that waits for the manager, so it is the
 * one thing there that carries the gold, and the gilded sun of decision
 * 0015.
 */
export function DecisionCard({ decision, askedBy, onDecide, answeredOn }: DecisionCardProps) {
  const [pick, setPick] = useState<OptionPick | undefined>(undefined);
  const [note, setNote] = useState('');
  const [isSending, setIsSending] = useState(false);
  const saysSomething = note.trim() !== '';
  // An option is an answer by itself. Words alone are one too. Nothing
  // picked and nothing said is not.
  const canDecide = !isSending && (typeof pick === 'number' || saysSomething);
  // The words of the manager and the button come up with the last option,
  // the one for an answer in words.
  const afterTheOptions = appearsAt(decision.options.length + 1);

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
    <form className={classNames(waitsForManager.frame, styles.card)} onSubmit={handleSubmit}>
      <p className={classNames('label', waitsForManager.part, styles.asks)}>
        {askedBy} asks you to decide
      </p>
      <h4 className={classNames(waitsForManager.part, styles.question)} data-selectable>
        {decision.question}
      </h4>
      {decision.reason !== '' && (
        <p className={classNames(waitsForManager.part, styles.reason)} data-selectable>
          {decision.reason}
        </p>
      )}

      <OptionList
        label="Options"
        options={decision.options}
        recommended={decision.recommended}
        pick={pick}
        onPick={setPick}
        isAppearing
        isReadOnly={answeredOn !== undefined}
      />

      {answeredOn !== undefined ? (
        <p
          className={classNames(waitsForManager.part, styles.elsewhere)}
          style={afterTheOptions}
          role="note"
        >
          <span className={styles.elsewhereWhere}>Asked on {answeredOn}.</span> Answer it there: the
          conversation runs in Teamwright on {answeredOn}.
        </p>
      ) : (
        <AnswerHere
          pick={pick}
          note={note}
          onNote={setNote}
          canDecide={canDecide}
          style={afterTheOptions}
        />
      )}
    </form>
  );
}

interface AnswerHereProps {
  pick: OptionPick | undefined;
  note: string;
  onNote: (note: string) => void;
  canDecide: boolean;
  /** When the words and the button come up. */
  style: CSSProperties;
}

/** The words of the manager and the button that sends the answer. */
function AnswerHere({ pick, note, onNote, canDecide, style }: AnswerHereProps) {
  return (
    <>
      <textarea
        className={classNames(waitsForManager.part, styles.note)}
        style={style}
        aria-label="Your words"
        placeholder={
          pick === 'own words' ? 'What you decide, in your own words…' : 'Add a note, if you like…'
        }
        value={note}
        onChange={(event) => {
          onNote(event.target.value);
        }}
        rows={2}
      />

      <div className={classNames(waitsForManager.part, styles.actions)} style={style}>
        <Button variant="primary" type="submit" disabled={!canDecide}>
          Decide
        </Button>
      </div>
    </>
  );
}
