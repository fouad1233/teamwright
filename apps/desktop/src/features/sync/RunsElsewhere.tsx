import { MonitorSmartphone } from 'lucide-react';

import type { Conversation } from '@/shared/api/types';

import styles from './RunsElsewhere.module.css';
import { homeComputerName, lastNewsOf, sinceInWords } from './syncModel';
import { useSyncState } from './syncQueries';
import { useNow } from './useNow';

interface RunsElsewhereProps {
  conversation: Conversation;
  /**
   * What waits there, in a sentence of its own, such as "Its question is
   * answered there." Left out, the line says that the conversation can be
   * continued there.
   */
  whatWaits?: string;
}

/**
 * Stands where the reply box and the buttons that act on a turn stand, in a
 * conversation that runs on another computer. Its session lives there, so
 * it can be read here and continued only there.
 */
export function RunsElsewhere({ conversation, whatWaits }: RunsElsewhereProps) {
  const computer = useComputerOf(conversation);
  const now = useNow();

  return (
    <p className={styles.elsewhere} role="note">
      <MonitorSmartphone aria-hidden />
      <span className={styles.text}>
        <span className={styles.where}>Runs on {computer}</span>
        <span className={styles.news}>last news {sinceInWords(lastNewsOf(conversation), now)}</span>
        <span className={styles.continue}>
          {whatWaits ?? `It can be continued there, in Teamwright on ${computer}.`}
        </span>
      </span>
    </p>
  );
}

/** The name of the computer a conversation runs on. */
export function useComputerOf(conversation: Pick<Conversation, 'homeComputer'>): string {
  return homeComputerName(useSyncState().data, conversation);
}
