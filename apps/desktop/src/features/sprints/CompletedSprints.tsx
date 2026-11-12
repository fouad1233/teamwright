import { ChevronRight } from 'lucide-react';
import { useState, type CSSProperties } from 'react';

import type { Id, Project, Sprint, Task } from '@/shared/api/types';
import { classNames } from '@/shared/lib/classNames';

import styles from './CompletedSprints.module.css';
import { SprintRecord } from './SprintRecord';
import { formatSprintDates, formatSummary, paceBarsOf } from './sprintModel';

interface CompletedSprintsProps {
  project: Project;
  sprints: readonly Sprint[];
  tasks: readonly Task[];
  onOpenTask: (taskId: Id) => void;
}

/**
 * The sprints that are over, the latest first: what each finished of what
 * it took on, in words and as a bar. The bars share one scale, so that the
 * pace from sprint to sprint can be read down the column. Pressing a line
 * unfolds the sprint in place, and pressing it again folds it.
 */
export function CompletedSprints({ project, sprints, tasks, onOpenTask }: CompletedSprintsProps) {
  const [unfolded, setUnfolded] = useState<ReadonlySet<Id>>(new Set());
  const bars = paceBarsOf(sprints);
  if (bars.length === 0) {
    return null;
  }

  function toggle(sprintId: Id) {
    setUnfolded((before) => {
      const after = new Set(before);
      if (!after.delete(sprintId)) {
        after.add(sprintId);
      }
      return after;
    });
  }

  return (
    <section className={styles.completed} aria-label="Completed sprints">
      <h2 className={styles.title}>
        <span className="label">Completed sprints</span>
      </h2>
      <ol className={styles.list}>
        {bars.map(({ sprint, summary, committedShare, completedShare }) => {
          const isOpen = unfolded.has(sprint.id);
          const recordId = `record-${sprint.id}`;
          return (
            <li key={sprint.id} className={styles.sprint}>
              <button
                type="button"
                className={styles.line}
                aria-expanded={isOpen}
                aria-controls={isOpen ? recordId : undefined}
                onClick={() => {
                  toggle(sprint.id);
                }}
              >
                <ChevronRight
                  className={classNames(styles.fold, isOpen && styles.open)}
                  aria-hidden
                />
                <span className={styles.name}>{sprint.name}</span>
                <span className={styles.dates}>{formatSprintDates(sprint)}</span>
                <span className={styles.summary}>{formatSummary(summary)}</span>
                <span
                  className={styles.bar}
                  aria-hidden
                  style={
                    {
                      '--committed': committedShare,
                      '--completed': completedShare,
                    } as CSSProperties
                  }
                >
                  <span className={styles.committed} />
                  <span className={styles.done} />
                </span>
              </button>
              {isOpen && (
                <SprintRecord
                  id={recordId}
                  project={project}
                  sprint={sprint}
                  tasks={tasks}
                  onOpenTask={onOpenTask}
                />
              )}
            </li>
          );
        })}
      </ol>
    </section>
  );
}
