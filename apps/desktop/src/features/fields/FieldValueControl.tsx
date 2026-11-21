import { ExternalLink } from 'lucide-react';
import { useState, type KeyboardEvent } from 'react';

import { useBackend } from '@/shared/api/BackendProvider';
import type { CustomField, FieldValue } from '@/shared/api/types';
import { classNames } from '@/shared/lib/classNames';
import { IconButton, ValueButton } from '@/shared/ui/Button';
import { DateField } from '@/shared/ui/DateField';
import { ChoiceMenu, type Choice } from '@/shared/ui/Menu';
import { describeFailure, useNotifier } from '@/shared/ui/Toast';

import { TagChip } from '../tags/TagChip';
import { choiceOf, numberForEditing, parseNumber, shortAddress } from './fieldModel';
import styles from './FieldValueControl.module.css';

interface FieldValueControlProps {
  field: CustomField;
  value: FieldValue | undefined;
  /** What the control is about, for screen readers: "Unit cost of SSN-3". */
  label: string;
  /** Gives the task a value, or clears it with nothing. Resolves to whether it was taken. */
  onChange: (value: FieldValue | null) => Promise<boolean>;
  /** `panel` for the panel of a task, `cell` for a cell of the table. */
  look: 'panel' | 'cell';
}

/** The value of "nothing" in the menus of a choice and of yes or no. */
const NONE = 'none';

/**
 * The value of one custom field of a task, changed where it stands, with a
 * control that suits its kind: a line to type in for text, a number and a
 * link, the calendar of the program for a date, and a menu for yes or no
 * and for a choice. Every control can clear the value.
 */
export function FieldValueControl({ field, value, label, onChange, look }: FieldValueControlProps) {
  switch (field.kind) {
    case 'text':
      return (
        <TypedValue
          key={value?.kind === 'text' ? value.text : ''}
          stored={value?.kind === 'text' ? value.text : ''}
          label={label}
          look={look}
          maxLength={200}
          read={(typed) => ({ kind: 'text', text: typed })}
          onChange={onChange}
        />
      );
    case 'number': {
      const stored = value?.kind === 'number' ? numberForEditing(value.number) : '';
      return (
        <TypedValue
          key={stored}
          stored={stored}
          label={label}
          look={look}
          maxLength={24}
          isFigure
          unit={field.unit}
          inputMode="decimal"
          read={(typed) => {
            const number = parseNumber(typed);
            return number === null ? undefined : { kind: 'number', number };
          }}
          hint="Write a number, such as 12.5"
          onChange={onChange}
        />
      );
    }
    case 'link':
      return <LinkValue value={value} label={label} look={look} onChange={onChange} />;
    case 'date':
      return (
        <DateField
          label={label}
          value={value?.kind === 'date' ? value.date : null}
          onChange={(date) => {
            void onChange(date === null ? null : { kind: 'date', date });
          }}
        />
      );
    case 'yes_no': {
      const current = value?.kind === 'yes_no' ? (value.yes ? 'yes' : 'no') : NONE;
      const choices: Choice<string>[] = [
        { value: 'yes', label: 'Yes' },
        { value: 'no', label: 'No' },
        { value: NONE, label: 'Not set' },
      ];
      return (
        <ChoiceMenu
          label={label}
          choices={choices}
          value={current}
          onChoose={(chosen) => {
            void onChange(chosen === NONE ? null : { kind: 'yes_no', yes: chosen === 'yes' });
          }}
        >
          <ValueButton
            className={classNames(styles.button, styles[look])}
            isEmpty={current === NONE}
            aria-label={label}
            data-cell-target=""
          >
            {current === NONE
              ? look === 'cell'
                ? '–'
                : 'Not set'
              : current === 'yes'
                ? 'Yes'
                : 'No'}
          </ValueButton>
        </ChoiceMenu>
      );
    }
    case 'choice': {
      const chosen = choiceOf(field, value);
      const choices: Choice<string>[] = [
        ...field.choices.map((choice) => ({
          value: choice.id,
          label: choice.name,
          icon: <span className={styles.swatch} data-tone={choice.color} />,
        })),
        { value: NONE, label: 'None' },
      ];
      return (
        <ChoiceMenu
          label={label}
          choices={choices}
          value={chosen?.id ?? NONE}
          onChoose={(choiceId) => {
            void onChange(choiceId === NONE ? null : { kind: 'choice', choiceId });
          }}
        >
          <ValueButton
            className={classNames(styles.button, styles[look])}
            isEmpty={chosen === null}
            aria-label={label}
            data-cell-target=""
          >
            {chosen === null ? (
              look === 'cell' ? (
                '–'
              ) : field.choices.length === 0 ? (
                'No choices yet'
              ) : (
                'None'
              )
            ) : (
              <TagChip tag={chosen} size={look === 'panel' ? 'regular' : 'small'} />
            )}
          </ValueButton>
        </ChoiceMenu>
      );
    }
  }
}

