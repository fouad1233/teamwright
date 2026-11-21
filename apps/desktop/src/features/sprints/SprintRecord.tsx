import type { Id, Project, Sprint, Task } from '@/shared/api/types';
import { taskReference } from '@/shared/lib/format';

import { RecordRow } from './BacklogRow';
import { BurndownChart } from './BurndownChart';
import styles from './SprintRecord.module.css';
import { movedOnLinesOf, movedOnNote, recordFactsOf, reportVersionOf, rowsOf } from './sprintModel';
import { useSprintReport, useSprints } from './sprintQueries';

interface SprintRecordProps {
  id: string;
  project: Project;
  sprint: Sprint;
  tasks: readonly Task[];
  onOpenTask: (taskId: Id) => void;
}

/**
 * What stays of a completed sprint, read from top to bottom: its goal and
 * facts, its burndown as it ended, the tasks that are in it, which are the
 * ones it finished, and the tasks that moved on when it was completed, with
 * where each went. Those are read from the log by the backend; while its
 * report has not come, a note only says that some moved on.
 */
export function SprintRecord({ id, project, sprint, tasks, onOpenTask }: SprintRecordProps) {
  const report = useSprintReport(sprint.id, reportVersionOf(tasks, sprint.id));
  const sprints = useSprints(project.id);
  const rows = rowsOf(project, tasks, sprint.id);
  const movedOn = report.data ? movedOnLinesOf(report.data, sprints.data ?? []) : [];
  const note = sprint.summary && movedOn.length === 0 ? movedOnNote(sprint.summary) : null;

  return (
    <div id={id} className={styles.record} role="region" aria-label={`${sprint.name}, completed`}>
      <div className={styles.top}>
        <div className={styles.about}>
          {sprint.goal !== '' && <p className={styles.goal}>{sprint.goal}</p>}
          <dl className={styles.facts}>
            {recordFactsOf(sprint).map((fact) => (
              <div key={fact.label} className={styles.fact}>
                <dt className="label">{fact.label}</dt>
                <dd>{fact.value}</dd>
              </div>
            ))}
          </dl>
        </div>
        {report.data && <BurndownChart project={project} report={report.data} today={null} />}
      </div>

      <ul className={styles.rows}>
        {rows.map((task) => (
          <RecordRow key={task.id} project={project} task={task} onOpen={onOpenTask} />
        ))}
        {rows.length === 0 && <li className={styles.empty}>No task is left in this sprint.</li>}
      </ul>
      {movedOn.length > 0 && (
        <div className={styles.movedOn}>
          <h3 className="label">Moved on when it was completed</h3>
          <ul className={styles.movedRows}>
            {movedOn.map((line) => (
              <li key={line.taskId} className={styles.movedRow}>
                <button
                  type="button"
                  className={styles.movedTask}
                  onClick={() => {
                    onOpenTask(line.taskId);
                  }}
                >
                  <span className={styles.reference}>{taskReference(project, line)}</span>
                  <span className={styles.title}>{line.title}</span>
                </button>
                <span className={styles.movedFacts}>
                  {line.work} · to {line.to}
                </span>
              </li>
            ))}
          </ul>
        </div>
      )}
      {note && <p className={styles.note}>{note}</p>}
    </div>
  );
}
