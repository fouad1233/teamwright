import type { ComponentProps, ReactNode } from 'react';

import { classNames } from '@/shared/lib/classNames';

import styles from './Form.module.css';

/** A form in a dialog: its fields under each other, with even gaps. */
export function Form({ className, ...rest }: ComponentProps<'form'>) {
  return <form className={classNames(styles.form, className)} {...rest} />;
}

/** A sentence that explains something about the fields above it. */
export function FormNote({ children }: { children: ReactNode }) {
  return <p className={styles.note}>{children}</p>;
}

/**
 * Why the backend refused what the form asked for. It is announced to
 * screen readers when it appears.
 */
export function Refusal({ children }: { children: ReactNode }) {
  return (
    <p className={styles.refusal} role="alert">
      {children}
    </p>
  );
}

/** The buttons at the end of a form or a question, set to the right. */
export function FormActions({ children }: { children: ReactNode }) {
  return <div className={styles.actions}>{children}</div>;
}