interface TypedValueProps {
  /** What the line holds now, as it is edited. */
  stored: string;
  label: string;
  look: 'panel' | 'cell';
  maxLength: number;
  /** A number is set to the right in a cell, so that the units stand under each other. */
  isFigure?: boolean;
  /** Shown after the number, such as "€". */
  unit?: string | null;
  inputMode?: 'decimal';
  /** What the typed text stands for, or nothing when it is not understood. */
  read: (typed: string) => FieldValue | undefined;
  /** Said when the text is not understood. */
  hint?: string;
  onChange: (value: FieldValue | null) => Promise<boolean>;
}

/**
 * A line to type a value in. Enter or leaving it saves; Escape undoes. An
 * empty line clears the value.
 */
function TypedValue({
  stored,
  label,
  look,
  maxLength,
  isFigure = false,
  unit,
  inputMode,
  read,
  hint,
  onChange,
}: TypedValueProps) {
  const [draft, setDraft] = useState(stored);
  const isEmpty = draft.trim() === '';
  const meant = isEmpty ? null : read(draft);
  const isUnderstood = meant !== undefined;

  function save() {
    if (draft.trim() === stored.trim()) {
      return;
    }
    if (meant === undefined) {
      setDraft(stored);
      return;
    }
    void onChange(meant).then((taken) => {
      if (!taken) {
        setDraft(stored);
      }
    });
  }

  function handleKeyDown(event: KeyboardEvent<HTMLInputElement>) {
    event.stopPropagation();
    if (event.key === 'Enter') {
      event.currentTarget.blur();
    } else if (event.key === 'Escape') {
      setDraft(stored);
    }
  }

  return (
    <span className={classNames(styles.typed, styles[look], isFigure && styles.figure)}>
      <input
        className={classNames(styles.input, !isUnderstood && styles.notUnderstood)}
        value={draft}
        placeholder={look === 'cell' ? '–' : 'None'}
        maxLength={maxLength}
        inputMode={inputMode}
        aria-label={label}
        aria-invalid={!isUnderstood}
        title={isUnderstood ? undefined : hint}
        data-cell-target=""
        data-escape-undoes
        onChange={(event) => {
          setDraft(event.target.value);
        }}
        onBlur={save}
        onKeyDown={handleKeyDown}
      />
      {unit && !isEmpty && <span className={styles.unit}>{unit}</span>}
    </span>
  );
}

interface LinkValueProps {
  value: FieldValue | undefined;
  label: string;
  look: 'panel' | 'cell';
  onChange: (value: FieldValue | null) => Promise<boolean>;
}

/**
 * A web address: a line to type it in, and a button that opens it in the
 * browser of the computer, through the same checked path as every other
 * address the program opens.
 */
function LinkValue({ value, label, look, onChange }: LinkValueProps) {
  const backend = useBackend();
  const notifier = useNotifier();
  const address = value?.kind === 'link' ? value.address : '';
  return (
    <span className={styles.link}>
      <TypedValue
        key={address}
        stored={address}
        label={label}
        look={look}
        maxLength={2000}
        read={(typed) => ({ kind: 'link', address: typed.trim() })}
        onChange={onChange}
      />
      {address !== '' && (
        <IconButton
          label={`Open ${shortAddress(address)}`}
          className={styles.open}
          onClick={() => {
            backend.openWebAddress(address).catch((failure: unknown) => {
              notifier.showError(describeFailure(failure));
            });
          }}
        >
          <ExternalLink />
        </IconButton>
      )}
    </span>
  );
}
