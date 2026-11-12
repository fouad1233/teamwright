import { ListFilter } from 'lucide-react';

import type { View } from '@/shared/api/types';

import { useTeamMembers } from '../team/TeamContext';
import styles from './FilterLine.module.css';
import { filterInWords } from './viewModel';

interface FilterLineProps {
  view: View;
  onClear: () => void;
}

/**
 * The line at the top of a filtered view that says in words which tasks it
 * shows, with a way to show every task again. Nothing for a view that shows
 * every task.
 */
export function FilterLine({ view, onClear }: FilterLineProps) {
  const members = useTeamMembers();
  const words = filterInWords(view.filter, members);
  if (words === null) {
    return null;
  }
  return (
    <p className={styles.line}>
      <ListFilter aria-hidden className={styles.mark} />
      <span>{words}</span>
      <button type="button" className={styles.clear} onClick={onClear}>
        Show every task
      </button>
    </p>
  );
}
