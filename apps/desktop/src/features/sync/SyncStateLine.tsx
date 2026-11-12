import { MonitorSmartphone } from 'lucide-react';

import { useNavigation } from '@/app/navigation';
import { classNames } from '@/shared/lib/classNames';

import styles from './SyncStateLine.module.css';
import { stateLineOf } from './syncModel';
import { useSyncState } from './syncQueries';
import { useNow } from './useNow';

/**
 * The way to Computers, at the foot of the navigation. Once a folder is
 * chosen it says in words how syncing stands. It lights only when
 * something is wrong or waits for the manager, and then the words say it
 * too.
 */
export function SyncStateLine() {
  const navigation = useNavigation();
  const state = useSyncState().data;
  const now = useNow();
  const line = state ? stateLineOf(state, now) : null;
  const isHere = navigation.place === 'computers';

  return (
    <button
      type="button"
      className={classNames(styles.line, isHere && styles.here, line?.isLit && styles.lit)}
      aria-current={isHere ? 'page' : undefined}
      aria-label={line ? `Computers: ${line.words}` : 'Computers'}
      onClick={navigation.openComputers}
    >
      {line?.isLit && (
        <span className={styles.joints} aria-hidden>
          <span />
        </span>
      )}
      <MonitorSmartphone />
      <span className={styles.text}>
        <span className={styles.name}>Computers</span>
        {line && <span className={styles.words}>{line.words}</span>}
      </span>
    </button>
  );
}
