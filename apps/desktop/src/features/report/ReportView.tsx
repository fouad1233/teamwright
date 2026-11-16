import { Check, Printer } from 'lucide-react';
import { useState, type ReactNode } from 'react';
import { createPortal } from 'react-dom';

import type { Id, Project, Sprint, SprintReport, Task, View } from '@/shared/api/types';
import { classNames } from '@/shared/lib/classNames';
import { today as todayWhereTheUserIs } from '@/shared/lib/format';
import { Button } from '@/shared/ui/Button';

import { countsOf, sprintsToShow, standingInWords } from '../dashboard/dashboardModel';
import { reportVersionOf } from '../sprints/sprintModel';
import { useSprintReport, useSprints } from '../sprints/sprintQueries';
import { useTeamMembers } from '../team/TeamContext';
import type { KindProps } from '../views/viewKinds';
import { useConversationsOfProject, useUsageOfProject } from '../work/WorkContext';
import { PARTS, PART_LABELS, shows, togglePart, withIntroduction } from './reportModel';
import { ReportSheet, type ReportSources } from './ReportSheet';
import styles from './ReportView.module.css';
import { useReportSettings } from './useReportSettings';

/**
 * The report view: one page about the project as it stands when the view
 * is opened, set like a printed sheet in the middle of the view, with the
 * parts it shows chosen beside it. Everything on it is what the dashboard
 * works out. It leaves the program only as paper or a PDF.
 */
export function ReportView({ view, project, tasks }: KindProps) {
  const sprints = useSprints(project.id);
  const active = sprintsToShow(sprints.data ?? []).active;
  const page = (report: SprintReport | null) => (
    <ReportPage
      view={view}
      project={project}
      tasks={tasks}
      sprints={sprints.data ?? null}
      report={report}
    />
  );
  return active ? (
    <RunningSprintReport sprintId={active.id} version={reportVersionOf(tasks, active.id)}>
      {page}
    </RunningSprintReport>
  ) : (
    page(null)
  );
}

interface RunningSprintReportProps {
  sprintId: Id;
  /** Changes whenever the tasks of the sprint change, so the report is asked for again. */
  version: string;
  children: (report: SprintReport | null) => ReactNode;
}

/** Asks for the report of the running sprint, for its burndown, and hands it on once it has come. */
function RunningSprintReport({ sprintId, version, children }: RunningSprintReportProps) {
  const report = useSprintReport(sprintId, version);
  return children(report.data ?? null);
}

interface ReportPageProps {
  view: View;
  project: Project;
  tasks: Task[];
  sprints: readonly Sprint[] | null;
  report: SprintReport | null;
}

function ReportPage({ view, project, tasks, sprints, report }: ReportPageProps) {
  const [settings, change] = useReportSettings(view);
  const members = useTeamMembers();
  const conversations = useConversationsOfProject();
  const usage = useUsageOfProject();
  // Today is read once when the view opens, like the rest of what it shows.
  const [today] = useState(() => todayWhereTheUserIs());

  const active = sprintsToShow(sprints ?? []).active;
  const activeReport = active && report?.sprintId === active.id ? report : null;
  const sources: ReportSources = {
    project,
    tasks,
    members,
    counts: countsOf(project, tasks, conversations, today),
    standing: standingInWords({
      project,
      tasks,
      members,
      conversations,
      sprints,
      report: activeReport,
      today,
    }),
    sprints,
    active,
    report: activeReport,
    usage,
    today,
  };

  return (
    <div className={styles.view}>
      <div className={styles.ground}>
        <ReportSheet
          sources={sources}
          settings={settings}
          onIntroduce={(introduction) => {
            change((before) => withIntroduction(before, introduction));
          }}
        />
        <aside className={styles.side} aria-label="The report">
          <Button
            variant="primary"
            icon={<Printer aria-hidden />}
            onClick={() => {
              window.print();
            }}
          >
            Print or save as PDF
          </Button>
          <p className={styles.sideNote}>
            The page is made from the project as it stands now. Nothing is sent anywhere.
          </p>
          <div className={styles.parts} role="group" aria-labelledby="report-parts">
            <h2 id="report-parts" className="label">
              Parts
            </h2>
            {PARTS.map((part) => {
              const isOn = shows(settings, part);
              return (
                <button
                  key={part}
                  type="button"
                  role="checkbox"
                  aria-checked={isOn}
                  className={classNames(styles.partChoice, isOn && styles.partOn)}
                  onClick={() => {
                    change((before) => togglePart(before, part));
                  }}
                >
                  <span className={styles.box} aria-hidden>
                    {isOn && <Check />}
                  </span>
                  {PART_LABELS[part]}
                </button>
              );
            })}
          </div>
        </aside>
      </div>
      {/* The copy that is printed: the sheet alone, straight in the body of
          the document, so that nothing of the program is printed with it. */}
      {createPortal(
        <div className={styles.printCopy}>
          <ReportSheet sources={sources} settings={settings} />
        </div>,
        document.body,
      )}
    </div>
  );
}
