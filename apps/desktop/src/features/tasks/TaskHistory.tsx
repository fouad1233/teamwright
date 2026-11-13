import { useState } from 'react';

import type { Project, Task } from '@/shared/api/types';
import { formatMoment } from '@/shared/lib/format';
import { describeFailure } from '@/shared/ui/Toast';

import { useTeamMembers } from '../team/TeamContext';
import styles from './TaskHistory.module.css';
import { changeInWords } from './taskHistoryModel';
import { useTaskHistory } from './taskQueries';

/** How many changes are shown before the manager asks for all of them. */
const SHOWN_AT_FIRST = 5;

/**
 * The fewest changes worth hiding. A line that offers one more change would
 * take the room of that change, so a single one is shown instead.
 */
const FEWEST_HIDDEN = 2;

interface TaskHistoryProps {
  project: Project;
  task: Task;
  /** Every task of the project, to name the tasks this one waited for. */
  tasks: Task[];
}

/**
 * What has happened to a task since it was opened, the latest change first.
 * A long history shows its latest changes, and the rest on request.
 */
export function TaskHistory({ project, task, tasks }: TaskHistoryProps) {
  const history = useTaskHistory(task);
  const members = useTeamMembers();
  const [showsAll, setShowsAll] = useState(false);

  if (history.isError) {
    return (
      <p className={styles.note} role="alert">
        The history could not be loaded. {describeFailure(history.error)}
      </p>
    );
  }
  if (!history.data) {
    return <p className={styles.note}>Loading the history…</p>;
  }

  const latestFirst = [...history.data].reverse();
  const isLong = latestFirst.length >= SHOWN_AT_FIRST + FEWEST_HIDDEN;
  const shown = isLong && !showsAll ? latestFirst.slice(0, SHOWN_AT_FIRST) : latestFirst;
  const hiddenCount = latestFirst.length - shown.length;

  return (
    <div className={styles.history}>
      <ol className={styles.changes}>
        {shown.map((change, position) => (
          // The list only ever grows at one end, so the place from the
          // other end identifies a change.
          <li key={latestFirst.length - position} className={styles.change}>
            <time className={styles.when} dateTime={change.at}>
              {formatMoment(change.at)}
            </time>
            <span className={styles.what} data-selectable>
              {changeInWords(change, { project, tasks, members })}
            </span>
            {change.computer !== undefined && (
              // Only a change made on another computer says where: the
              // manager knows which computer they are sitting at.
              <span className={styles.where}>on {change.computer}</span>
            )}
          </li>
        ))}
      </ol>
      {hiddenCount > 0 && (
        <button
          type="button"
          className={styles.more}
          onClick={() => {
            setShowsAll(true);
          }}
        >
          Show {hiddenCount} earlier changes
        </button>
      )}
    </div>
  );
}
