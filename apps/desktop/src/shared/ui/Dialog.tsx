import { X } from 'lucide-react';
import { useEffect, useRef, type MouseEvent, type ReactNode } from 'react';

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
 * A modal dialog. It is built on the browser's own `<dialog>` element, which
 * traps the keyboard focus and closes on Escape without any code here.
 *
 * When the dialog opens, the keyboard focus goes to the element inside it
 * that carries the attribute `data-initial-focus`: the first field of a
 * form, or the safe choice of a confirmation.
 */
export function Dialog({ open, title, onClose, children }: DialogProps) {
  const dialogElement = useRef<HTMLDialogElement>(null);

  useEffect(() => {
    const dialog = dialogElement.current;
    if (!dialog) {
      return;
    }
    if (open && !dialog.open) {
      dialog.showModal();
      // Left to itself, the browser focuses the first control, which is the
      // close button in the header.
      dialog.querySelector<HTMLElement>('[data-initial-focus]')?.focus();
    } else if (!open && dialog.open) {
      dialog.close();
    }
  }, [open]);

  function closeIfBackdropWasClicked(event: MouseEvent<HTMLDialogElement>) {
    // The dialog element itself only receives clicks on its backdrop; clicks
    // inside land on the panel or its children.
    if (event.target === dialogElement.current) {
      onClose();
    }
  }

  return (
    <dialog
      ref={dialogElement}
      className={styles.dialog}
      onClose={onClose}
      onClick={closeIfBackdropWasClicked}
    >
      {open && (
        <div className={styles.panel}>
          <header className={styles.header}>
            <h2 className={styles.title}>{title}</h2>
            <IconButton label="Close" onClick={onClose}>
              <X />
            </IconButton>
          </header>
          {children}
        </div>
      )}
    </dialog>
  );
}
