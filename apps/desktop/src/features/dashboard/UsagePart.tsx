import type { CSSProperties } from 'react';

import { usageInWords } from '../office/officeModel';
import { DashboardSection } from './DashboardSection';
import { shareOf, type ProjectUsage } from './dashboardModel';
import styles from './UsagePart.module.css';

interface UsagePartProps {
  usage: ProjectUsage;
  /** Whether the project has a team at all. */
  hasTeam: boolean;
}

/**
 * What the project used in tokens: in all, at the right of the heading,
 * and for the Advisor and every AI specialist, as bars on one scale with
 * the number written after each. A person uses none and is not listed.
 */
export function UsagePart({ usage, hasTeam }: UsagePartProps) {
  const largest = Math.max(1, ...usage.users.map((user) => user.tokens));
  return (
    <DashboardSection
      title="Usage"
      aside={<span className={styles.total}>The project: {usageInWords(usage.total)}</span>}
    >
      {usage.total === 0 ? (
        <p className={styles.quiet}>
          {hasTeam
            ? 'Nothing has used tokens yet. The count grows as the specialists and the Advisor work.'
            : 'Nothing has used tokens yet. Nobody is on the team, and the Advisor has not planned the project.'}
        </p>
      ) : (
        <ul className={styles.users}>
          {usage.users.map((user) => (
            <li key={user.key} className={styles.user}>
              <span className={styles.name}>{user.name}</span>
              <span
                className={styles.track}
                aria-hidden
                style={{ '--share': shareOf(user.tokens, largest) } as CSSProperties}
              >
                <span className={styles.bar} />
              </span>
              <span className={styles.tokens}>{usageInWords(user.tokens)}</span>
            </li>
          ))}
        </ul>
      )}
    </DashboardSection>
  );
}
