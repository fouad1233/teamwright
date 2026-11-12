import type { CSSProperties } from 'react';

import type { Assignee } from '@/shared/api/types';

import { AssigneeAvatar } from '../team/Assignee';
import { largestHolding, shareOf, type Holder, type StatusShare } from './dashboardModel';
import styles from './WorkCharts.module.css';

interface StatusBarProps {
  /** Every status of the workflow, in its order, with its tasks counted. */
  shares: readonly StatusShare[];
  total: number;
}

/**
 * The work by status: one bar with a segment for every status, in the
 * order of the workflow and the pigment of the status, and under it the
 * name and the count of each, in the same order.
 */
export function StatusBar({ shares, total }: StatusBarProps) {
  const inWords = shares
    .map(({ status, count }) => `${count.toString()} ${status.name}`)
    .join(', ');
  return (
    <div className={styles.statusChart}>
      <div
        className={styles.statusBar}
        role="img"
        aria-label={`${total.toString()} tasks: ${inWords}`}
      >
        {shares
          .filter(({ count }) => count > 0)
          .map(({ status, count }) => (
            <span
              key={status.id}
              className={styles.segment}
              data-tone={status.color}
              style={{ flexGrow: count }}
            />
          ))}
      </div>
      <ul className={styles.statusCounts}>
        {shares.map(({ status, count }) => (
          <li key={status.id} className={styles.statusCount}>
            <span className={styles.swatch} data-tone={status.color} aria-hidden />
            <span className={styles.statusName}>{status.name}</span>
            <span className={styles.number}>{count}</span>
          </li>
        ))}
      </ul>
    </div>
  );
}

/** Who a holder is, as the avatar of a task shows it. */
function assigneeOf(holder: Holder): Assignee | null {
  if (holder.kind === 'manager') {
    return { kind: 'manager' };
  }
  return holder.member ? { kind: 'member', memberId: holder.member.id } : null;
}

/**
 * The work by member: for the manager, every member and nobody, a bar of
 * the tasks given to them, filled in ink as far as they are finished, with
 * the numbers written after it. The bars share one scale.
 */
export function HolderBars({ holders }: { holders: readonly Holder[] }) {
  const largest = largestHolding(holders);
  return (
    <ul className={styles.holders}>
      {holders.map((holder) => {
        const given = holder.open + holder.finished;
        return (
          <li key={holder.key} className={styles.holder}>
            <AssigneeAvatar assignee={assigneeOf(holder)} />
            <span className={styles.holderName}>{holder.name}</span>
            <span
              className={styles.track}
              aria-hidden
              style={
                {
                  '--given': shareOf(given, largest),
                  '--finished': shareOf(holder.finished, largest),
                } as CSSProperties
              }
            >
              <span className={styles.given} />
              <span className={styles.finished} />
            </span>
            <span className={styles.figures}>
              {given === 0 ? (
                'Nothing given'
              ) : (
                <>
                  {holder.open} open
                  <span className={styles.separator} aria-hidden />
                  {holder.finished} finished
                </>
              )}
            </span>
          </li>
        );
      })}
    </ul>
  );
}
