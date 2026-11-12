import { X } from 'lucide-react';
import { Dialog as DialogPrimitive } from 'radix-ui';
import type { ReactNode } from 'react';

import { classNames } from '@/shared/lib/classNames';

import { IconButton } from './Button';
import styles from './Dialog.module.css';

interface DialogProps {
  open: boolean;
  title: string;
  /** `wide` for a dialog that edits a list, `regular` for a form or a question. */
  size?: 'regular' | 'wide';
  /** Called when the user dismisses the dialog: Escape, the close button or a click outside. */
  onClose: () => void;
  children: ReactNode;
}

/**
 * A modal dialog. While it is open the keyboard focus stays inside it, and
 * Escape or a click outside closes it.
 *
 * When the dialog opens, the focus goes to the element inside it that
 * carries the attribute `data-initial-focus`: the first field of a form, or
 * the safe choice of a confirmation.
 *
 * A field that is edited in place and takes Escape to undo the edit carries
 * the attribute `data-escape-undoes`. While the focus is in such a field,
 * Escape belongs to the field and the dialog stays open.
 */
export function Dialog({ open, title, size = 'regular', onClose, children }: DialogProps) {
  return (
    <DialogPrimitive.Root
      open={open}
      onOpenChange={(isOpen) => {
        if (!isOpen) {
          onClose();
        }
      }}
    >
      <DialogPrimitive.Portal>
        <DialogPrimitive.Overlay className={styles.overlay} />
        <DialogPrimitive.Content
          className={classNames(styles.dialog, size === 'wide' && styles.wide)}
          // A dialog here is a short form or a question. Its title says all
          // that a description would.
          aria-describedby={undefined}
          onOpenAutoFocus={focusInitialElement}
          onEscapeKeyDown={leaveEscapeToTheField}
        >
          <header className={styles.header}>
            <DialogPrimitive.Title className={styles.title}>{title}</DialogPrimitive.Title>
            <DialogPrimitive.Close asChild>
              <IconButton label="Close">
                <X />
              </IconButton>
            </DialogPrimitive.Close>
          </header>
          {children}
        </DialogPrimitive.Content>
      </DialogPrimitive.Portal>
    </DialogPrimitive.Root>
  );
}

/** Keeps the dialog open when Escape is pressed in a field that undoes its edit with it. */
function leaveEscapeToTheField(event: KeyboardEvent) {
  const focused = document.activeElement;
  if (focused instanceof HTMLElement && focused.dataset.escapeUndoes !== undefined) {
    event.preventDefault();
  }
}

/** Moves the focus to the element the dialog marked as its starting point. */
function focusInitialElement(event: Event) {
  const dialog = event.currentTarget;
  if (!(dialog instanceof HTMLElement)) {
    return;
  }
  const initial = dialog.querySelector<HTMLElement>('[data-initial-focus]');
  if (initial) {
    event.preventDefault();
    initial.focus();
  }
}
