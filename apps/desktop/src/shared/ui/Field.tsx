import type {
  InputHTMLAttributes,
  ReactNode,
  SelectHTMLAttributes,
  TextareaHTMLAttributes,
} from 'react';

import { classNames } from '@/shared/lib/classNames';

import styles from './Field.module.css';

interface FieldProps {
  label: string;
  /** A short explanation shown under the control. */
  hint?: string;
  children: ReactNode;
}

/** A labeled form control. The label and the control are linked, so clicking the label focuses it. */
export function Field({ label, hint, children }: FieldProps) {
  return (
    <label className={styles.field}>
      <span className={styles.label}>{label}</span>
      {children}
      {hint && <span className={styles.hint}>{hint}</span>}
    </label>
  );
}

/** A single-line text input. */
export function TextInput({ className, ...rest }: InputHTMLAttributes<HTMLInputElement>) {
  return <input className={classNames(styles.control, className)} {...rest} />;
}

/** A multi-line text input. */
export function TextArea({ className, ...rest }: TextareaHTMLAttributes<HTMLTextAreaElement>) {
  return <textarea className={classNames(styles.control, styles.textArea, className)} {...rest} />;
}

/** A drop-down list. */
export function SelectInput({
  className,
  children,
  ...rest
}: SelectHTMLAttributes<HTMLSelectElement>) {
  return (
    <select className={classNames(styles.control, styles.select, className)} {...rest}>
      {children}
    </select>
  );
}
