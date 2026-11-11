import { X } from 'lucide-react';
import { Dialog as DialogPrimitive } from 'radix-ui';
import type { ReactNode } from 'react';

import { IconButton } from './Button';
import styles from './Dialog.module.css';

interface DialogProps {
  open: boolean;
  title: string;
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
 */
export function Dialog({ open, title, onClose, children }: DialogProps) {
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
          className={styles.dialog}
          // A dialog here is a short form or a question. Its title says all
          // that a description would.
          aria-describedby={undefined}
          onOpenAutoFocus={focusInitialElement}
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
