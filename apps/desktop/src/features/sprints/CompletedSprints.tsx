import type { CSSProperties } from 'react';

import type { Sprint } from '@/shared/api/types';

import styles from './CompletedSprints.module.css';
import { formatSprintDates, formatSummary, paceBarsOf } from './sprintModel';

/**
 * The sprints that are over, the latest first: what each finished of what
 * it took on, in words and as a bar. The bars share one scale, so that the
 * pace from sprint to sprint can be read down the column.
 */
export function CompletedSprints({ sprints }: { sprints: readonly Sprint[] }) {
  const bars = paceBarsOf(sprints);
  if (bars.length === 0) {
    return null;
  }

  return (
    <section className={styles.completed} aria-label="Completed sprints">
      <h2 className={styles.title}>
        <span className="label">Completed sprints</span>
      </h2>
      <ol className={styles.list}>
        {bars.map(({ sprint, summary, committedShare, completedShare }) => (
          <li key={sprint.id} className={styles.sprint}>
            <span className={styles.name}>{sprint.name}</span>
            <span className={styles.dates}>{formatSprintDates(sprint)}</span>
            <span className={styles.summary}>{formatSummary(summary)}</span>
            <span
              className={styles.bar}
              role="img"
              aria-label={formatSummary(summary)}
              style={
                {
                  '--committed': committedShare,
                  '--completed': completedShare,
                } as CSSProperties
              }
            >
              <span className={styles.committed} />
              <span className={styles.done} />
            </span>
          </li>
        ))}
      </ol>
    </section>
  );
}
