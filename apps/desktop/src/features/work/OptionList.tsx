import { useId } from 'react';

import type { DecisionOption } from '@/shared/api/types';
import { classNames } from '@/shared/lib/classNames';

import styles from './OptionList.module.css';
import { appearsAt, waitsForManager } from './waitsForManager';

/** What the manager has picked: one of the options, or to answer in words alone. */
export type OptionPick = number | 'own words';

interface OptionListProps {
  /** What the options are for, for screen readers. */
  label: string;
  options: readonly DecisionOption[];
  /** The option that is recommended, by its place from 0, if one is. */
  recommended: number | null;
  /** What is picked, or nothing yet. */
  pick: OptionPick | undefined;
  onPick: (pick: OptionPick) => void;
  /** Lets the options come up one after the other, as part of a decision that appears. */
  isAppearing?: boolean;
}

/**
 * The answers to a question that the manager can pick from: each option as
 * a small card with what it leads to, and one more for an answer in the
 * manager's own words.
 */
export function OptionList({
  label,
  options,
  recommended,
  pick,
  onPick,
  isAppearing = false,
}: OptionListProps) {
  const name = useId();

  return (
    <div className={styles.options} role="radiogroup" aria-label={label}>
      {options.map((option, place) => (
        <label
          // The options of a question never change, so their place identifies them.
          key={place}
          className={classNames(
            styles.option,
            pick === place && styles.picked,
            isAppearing && waitsForManager.part,
          )}
          // The options come up after the question, one after the other.
          style={appearsAt(place + 1)}
        >
          <input
            type="radio"
            name={name}
            className={styles.radio}
            checked={pick === place}
            onChange={() => {
              onPick(place);
            }}
          />
          <span className={styles.optionLabel}>
            {option.label}
            {recommended === place && (
              <span className={classNames('label', styles.recommended)}>Recommended</span>
            )}
          </span>
          {option.consequence !== '' && (
            <span className={styles.consequence}>{option.consequence}</span>
          )}
        </label>
      ))}
      <label
        className={classNames(
          styles.option,
          pick === 'own words' && styles.picked,
          isAppearing && waitsForManager.part,
        )}
        style={appearsAt(options.length + 1)}
      >
        <input
          type="radio"
          name={name}
          className={styles.radio}
          checked={pick === 'own words'}
          onChange={() => {
            onPick('own words');
          }}
        />
        <span className={styles.optionLabel}>Something else</span>
        <span className={styles.consequence}>Say below what you want instead.</span>
      </label>
    </div>
  );
}
