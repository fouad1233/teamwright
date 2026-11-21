import { Hourglass } from 'lucide-react';

import { useNow } from '../sync/useNow';
import styles from './PlanLimitLine.module.css';
import { planLimitLine } from './planLimitModel';
import { usePlanLimit } from './workQueries';

/**
 * At the foot of the navigation, under the way to the computers: the one
 * plain line that says the limit of the Claude plan is reached, and when
 * it resets. It is about this computer's Claude account, not a project, so
 * it stands where the things of this computer stand. It is not lit:
 * nothing happens and nothing waits for the manager, the work only waits
 * for the time (decision 0040). Nothing is shown while the limit is not
 * reached.
 */
export function PlanLimitLine() {
  const planLimit = usePlanLimit().data;
  const now = useNow();
  const line = planLimitLine(planLimit, now);
  if (line === null) {
    return null;
  }
  return (
    <p className={styles.line} role="status">
      <Hourglass aria-hidden />
      <span className={styles.words}>{line}</span>
    </p>
  );
}
