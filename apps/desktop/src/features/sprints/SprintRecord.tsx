import type { Id, Project, Sprint, Task } from '@/shared/api/types';

import { RecordRow } from './BacklogRow';
import { BurndownChart } from './BurndownChart';
import styles from './SprintRecord.module.css';
import { movedOnNote, recordFactsOf, reportVersionOf, rowsOf } from './sprintModel';
import { useSprintReport } from './sprintQueries';

interface SprintRecordProps {
  id: string;
  project: Project;
  sprint: Sprint;
  tasks: readonly Task[];
  onOpenTask: (taskId: Id) => void;
}

/**
 * What stays of a completed sprint, read from top to bottom: its goal and
 * facts, its burndown as it ended, and the tasks that are in it. Those are
 * the tasks it finished: the others moved on when it was completed, and
 * which ones they were is not recorded, so the note under the list only
 * says that some did.
 */
export function SprintRecord({ id, project, sprint, tasks, onOpenTask }: SprintRecordProps) {
  const report = useSprintReport(sprint.id, reportVersionOf(tasks, sprint.id));
  const rows = rowsOf(project, tasks, sprint.id);
  const note = sprint.summary ? movedOnNote(sprint.summary) : null;

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
        {report.data && <BurndownChart report={report.data} today={null} />}
      </div>

      <ul className={styles.rows}>
        {rows.map((task) => (
          <RecordRow key={task.id} project={project} task={task} onOpen={onOpenTask} />
        ))}
        {rows.length === 0 && <li className={styles.empty}>No task is left in this sprint.</li>}
      </ul>
      {note && <p className={styles.note}>{note}</p>}
    </div>
  );
}
