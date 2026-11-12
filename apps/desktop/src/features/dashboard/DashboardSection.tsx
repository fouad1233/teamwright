import type { ReactNode } from 'react';

import styles from './DashboardSection.module.css';

interface DashboardSectionProps {
  title: string;
  /** A fact or a way onwards, at the right end of the heading. */
  aside?: ReactNode;
  children: ReactNode;
}

/**
 * One part of the dashboard: its name as a label on a strong rule, as the
 * parts of the backlog are headed, and what it shows below.
 */
export function DashboardSection({ title, aside, children }: DashboardSectionProps) {
  return (
    <section className={styles.section} aria-label={title}>
      <header className={styles.heading}>
        <h2 className="label">{title}</h2>
        {aside}
      </header>
      {children}
    </section>
  );
}
