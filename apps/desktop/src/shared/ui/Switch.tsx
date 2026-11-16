import type { ReactNode } from 'react';

import { classNames } from '@/shared/lib/classNames';

import styles from './Switch.module.css';

interface SwitchProps {
  /** Whether it is on. */
  on: boolean;
  /** What it turns on and off, in a few words. */
  label: ReactNode;
  /** A line under the label that says what it does. */
  hint?: ReactNode;
  disabled?: boolean;
  onChange: (on: boolean) => void;
  className?: string;
}

/**
 * Something the manager turns on or off: the words on the left, a small
 * track with a knob on the right. The whole line can be pressed.
 */
export function Switch({ on, label, hint, disabled, onChange, className }: SwitchProps) {
  return (
    <button
      type="button"
      role="switch"
      aria-checked={on}
      disabled={disabled}
      className={classNames(styles.switch, className)}
      onClick={() => {
        onChange(!on);
      }}
    >
      <span className={styles.words}>
        <span className={styles.label}>{label}</span>
        {hint !== undefined && <span className={styles.hint}>{hint}</span>}
      </span>
      <span className={classNames(styles.track, on && styles.on)} aria-hidden>
        <span className={styles.knob} />
      </span>
    </button>
  );
}
