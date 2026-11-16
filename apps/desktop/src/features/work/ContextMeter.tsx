import type { CSSProperties } from 'react';

import type { ContextUse } from '@/shared/api/types';
import { classNames } from '@/shared/lib/classNames';

import styles from './ContextMeter.module.css';
import { contextAmount, contextInFull, contextShare } from './workModel';

interface ContextMeterProps {
  context: ContextUse;
  className?: string;
}

/**
 * How full the context of a session is: the word "Context" and the amount
 * on one line, and a hairline meter under it. A context that is nearly full
 * takes the warning colour, and its title says what comes of that. Without
 * a known window there is no meter, only the amount.
 */
export function ContextMeter({ context, className }: ContextMeterProps) {
  const share = contextShare(context);
  return (
    <span
      className={classNames(styles.context, context.isNearlyFull && styles.nearlyFull, className)}
      title={contextInFull(context)}
    >
      <span className={styles.line}>
        <span className="label">Context</span>
        <span className={styles.amount}>{contextAmount(context)}</span>
      </span>
      {share !== null && (
        <span
          className={styles.track}
          role="meter"
          aria-label="Context"
          aria-valuemin={0}
          aria-valuemax={context.window ?? 0}
          aria-valuenow={context.held}
          aria-valuetext={contextAmount(context)}
          style={{ '--share': share } as CSSProperties}
        >
          <span className={styles.fill} />
        </span>
      )}
    </span>
  );
}
