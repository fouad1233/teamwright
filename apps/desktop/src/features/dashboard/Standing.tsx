import type { CalendarDate, Id, Project, Task } from '@/shared/api/types';

import styles from './Standing.module.css';
import { TaskLines } from './TaskLines';

interface StandingProps {
  project: Project;
  /** Where the project stands, sentence by sentence. */
  sentences: readonly string[];
  /** The tasks that are not finished and past their due date. */
  atRisk: readonly Task[];
  today: CalendarDate;
  onOpenTask: (taskId: Id) => void;
}

/**
 * Where the project stands, in a few sentences worked out by rules, and
 * then by name the tasks at risk. The list is left out when nothing is
 * late.
 */
export function Standing({ project, sentences, atRisk, today, onOpenTask }: StandingProps) {
  return (
    <section className={styles.standing} aria-label="Where the project stands">
      <h2 className="label">Where the project stands</h2>
      <p className={styles.words} data-selectable>
        {sentences.map((sentence) => (
          <span key={sentence} className={styles.sentence}>
            {sentence}{' '}
          </span>
        ))}
      </p>
      {atRisk.length > 0 && (
        <TaskLines
          title="At risk"
          project={project}
          tasks={atRisk}
          today={today}
          show="due"
          emptyNote=""
          onOpenTask={onOpenTask}
        />
      )}
    </section>
  );
}
