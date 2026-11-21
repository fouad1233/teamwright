import { useId } from 'react';

import { classNames } from '@/shared/lib/classNames';
import { TextArea } from '@/shared/ui/Field';

import styles from './InstructionsField.module.css';
import { countInWords, INSTRUCTIONS_LIMIT, lengthOfInstructions } from './teamModel';

interface InstructionsFieldProps {
  value: string;
  onChange: (instructions: string) => void;
}

/**
 * The instructions of a specialist: a text area that grows with what is
 * written, and under it what they are for and how many characters are
 * left. Past the limit the count turns red; the backend refuses the text.
 */
export function InstructionsField({ value, onChange }: InstructionsFieldProps) {
  const length = lengthOfInstructions(value);
  const isTooLong = length > INSTRUCTIONS_LIMIT;
  const countId = useId();

  return (
    <label className={styles.field}>
      <span className="label">Instructions</span>
      <TextArea
        className={styles.text}
        value={value}
        onChange={(event) => {
          onChange(event.target.value);
        }}
        placeholder={
          'How this specialist works, what it cares about, what it must never do.\n' +
          'For example: Cite the data sheet for every value. Never guess a pin.'
        }
        aria-describedby={countId}
        aria-invalid={isTooLong}
        spellCheck
      />
      <span className={styles.foot}>
        <span className={styles.meaning}>
          Read at every turn, after Teamwright&apos;s own rules, which they cannot change.
        </span>
        <span
          id={countId}
          className={classNames(styles.count, isTooLong && styles.tooLong)}
          aria-live="polite"
        >
          {countInWords(length)} / {countInWords(INSTRUCTIONS_LIMIT)}
        </span>
      </span>
    </label>
  );
}
