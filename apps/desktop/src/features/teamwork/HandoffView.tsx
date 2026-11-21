import type { Handoff } from '@/shared/api/types';
import { classNames } from '@/shared/lib/classNames';
import { formatMoment } from '@/shared/lib/format';
import { Prose } from '@/shared/ui/Prose';

import { useTeamMembers } from '../team/TeamContext';
import styles from './HandoffView.module.css';
import { nameOf } from './teamworkModel';

interface HandoffViewProps {
  handoff: Handoff;
  /** How many handoffs the task published before this one. */
  earlier?: number;
}

/**
 * What a specialist handed over with a task: its summary, the interface
 * contract others may rely on, and the files that hold the result. It is
 * what a task that builds on this one starts with.
 */
export function HandoffView({ handoff, earlier = 0 }: HandoffViewProps) {
  const members = useTeamMembers();
  const byline = [nameOf(members, handoff.authorId), formatMoment(handoff.publishedAt)];
  if (earlier > 0) {
    byline.push(
      earlier === 1 ? 'replaces an earlier one' : `replaces ${earlier.toString()} earlier`,
    );
  }

  return (
    <div className={styles.handoff}>
      <Prose text={handoff.summary} />
      {handoff.contract !== '' && (
        <div className={styles.part}>
          <h4 className="label">Interface contract</h4>
          <p className={styles.contract} data-selectable>
            {handoff.contract}
          </p>
        </div>
      )}
      {handoff.files.length > 0 && (
        <div className={styles.part}>
          <h4 className="label">Files</h4>
          <ul className={styles.files}>
            {handoff.files.map((file) => (
              <li key={file} className={styles.file} title={file} data-selectable>
                {file}
              </li>
            ))}
          </ul>
        </div>
      )}
      <p className={classNames(styles.byline)}>{byline.join(' · ')}</p>
    </div>
  );
}
