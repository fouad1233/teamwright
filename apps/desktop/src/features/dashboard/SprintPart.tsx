import { ArrowRight } from 'lucide-react';

import type { CalendarDate, Id, Project, Sprint, SprintReport, Task } from '@/shared/api/types';
import { classNames } from '@/shared/lib/classNames';

import { BurndownChart } from '../sprints/BurndownChart';
import { CompletedSprints } from '../sprints/CompletedSprints';
import { daysLeft, formatDaysLeft, formatSprintDates } from '../sprints/sprintModel';
import { DashboardSection } from './DashboardSection';
import styles from './SprintPart.module.css';

interface SprintPartProps {
  project: Project;
  /** The tasks of the project: a completed sprint shows the ones it finished. */
  tasks: readonly Task[];
  /** The sprint that runs, if one does. */
  active: Sprint | undefined;
  /** Its report, once it has come. */
  report: SprintReport | null;
  /** The sprints that are over, the latest first. */
  completed: readonly Sprint[];
  today: CalendarDate;
  /** Opens the backlog view of the project. Left out when it has none: there is then no way there. */
  onOpenBacklog?: () => void;
  onOpenTask: (taskId: Id) => void;
}

/**
 * The sprint, for a project that uses sprints: the running one with its
 * burndown, and the completed ones side by side with what each took on
 * and what it finished. The rest is in the Backlog view.
 */
export function SprintPart({
  project,
  tasks,
  active,
  report,
  completed,
  today,
  onOpenBacklog,
  onOpenTask,
}: SprintPartProps) {
  const facts = active
    ? [
        formatSprintDates(active),
        ...(active.endOn === null ? [] : [formatDaysLeft(daysLeft(active.endOn, today))]),
      ]
    : [];

  return (
    <DashboardSection
      title="The sprint"
      aside={
        onOpenBacklog && (
          <button type="button" className={styles.onwards} onClick={onOpenBacklog}>
            Open the backlog
            <ArrowRight aria-hidden />
          </button>
        )
      }
    >
      {active ? (
        <div className={styles.running}>
          <div className={styles.heading}>
            <div className={styles.titleLine}>
              <h3 className={styles.name}>{active.name}</h3>
              <span className={classNames('label', styles.state)}>Active</span>
            </div>
            <p className={styles.facts}>
              {facts.map((fact, place) => (
                <span key={fact} className={styles.fact}>
                  {place > 0 && <span className={styles.separator} aria-hidden />}
                  {fact}
                </span>
              ))}
            </p>
            {active.goal !== '' && <p className={styles.goal}>{active.goal}</p>}
          </div>
          {report && <BurndownChart report={report} today={today} />}
        </div>
      ) : (
        <p className={styles.quiet}>No sprint is running.</p>
      )}
      <CompletedSprints
        project={project}
        sprints={completed}
        tasks={tasks}
        onOpenTask={onOpenTask}
      />
    </DashboardSection>
  );
}
