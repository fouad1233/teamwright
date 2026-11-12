import { useDroppable } from '@dnd-kit/core';
import type { ReactNode } from 'react';

import type { Id, Project, Sprint, Task } from '@/shared/api/types';
import { classNames } from '@/shared/lib/classNames';

import { BacklogRow } from './BacklogRow';
import styles from './SprintSection.module.css';
import { formatTotals, totalsOf } from './sprintModel';

/** The identifier under which the backlog accepts dropped rows. */
export const BACKLOG_DROP_ID = 'backlog';

interface SprintSectionProps {
  project: Project;
  /** The sprint, or nothing for the backlog. */
  sprint: Sprint | null;
  /** What the section is called: the name of the sprint, or "Backlog". */
  name: string;
  /** A word after the name that says where the sprint stands. */
  state?: ReactNode;
  /** The facts under the name: dates and what is left. The totals follow them. */
  facts?: readonly string[];
  /** The tasks of the section, as rows. */
  rows: readonly Task[];
  destinations: readonly Sprint[];
  /** What stands next to the heading: the burndown of the active sprint. */
  chart?: ReactNode;
  /** The buttons that do something with the sprint. */
  actions?: ReactNode;
  /** What is said where there are no rows. */
  emptyNote: string;
  /** What follows the rows: a line to add a task to the backlog. */
  footer?: ReactNode;
  onOpenTask: (taskId: Id) => void;
  onPlan: (taskId: Id, sprintId: Id | null) => void;
  onSetPoints: (taskId: Id, points: number | null) => Promise<void>;
}

/**
 * One sprint, or the backlog: a heading with its name, its facts and what
 * can be done with it, and its tasks as rows. A row dropped on it is
 * planned into it.
 */
export function SprintSection({
  project,
  sprint,
  name,
  state,
  facts = [],
  rows,
  destinations,
  chart,
  actions,
  emptyNote,
  footer,
  onOpenTask,
  onPlan,
  onSetPoints,
}: SprintSectionProps) {
  const { setNodeRef, isOver } = useDroppable({ id: sprint?.id ?? BACKLOG_DROP_ID });
  const allFacts = [...facts, formatTotals(totalsOf(rows))];

  return (
    <section
      ref={setNodeRef}
      className={classNames(styles.section, isOver && styles.dropTarget)}
      aria-label={name}
    >
      <header className={styles.header}>
        <div className={styles.heading}>
          <div className={styles.titleLine}>
            <h2 className={styles.name}>{name}</h2>
            {state}
          </div>
          <p className={styles.facts}>
            {allFacts.map((fact, place) => (
              <span key={fact} className={styles.fact}>
                {place > 0 && <span className={styles.separator} aria-hidden />}
                {fact}
              </span>
            ))}
          </p>
          {sprint && sprint.goal !== '' && <p className={styles.goal}>{sprint.goal}</p>}
        </div>
        {chart}
        {actions && <div className={styles.actions}>{actions}</div>}
      </header>

      <ul className={styles.rows}>
        {rows.map((task) => (
          <BacklogRow
            key={task.id}
            project={project}
            task={task}
            destinations={destinations}
            onOpen={onOpenTask}
            onPlan={onPlan}
            onSetPoints={onSetPoints}
          />
        ))}
        {rows.length === 0 && <li className={styles.empty}>{emptyNote}</li>}
      </ul>
      {footer}
    </section>
  );
}
