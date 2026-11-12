import type { CalendarDate, Id, Project, Task } from '@/shared/api/types';
import { classNames } from '@/shared/lib/classNames';
import { calendarDateOf, formatDate, taskReference } from '@/shared/lib/format';

import { AssigneeName } from '../team/Assignee';
import { daysLate, isLate, lateInWords, toneOf } from './dashboardModel';
import styles from './TaskLines.module.css';

/** What a line says at its right end: when the task was finished, or when it is due. */
type LineFact = 'finished' | 'due';

interface TaskLinesProps {
  /** What the list is, as a small label above it. Nothing for a list that is named already. */
  title?: string;
  project: Project;
  tasks: readonly Task[];
  today: CalendarDate;
  show: LineFact;
  /** What is said where the list is empty. */
  emptyNote: string;
  onOpenTask: (taskId: Id) => void;
}

/**
 * A short list of tasks, one line each: the mark of its status, its
 * reference and title, who has it, and when it was finished or is due.
 * Pressing a line opens its task.
 */
export function TaskLines({
  title,
  project,
  tasks,
  today,
  show,
  emptyNote,
  onOpenTask,
}: TaskLinesProps) {
  return (
    <div className={styles.lines}>
      {title !== undefined && (
        <h3 className={classNames('label', styles.title)}>
          {title}
          <span className={styles.count}>{tasks.length}</span>
        </h3>
      )}
      <ul className={styles.list}>
        {tasks.map((task) => (
          <li key={task.id}>
            <button
              type="button"
              className={styles.line}
              onClick={() => {
                onOpenTask(task.id);
              }}
            >
              <span className={styles.mark} data-tone={toneOf(project, task)} aria-hidden />
              <span className={styles.reference}>{taskReference(project, task)}</span>
              <span className={styles.name}>{task.title}</span>
              <span className={styles.holder}>
                <AssigneeName assignee={task.assignee} />
              </span>
              <LineDate task={task} project={project} today={today} show={show} />
            </button>
          </li>
        ))}
        {tasks.length === 0 && <li className={styles.empty}>{emptyNote}</li>}
      </ul>
    </div>
  );
}

interface LineDateProps {
  task: Task;
  project: Project;
  today: CalendarDate;
  show: LineFact;
}

/** The date at the end of a line. A task that is late says by how much, in the red of danger. */
function LineDate({ task, project, today, show }: LineDateProps) {
  if (show === 'finished') {
    return (
      <span className={styles.date}>
        {task.completedAt === null ? '' : formatDate(calendarDateOf(new Date(task.completedAt)))}
      </span>
    );
  }
  if (isLate(project, task, today)) {
    return (
      <span className={classNames(styles.date, styles.late)}>
        {lateInWords(daysLate(task, today))}
      </span>
    );
  }
  return (
    <span className={styles.date}>
      {task.dueOn === null
        ? ''
        : task.dueOn === today
          ? 'Due today'
          : `Due ${formatDate(task.dueOn)}`}
    </span>
  );
}
