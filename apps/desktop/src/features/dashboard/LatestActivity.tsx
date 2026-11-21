import { ArrowRight } from 'lucide-react';
import { useMemo } from 'react';

import type { CalendarDate, Id, Project, Task } from '@/shared/api/types';
import { formatMoment, taskReference } from '@/shared/lib/format';

import { entryInWords, whenInWords } from '../activity/activityModel';
import { useOpenActivity } from '../activity/useOpenActivity';
import { useOpenArea } from '../areas/AreaContext';
import { useTags } from '../tags/tagQueries';
import { useProjectHistory } from '../tasks/taskQueries';
import type { HistoryNames } from '../tasks/taskHistoryModel';
import { useTeamMembers } from '../team/TeamContext';
import { DashboardSection } from './DashboardSection';
import { latestActivity } from './dashboardModel';
import styles from './LatestActivity.module.css';
import sprintStyles from './SprintPart.module.css';

interface LatestActivityProps {
  project: Project;
  /** The tasks the dashboard shows: those of the open area, or all. */
  tasks: readonly Task[];
  /** Every task of the project, to name the tasks a change names. */
  everyTask: readonly Task[];
  today: CalendarDate;
  onOpenTask: (taskId: Id) => void;
}

/**
 * The last few things that happened in the project, newest first, as the
 * Activity view tells them: when, the key of the task, and one sentence,
 * with the other computer named when the change came from one. The way
 * onwards opens the Activity view, or adds one if the project has none.
 */
export function LatestActivity({
  project,
  tasks,
  everyTask,
  today,
  onOpenTask,
}: LatestActivityProps) {
  const members = useTeamMembers();
  const tags = useTags(project.id).data;
  const openArea = useOpenArea();
  const history = useProjectHistory(project.id);
  const activity = useOpenActivity(project.id);
  const firstPage = history.data?.pages[0];

  const names = useMemo<HistoryNames>(
    () => ({ project, tasks: everyTask, members, tags }),
    [project, everyTask, members, tags],
  );
  const taskIds = useMemo(
    () => (openArea === undefined ? null : new Set(tasks.map((task) => task.id))),
    [openArea, tasks],
  );
  const entries = useMemo(
    () => (firstPage === undefined ? undefined : latestActivity(firstPage, project, taskIds)),
    [firstPage, project, taskIds],
  );

  return (
    <DashboardSection
      title="Latest activity"
      aside={
        <button
          type="button"
          className={sprintStyles.onwards}
          onClick={() => {
            void activity.open();
          }}
        >
          {activity.hasActivity ? 'Open the activity' : 'Add the Activity view'}
          <ArrowRight aria-hidden />
        </button>
      }
    >
      {entries === undefined ? (
        <p className={styles.quiet}>{history.isError ? 'The activity could not be read.' : ''}</p>
      ) : entries.length === 0 ? (
        <p className={styles.quiet}>Nothing has happened to the tasks yet.</p>
      ) : (
        <ul className={styles.list}>
          {entries.map((entry) => (
            <li key={entry.key} className={styles.line}>
              <time className={styles.when} dateTime={entry.at} title={formatMoment(entry.at)}>
                {whenInWords(entry, today)}
              </time>
              {entry.task.gone ? (
                <span className={styles.key}>{taskReference(project, entry.task)}</span>
              ) : (
                <button
                  type="button"
                  className={styles.key}
                  onClick={() => {
                    onOpenTask(entry.task.id);
                  }}
                >
                  {taskReference(project, entry.task)}
                </button>
              )}
              <span className={styles.what}>
                {entryInWords(entry, names)}
                {entry.computer !== undefined && (
                  <span className={styles.aside}> on {entry.computer}</span>
                )}
              </span>
            </li>
          ))}
        </ul>
      )}
    </DashboardSection>
  );
}
