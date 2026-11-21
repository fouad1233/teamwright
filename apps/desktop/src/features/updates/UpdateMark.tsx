import { CircleArrowUp } from 'lucide-react';

import { useNavigation } from '@/app/navigation';

import styles from './UpdateMark.module.css';
import { markInWords } from './updateModel';
import { useUpdateStatus } from './updateQueries';

/**
 * A small mark at the foot of the navigation while a newer version is
 * known. It does not wait for anything: it has no joints and no gold, and
 * it leads to the page Computers, where the version is told.
 */
export function UpdateMark() {
  const navigation = useNavigation();
  const status = useUpdateStatus().data;
  const words = status ? markInWords(status) : null;

  if (words === null) {
    return null;
  }
  return (
    <button
      type="button"
      className={styles.mark}
      aria-label={`A newer version of Teamwright: ${words}`}
      onClick={navigation.openComputers}
    >
      <CircleArrowUp aria-hidden />
      <span>{words}</span>
    </button>
  );
}
