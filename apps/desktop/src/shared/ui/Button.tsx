import type { ComponentProps, ReactNode } from 'react';

import { classNames } from '@/shared/lib/classNames';

import styles from './Button.module.css';
import { followPointer } from './lamplight';
import { Tooltip } from './Tooltip';

/** How much a button stands out. */
type ButtonVariant = 'primary' | 'secondary' | 'ghost' | 'danger';

type ButtonProps = ComponentProps<'button'> & {
  /** `primary` for the one main action of a view, `secondary` otherwise. */
  variant?: ButtonVariant;
  /** An icon shown before the label. */
  icon?: ReactNode;
};

/** A button with a text label. The primary one is lit by a lamp under the pointer. */
export function Button({
  variant = 'secondary',
  icon,
  className,
  children,
  type = 'button',
  onPointerEnter,
  onPointerMove,
  ...rest
}: ButtonProps) {
  const isLit = variant === 'primary';
  return (
    <button
      type={type}
      className={classNames(styles.button, styles[variant], className)}
      onPointerEnter={(event) => {
        if (isLit) {
          followPointer(event);
        }
        onPointerEnter?.(event);
      }}
      onPointerMove={(event) => {
        if (isLit) {
          followPointer(event);
        }
        onPointerMove?.(event);
      }}
      {...rest}
    >
      {icon}
      {children}
    </button>
  );
}

type IconButtonProps = ComponentProps<'button'> & {
  /** What the button does. Read by screen readers and shown as a tooltip. */
  label: string;
};

/** A button that shows only an icon, with a tooltip that says what it does. */
export function IconButton({
  label,
  className,
  children,
  type = 'button',
  ...rest
}: IconButtonProps) {
  return (
    <Tooltip label={label}>
      <button
        type={type}
        className={classNames(styles.iconButton, className)}
        aria-label={label}
        {...rest}
      >
        {children}
      </button>
    </Tooltip>
  );
}

type ValueButtonProps = ComponentProps<'button'> & {
  /** Shows the content in a fainter ink, for a value that is not set. */
  isEmpty?: boolean;
};

/**
 * A value that can be changed by clicking it: the status, the assignee or a
 * date of a task. It looks like plain text until the pointer is over it, and
 * opens a menu or a calendar.
 */
export function ValueButton({
  isEmpty = false,
  className,
  children,
  type = 'button',
  ...rest
}: ValueButtonProps) {
  return (
    <button
      type={type}
      className={classNames(styles.valueButton, isEmpty && styles.emptyValue, className)}
      {...rest}
    >
      {children}
    </button>
  );
}
