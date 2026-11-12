import { useMemo, useState, type ReactNode } from 'react';

import { useNavigation } from '@/app/navigation';
import type { Id, Project, Sprint, SprintReport, Task } from '@/shared/api/types';
import { today as todayWhereTheUserIs } from '@/shared/lib/format';

import { reportVersionOf } from '../sprints/sprintModel';
import { useSprintReport, useSprints } from '../sprints/sprintQueries';
import { useTeamMembers } from '../team/TeamContext';
import { useConversationsOfProject } from '../work/WorkContext';
import { DashboardSection } from './DashboardSection';
import styles from './DashboardView.module.css';
import {
  countsOf,
  sprintsToShow,
  standingInWords,
  tasksAtRisk,
  thisWeek,
  usageOf,
  workByHolder,
  workByStatus,
} from './dashboardModel';
import { NumberTiles } from './NumberTiles';
import { ProjectNote } from './ProjectNote';
import { SprintPart } from './SprintPart';
import { Standing } from './Standing';
import { TaskLines } from './TaskLines';
import { UsagePart } from './UsagePart';
import { HolderBars, StatusBar } from './WorkCharts';

interface DashboardViewProps {
  project: Project;
  tasks: Task[];
}

/**
 * How a project is going, on one page to read from top to bottom
 * (decision 0020): where it stands in words, the manager's own note, a
 * few plain numbers, the work by status and by member, this week, the
 * sprint and what the specialists used. Everything is worked out by rules
 * in `dashboardModel.ts` from what the interface already knows; nothing
 * on it costs a token.
 */
export function DashboardView({ project, tasks }: DashboardViewProps) {
  const sprints = useSprints(project.id);
  const active = sprintsToShow(sprints.data ?? []).active;
  const page = (report: SprintReport | null) => (
    <DashboardPage project={project} tasks={tasks} sprints={sprints.data ?? null} report={report} />
  );
  return active ? (
    <ActiveSprintReport sprintId={active.id} version={reportVersionOf(tasks, active.id)}>
      {page}
    </ActiveSprintReport>
  ) : (
    page(null)
  );
}

interface ActiveSprintReportProps {
  sprintId: Id;
  /** Changes whenever the tasks of the sprint change, so the report is asked for again. */
  version: string;
  children: (report: SprintReport | null) => ReactNode;
}

/** Asks for the report of the running sprint, and hands it on once it has come. */
function ActiveSprintReport({ sprintId, version, children }: ActiveSprintReportProps) {
  const report = useSprintReport(sprintId, version);
  return children(report.data ?? null);
}

interface DashboardPageProps extends DashboardViewProps {
  /** The sprints of the project, or nothing while they are loading. */
  sprints: readonly Sprint[] | null;
  /** The report of the running sprint, once it has come. */
  report: SprintReport | null;
}

/** The page itself, once it is known whether a sprint runs. */
function DashboardPage({ project, tasks, sprints, report }: DashboardPageProps) {
  const navigation = useNavigation();
  const members = useTeamMembers();
  const conversations = useConversationsOfProject();
  // Today is read once when the view opens, like the rest of what it shows.
  const [today] = useState(() => todayWhereTheUserIs());

  const { active, completed } = sprintsToShow(sprints ?? []);
  const activeReport = active && report?.sprintId === active.id ? report : null;

  const counts = useMemo(
    () => countsOf(project, tasks, conversations, today),
    [project, tasks, conversations, today],
  );
  const standing = standingInWords({
    project,
    tasks,
    members,
    conversations,
    sprints,
    report: activeReport,
    today,
  });
  const atRisk = tasksAtRisk(project, tasks, today);
  const week = thisWeek(project, tasks, today);
  const usage = usageOf(members, conversations);
  const hasTasks = tasks.length > 0;
  const usesSprints = (sprints?.length ?? 0) > 0;

  return (
    <div className={styles.dashboard}>
      <div className={styles.sheet}>
        <div className={styles.top}>
          <Standing
            project={project}
            sentences={standing}
            atRisk={atRisk}
            today={today}
            onOpenTask={navigation.openTask}
          />
          <ProjectNote
            // A note of its own for every project and every stored text, so
            // that nothing typed for one lingers in another.
            key={`${project.id}:${project.description}`}
            project={project}
          />
        </div>

        {/* A row of noughts says nothing the sentence above does not. */}
        {(hasTasks || counts.waiting > 0) && (
          <NumberTiles counts={counts} onOpenInbox={navigation.openInbox} />
        )}

        {hasTasks ? (
          <>
            <DashboardSection title="Work by status">
              <StatusBar shares={workByStatus(project, tasks)} total={tasks.length} />
            </DashboardSection>

            <div className={styles.pair}>
              <DashboardSection title="Work by member">
                <HolderBars holders={workByHolder(project, tasks, members)} />
                {members.length === 0 && (
                  <p className={styles.quiet}>
                    Nobody is on the team yet. Add members in the Team view.
                  </p>
                )}
              </DashboardSection>

              <DashboardSection title="This week">
                <div className={styles.week}>
                  <TaskLines
                    title="Finished"
                    project={project}
                    tasks={week.finished}
                    today={today}
                    show="finished"
                    emptyNote="Nothing finished since Monday."
                    onOpenTask={navigation.openTask}
                  />
                  <TaskLines
                    title="Due or late"
                    project={project}
                    tasks={week.due}
                    today={today}
                    show="due"
                    emptyNote="Nothing due before Sunday."
                    onOpenTask={navigation.openTask}
                  />
                </div>
              </DashboardSection>
            </div>
          </>
        ) : (
          <p className={styles.quiet}>
            The charts appear once the project has tasks. Add the first one with New task, or plan
            the project with the Advisor.
          </p>
        )}

        {usesSprints && (
          <SprintPart
            project={project}
            tasks={tasks}
            active={active}
            report={activeReport}
            completed={completed}
            today={today}
            onOpenBacklog={() => {
              navigation.showView('backlog');
            }}
            onOpenTask={navigation.openTask}
          />
        )}

        <UsagePart usage={usage} hasTeam={members.length > 0} />
      </div>
    </div>
  );
}
