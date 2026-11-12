import { X } from 'lucide-react';
import { useEffect, useRef, type KeyboardEvent, type ReactNode } from 'react';

import { IconButton } from './Button';
import styles from './SidePanel.module.css';

interface SidePanelProps {
  /** What the panel shows, for screen readers. */
  label: string;
  /** What stands at the top of the panel, in front of the button that closes it. */
  header: ReactNode;
  onClose: () => void;
  children: ReactNode;
}

/**
 * A panel at the side of a view, over its right edge: for one thing that is
 * looked at more closely while the view stays where it is. Escape closes
 * it.
 */
export function SidePanel({ label, header, onClose, children }: SidePanelProps) {
  const panel = useRef<HTMLElement>(null);

  // Moving the focus into the panel lets Escape close it right away.
  useEffect(() => {
    panel.current?.focus();
  }, []);

  function closeOnEscape(event: KeyboardEvent) {
    if (event.key === 'Escape') {
      onClose();
    }
  }

  return (
    <aside
      ref={panel}
      className={styles.panel}
      aria-label={label}
      tabIndex={-1}
      onKeyDown={closeOnEscape}
    >
      <header className={styles.header}>
        {header}
        <span className={styles.space} />
        <IconButton label="Close" onClick={onClose}>
          <X />
        </IconButton>
      </header>
      <div className={styles.body}>{children}</div>
    </aside>
  );
}
