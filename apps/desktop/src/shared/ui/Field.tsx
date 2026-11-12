import { ChevronDown } from 'lucide-react';
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

type ChoiceInputProps = ComponentProps<'button'> & {
  /** Shows the content in a fainter ink, for a choice that has not been made. */
  isEmpty?: boolean;
};

/**
 * A field of a form whose value is chosen from a menu. It looks like a text
 * input, and a mark at its end says that there is more to choose from. Use
 * it as the control that opens a `ChoiceMenu`.
 */
export function ChoiceInput({
  isEmpty = false,
  className,
  children,
  type = 'button',
  ...rest
}: ChoiceInputProps) {
  return (
    <button
      type={type}
      className={classNames(styles.control, styles.choice, isEmpty && styles.noChoice, className)}
      {...rest}
    >
      <span className={styles.chosen}>{children}</span>
      <ChevronDown aria-hidden />
    </button>
  );
}
