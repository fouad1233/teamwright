import { useState, type FormEvent } from 'react';

import type { NewPermissionAnswer, PermissionRequest } from '@/shared/api/types';
import { classNames } from '@/shared/lib/classNames';
import { Button } from '@/shared/ui/Button';
import { TextInput } from '@/shared/ui/Field';

import { OptionList } from '../work/OptionList';
import { appearsAt, waitsForManager } from '../work/waitsForManager';
import styles from './PermissionCard.module.css';
import { WHAT_A_COMMAND_MEANS, choicesFor, kindInWords, runsCommands } from './permissionModel';

interface PermissionCardProps {
  request: PermissionRequest;
  /** The name of the specialist that asks. */
  askedBy: string;
  /** Sends the answer. Resolves to whether it was taken. */
  onAnswer: (answer: NewPermissionAnswer) => Promise<boolean>;
}

/**
 * A request of a specialist to do something it may not do on its own
 * (decision 0038): exactly what it wants to run or fetch, and why. The
 * manager allows it once or for the task (a web domain also always), or
 * refuses it. For a command, the card says what allowing it really means.
 *
 * What is allowed is what the card shows, and nothing more: the command or
 * the domain can be edited before it is allowed, and the application checks
 * it again. Like a question, it is the manager's to do, so it carries the
 * gold.
 */
export function PermissionCard({ request, askedBy, onAnswer }: PermissionCardProps) {
  const choices = choicesFor(request, askedBy);
  const [pick, setPick] = useState<number | undefined>(undefined);
  const [what, setWhat] = useState(request.what);
  const [note, setNote] = useState('');
  const [isSending, setIsSending] = useState(false);
  const choice = pick === undefined ? undefined : choices[pick]?.choice;
  const allows = choice !== undefined && choice !== 'refuse';
  const canAnswer = !isSending && choice !== undefined && (!allows || what.trim() !== '');
  const isCommand = runsCommands(request.kind);
  const afterTheChoices = appearsAt(choices.length + 3);

  async function send() {
    if (!canAnswer) {
      return;
    }
    setIsSending(true);
    const answer: NewPermissionAnswer =
      choice === 'refuse'
        ? { decision: 'refuse', note }
        : { decision: 'allow', scope: choice, kind: request.kind, what, note };
    if (!(await onAnswer(answer))) {
      setIsSending(false);
    }
  }

  function handleSubmit(event: FormEvent) {
    event.preventDefault();
    void send();
  }

  return (
    <form className={classNames(waitsForManager.frame, styles.card)} onSubmit={handleSubmit}>
      <p className={classNames('label', waitsForManager.part, styles.asks)}>
        {askedBy} asks for permission
      </p>
      <div className={classNames(waitsForManager.part, styles.wanted)} style={appearsAt(1)}>
        <span className={classNames('label', styles.kind)}>{kindInWords(request.kind)}</span>
        <code className={styles.what} data-selectable>
          {request.what}
        </code>
      </div>
      <p
        className={classNames(waitsForManager.part, styles.reason)}
        style={appearsAt(2)}
        data-selectable
      >
        {request.reason}
      </p>
      {isCommand && (
        <div
          className={classNames(waitsForManager.part, styles.warning)}
          style={appearsAt(2)}
          role="note"
        >
          {WHAT_A_COMMAND_MEANS.map((sentence) => (
            <p key={sentence}>{sentence}</p>
          ))}
        </div>
      )}

      <OptionList
        label="Answers"
        options={choices.map(({ option }) => option)}
        recommended={null}
        pick={pick}
        onPick={(picked) => {
          if (typeof picked === 'number') {
            setPick(picked);
          }
        }}
        isAppearing
        hasOwnWords={false}
      />

      {allows && (
        <div className={styles.rule}>
          <label className={styles.ruleField}>
            <span className="label">What you allow</span>
            <TextInput
              className={styles.ruleInput}
              value={what}
              spellCheck={false}
              onChange={(event) => {
                setWhat(event.target.value);
              }}
            />
          </label>
        </div>
      )}

      <textarea
        className={classNames(waitsForManager.part, styles.note)}
        style={afterTheChoices}
        aria-label="Your words"
        placeholder={
          choice === 'refuse' ? 'Why, if you like…' : 'Add a word for the specialist, if you like…'
        }
        value={note}
        onChange={(event) => {
          setNote(event.target.value);
        }}
        rows={2}
      />
      <div className={classNames(waitsForManager.part, styles.actions)} style={afterTheChoices}>
        <Button variant="primary" type="submit" disabled={!canAnswer}>
          Answer
        </Button>
      </div>
    </form>
  );
}
