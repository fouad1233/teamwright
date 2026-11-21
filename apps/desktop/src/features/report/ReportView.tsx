import { Check, FileDown, Printer } from 'lucide-react';
import { useState, type ReactNode } from 'react';
import { createPortal } from 'react-dom';

import { useIsDemo } from '@/shared/api/BackendProvider';
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
import switchStyles from '../sprints/SprintBoard.module.css';
import {
  PARTS,
  PART_LABELS,
  SHEET_LABELS,
  shows,
  togglePart,
  withIntroduction,
  withSheet,
  type Sheet,
} from './reportModel';
import { useProjectReport, useSaveReport } from './reportQueries';
import { ReportSheet, type ReportSources } from './ReportSheet';
import styles from './ReportView.module.css';
import { useReportSettings } from './useReportSettings';
import { WholeProjectSheet } from './WholeProjectSheet';

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

  const isWhole = settings.sheet === 'whole';
  const whole = useProjectReport(project.id, isWhole).data;
  const saveReport = useSaveReport();
  // A browser has no files to keep the report in.
  const isDemo = useIsDemo();
  const nowSheet = (onIntroduce?: (introduction: string) => void) => (
    <ReportSheet sources={sources} settings={settings} onIntroduce={onIntroduce} />
  );
  const wholeSheet = whole ? (
    <WholeProjectSheet report={whole} />
  ) : (
    <p className={styles.sideNote}>Gathering the report of the project.</p>
  );

  return (
    <div className={styles.view}>
      <div className={styles.ground}>
        {isWhole
          ? wholeSheet
          : nowSheet((introduction) => {
              change((before) => withIntroduction(before, introduction));
            })}
        <aside className={styles.side} aria-label="The report">
          <SheetChoice
            sheet={settings.sheet}
            onChoose={(sheet) => {
              change((before) => withSheet(before, sheet));
            }}
          />
          <Button
            variant="primary"
            icon={<Printer aria-hidden />}
            onClick={() => {
              window.print();
            }}
          >
            Print or save as PDF
          </Button>
          {isWhole && (
            <Button
              icon={<FileDown aria-hidden />}
              disabled={isDemo}
              onClick={() => {
                void saveReport(project);
              }}
            >
              Save as Markdown
            </Button>
          )}
          <p className={styles.sideNote}>
            {isWhole
              ? isDemo
                ? 'The whole project, from everything it has stored. The browser demo has no files to keep it in.'
                : 'The whole project, from everything it has stored. Markdown is kept in the work folder, next to the work.'
              : 'The page is made from the project as it stands now. Nothing is sent anywhere.'}
          </p>
          {!isWhole && (
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
          )}
        </aside>
      </div>
      {/* The copy that is printed: the sheet alone, straight in the body of
          the document, so that nothing of the program is printed with it. */}
      {createPortal(
        <div className={styles.printCopy}>{isWhole ? wholeSheet : nowSheet()}</div>,
        document.body,
      )}
    </div>
  );
}

const SHEETS: readonly Sheet[] = ['now', 'whole'];

/** The choice of the sheet: the page of now, or the report of the whole project. */
function SheetChoice({ sheet, onChoose }: { sheet: Sheet; onChoose: (sheet: Sheet) => void }) {
  return (
    <div
      className={classNames(switchStyles.switch, styles.sheetChoice)}
      role="radiogroup"
      aria-label="Which report"
    >
      {SHEETS.map((each) => {
        const isChosen = each === sheet;
        return (
          <button
            key={each}
            type="button"
            role="radio"
            aria-checked={isChosen}
            className={classNames(switchStyles.choice, isChosen && switchStyles.chosen)}
            onClick={() => {
              onChoose(each);
            }}
          >
            {SHEET_LABELS[each]}
          </button>
        );
      })}
    </div>
  );
}
