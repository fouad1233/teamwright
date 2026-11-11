import type { ButtonHTMLAttributes, ReactNode } from 'react';

import { classNames } from '@/shared/lib/classNames';

import styles from './Button.module.css';

/** How much a button stands out. */
type ButtonVariant = 'primary' | 'secondary' | 'ghost' | 'danger';

interface ButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  /** `primary` for the main action of a view, `secondary` otherwise. */
  variant?: ButtonVariant;
  /** An icon shown before the label. */
  icon?: ReactNode;
}

/** A button with a text label. */
export function Button({
  variant = 'secondary',
  icon,
  className,
  children,
  type = 'button',
  ...rest
}: ButtonProps) {
  return (
    <button type={type} className={classNames(styles.button, styles[variant], className)} {...rest}>
      {icon}
      {children}
    </button>
  );
}

interface IconButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  /** What the button does, for screen readers and as a tooltip. */
  label: string;
}

/** A button that shows only an icon. */
export function IconButton({
  label,
  className,
  children,
  type = 'button',
  ...rest
}: IconButtonProps) {
  return (
    <button
      type={type}
      className={classNames(styles.iconButton, className)}
      aria-label={label}
      title={label}
      {...rest}
    >
      {children}
    </button>
  );
}
