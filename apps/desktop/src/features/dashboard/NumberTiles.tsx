import { classNames } from '@/shared/lib/classNames';

import type { DashboardCounts } from './dashboardModel';
import styles from './NumberTiles.module.css';

interface NumberTilesProps {
  counts: DashboardCounts;
  onOpenInbox: () => void;
}

/**
 * The plain numbers of the project, side by side in the cells of one
 * block. The last cell, what waits for the manager, leads to the inbox and
 * is held by the gold corners of a waiting card while anything waits. It
 * is the only light on the page.
 */
export function NumberTiles({ counts, onOpenInbox }: NumberTilesProps) {
  const tiles = [
    { label: 'Open', value: counts.open },
    { label: 'In work', value: counts.inWork },
    { label: 'Finished', value: counts.finished },
    { label: 'Late', value: counts.late, isDanger: counts.late > 0 },
    { label: 'Given to nobody', value: counts.givenToNobody },
  ];
  const isWaiting = counts.waiting > 0;

  return (
    <ul className={styles.tiles} aria-label="The project in numbers">
      {tiles.map(({ label, value, isDanger }) => (
        <li key={label} className={styles.tile}>
          <span className="label">{label}</span>
          <span className={classNames(styles.value, isDanger && styles.danger)}>{value}</span>
        </li>
      ))}
      <li className={classNames(styles.tile, styles.waiting)}>
        <button
          type="button"
          className={styles.press}
          aria-label={`${counts.waiting.toString()} waiting for you. Open the inbox`}
          onClick={onOpenInbox}
        >
          <span className={classNames('label', isWaiting && styles.waitingLabel)}>
            Waiting for you
          </span>
          <span className={styles.value}>{counts.waiting}</span>
        </button>
        {isWaiting && (
          <span className={styles.joints} aria-hidden>
            <span />
          </span>
        )}
      </li>
    </ul>
  );
}
