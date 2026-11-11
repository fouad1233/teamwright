import type { ComponentProps, ReactNode } from 'react';

import { classNames } from '@/shared/lib/classNames';

import styles from './Field.module.css';

interface FieldProps {
  label: string;
  /** A short explanation shown under the control. */
  hint?: string;
  children: ReactNode;
}

/** A labeled form control. Clicking the label focuses the control. */
export function Field({ label, hint, children }: FieldProps) {
  return (
    <label className={styles.field}>
      <span className="label">{label}</span>
      {children}
      {hint !== undefined && <span className={styles.hint}>{hint}</span>}
    </label>
  );
}

/** A single-line text input. */
export function TextInput({ className, ...rest }: ComponentProps<'input'>) {
  return <input className={classNames(styles.control, className)} {...rest} />;
}

/** A multi-line text input. */
export function TextArea({ className, ...rest }: ComponentProps<'textarea'>) {
  return <textarea className={classNames(styles.control, styles.textArea, className)} {...rest} />;
}
