import type { CSSProperties } from 'react';

import { useIsDemo } from '@/shared/api/BackendProvider';
import { classNames } from '@/shared/lib/classNames';

import { useNow } from '../sync/useNow';
import {
  planLimitLine,
  resetInWords,
  usedInWords,
  usedShare,
  windowName,
  windowsToShow,
} from '../work/planLimitModel';
import { usePlanLimit } from '../work/workQueries';
import styles from './PlanLimitPart.module.css';

/**
 * What is known about the limits of the Claude plan this computer runs the
 * specialists on (decision 0040): whether the limit is reached and when it
 * resets, and of the five hours and the week how much is used and when
 * each starts again, as Claude Code last reported them. What it did not
 * report is "unknown", never guessed. It is the same for every project:
 * the specialists of all of them share one plan.
 */
export function PlanLimitPart() {
  const planLimit = usePlanLimit().data;
  const now = useNow();
  const isDemo = useIsDemo();
  const line = planLimitLine(planLimit, now);
  const windows = windowsToShow(planLimit);

  return (
    <div className={styles.plan}>
      <h4 className={classNames('label', styles.title)}>
        {isDemo ? 'The Claude plan, pretended' : 'Your Claude plan, on this computer'}
      </h4>
      <p className={classNames(styles.standing, line !== null && styles.reached)}>
        {line ?? (planLimit ? 'The limit is not reached.' : 'Asking what is known…')}
      </p>
      <dl className={styles.windows}>
        {windows.map((window) => {
          const share = usedShare(window);
          return (
            <div key={window.kind} className={styles.window}>
              <dt className={styles.name}>{windowName(window.kind)}</dt>
              <dd className={styles.track} aria-hidden>
                {share !== null && (
                  <span
                    className={classNames(styles.bar, share >= 1 && styles.full)}
                    style={{ '--share': share } as CSSProperties}
                  />
                )}
              </dd>
              <dd className={classNames(styles.used, window.used === null && styles.unknown)}>
                {window.used === null ? 'unknown' : `${usedInWords(window)} used`}
              </dd>
              <dd className={styles.reset}>
                {window.resetsAt === null
                  ? 'reset unknown'
                  : `resets ${resetInWords(window.resetsAt, now)}`}
              </dd>
            </div>
          );
        })}
      </dl>
    </div>
  );
}
