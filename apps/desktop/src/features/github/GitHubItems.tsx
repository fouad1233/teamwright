import type { GitHubItem } from '@/shared/api/types';

import { StatusMark } from '../tasks/TaskBadges';
import styles from './GitHubItems.module.css';
import { markOf, numberInWords, stateInWords } from './gitHubModel';

interface GitHubItemsProps {
  items: readonly GitHubItem[];
  /** Opens the page of an item in the manager's browser. */
  onOpen: (url: string) => void;
}

/**
 * Pull requests and issues, one line each: the state as a mark and a word,
 * the number, the title. A line opens the item on GitHub.
 */
export function GitHubItems({ items, onOpen }: GitHubItemsProps) {
  return (
    <ul className={styles.items}>
      {items.map((item) => (
        <li key={`${item.kind}-${item.number.toString()}`}>
          <button
            type="button"
            className={styles.item}
            title={`Open ${numberInWords(item)} on GitHub`}
            onClick={() => {
              onOpen(item.url);
            }}
          >
            <span className={styles.state}>
              <StatusMark status={markOf(item)} />
              {stateInWords(item)}
            </span>
            <span className={styles.number}>
              {item.kind === 'pull_request' ? 'PR' : 'Issue'} #{item.number}
            </span>
            <span className={styles.title}>{item.title}</span>
          </button>
        </li>
      ))}
    </ul>
  );
}
