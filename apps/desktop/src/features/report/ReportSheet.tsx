import type { CSSProperties, ReactNode } from 'react';

import type {
  CalendarDate,
  Member,
  Project,
  ProjectUsage,
  Sprint,
  SprintReport,
  Task,
} from '@/shared/api/types';
import { classNames } from '@/shared/lib/classNames';
import { calendarDateOf, formatDate, taskReference } from '@/shared/lib/format';

import {
  daysLate,
  lateInWords,
  tasksAtRisk,
  thisWeek,
  usageTable,
  workByHolder,
  type DashboardCounts,
} from '../dashboard/dashboardModel';
import { BurndownChart } from '../sprints/BurndownChart';
import { daysLeft, formatDaysLeft, formatSprintDates } from '../sprints/sprintModel';
import { AssigneeName } from '../team/Assignee';
import { formatTokens } from '../work/workModel';
import {
  INTRODUCTION_MAX_LENGTH,
  PART_LABELS,
  attentionInWords,
  comingNext,
  dateLine,
  firstLines,
  moreInWords,
  progressBar,
  shows,
  tasksInWork,
  type Part,
  type ReportSettings,
} from './reportModel';
import styles from './ReportView.module.css';

/** Everything the sheet is made from: what the dashboard works out, read once. */
export interface ReportSources {
  project: Project;
  tasks: readonly Task[];
  members: readonly Member[];
  counts: DashboardCounts;
  /** The sentence of the dashboard, sentence by sentence. */
  standing: readonly string[];
  /** The sprints of the project, or nothing while they are loading. */
  sprints: readonly Sprint[] | null;
  /** The running sprint, if one runs. */
  active: Sprint | undefined;
  /** Its report, once it has come. */
  report: SprintReport | null;
  usage: ProjectUsage | undefined;
  today: CalendarDate;
}

interface ReportSheetProps {
  sources: ReportSources;
  settings: ReportSettings;
  /**
   * Changes the introduction. Left out for the copy that is printed, where
   * the introduction is the paragraph it becomes.
   */
  onIntroduce?: (introduction: string) => void;
}

/**
 * The page of the report: the project's name and the date, then the parts
 * the manager chose, in the order of `PARTS`, one column of text with a
 * few quiet figures. It is paper in both themes (`data-paper`).
 */
export function ReportSheet({ sources, settings, onIntroduce }: ReportSheetProps) {
  const { project, tasks, today } = sources;
  const on = (part: Part) => shows(settings, part);
  const hasTasks = tasks.length > 0;

  return (
    <article className={styles.sheet} data-paper aria-label={`Report on ${project.name}`}>
      <header className={styles.head}>
        <span className={styles.mark} aria-hidden />
        <p className={classNames('label', styles.kicker)}>
          Report <span className={styles.key}>{project.key}</span>
        </p>
        <h1 className={styles.title}>{project.name}</h1>
        <p className={styles.date}>{dateLine(today)}</p>
      </header>

      {on('introduction') && (
        <Introduction text={settings.introduction} onIntroduce={onIntroduce} />
      )}

      {hasTasks ? (
        <>
          {on('standing') && <Standing sentences={sources.standing} />}
          {on('progress') && <Progress counts={sources.counts} />}
          {on('sprint') && (sources.sprints?.length ?? 0) > 0 && <SprintOf sources={sources} />}
          {on('finished') && (
            <TaskPart
              part="finished"
              project={project}
              tasks={thisWeek(project, tasks, today).finished}
              fact={(task) =>
                task.completedAt === null
                  ? ''
                  : formatDate(calendarDateOf(new Date(task.completedAt)))
              }
              factTitle="Finished"
              emptyNote="Nothing was finished since Monday."
            />
          )}
          {on('inWork') && (
            <TaskPart
              part="inWork"
              project={project}
              tasks={tasksInWork(project, tasks)}
              fact={(task) =>
                project.statuses.find((status) => status.id === task.statusId)?.name ?? ''
              }
              factTitle="Status"
              emptyNote="Nothing is in work."
            />
          )}
          {on('next') && (
            <TaskPart
              part="next"
              project={project}
              tasks={comingNext(project, tasks, today)}
              fact={(task) =>
                task.dueOn === null ? '' : task.dueOn === today ? 'Today' : formatDate(task.dueOn)
              }
              factTitle="Due"
              emptyNote="Nothing more is due before Sunday."
            />
          )}
          {on('attention') && <Attention sources={sources} />}
          {on('team') && <Team sources={sources} />}
          {on('usage') && <Usage sources={sources} />}
        </>
      ) : (
        <p className={styles.quiet}>
          This page tells somebody else how the project stands, on paper or as a PDF. It fills in
          once the project has tasks: add the first one with New task, or plan the project with the
          Advisor.
        </p>
      )}

      <footer className={styles.foot}>
        <span>{project.name}</span>
        <span>Made with Teamwright on {formatDate(today)}</span>
      </footer>
    </article>
  );
}

interface IntroductionProps {
  text: string;
  onIntroduce: ((introduction: string) => void) | undefined;
}

/**
 * The manager's own words. On the screen they are written in place, in a
 * plain text area that looks like the paragraph it becomes; on paper they
 * are that paragraph, and nothing when there are none.
 */
function Introduction({ text, onIntroduce }: IntroductionProps) {
  if (onIntroduce === undefined) {
    return text.trim() === '' ? null : (
      <p className={styles.introduction} data-selectable>
        {text}
      </p>
    );
  }
  return (
    <textarea
      className={classNames(styles.introduction, styles.introductionInput)}
      value={text}
      maxLength={INTRODUCTION_MAX_LENGTH}
      rows={2}
      spellCheck
      aria-label="Introduction"
      placeholder="Write a few words for the reader: what this page is for, and what you would like from them. They are kept with the view."
      onChange={(event) => {
        onIntroduce(event.target.value);
      }}
    />
  );
}

/** A part of the sheet: its name as a small label over a hairline, and what it says. */
function PartOf({ part, children }: { part: Part; children: ReactNode }) {
  return (
    <section className={styles.part} aria-label={PART_LABELS[part]}>
      <h2 className={classNames('label', styles.partTitle)}>{PART_LABELS[part]}</h2>
      {children}
    </section>
  );
}

/** The sentence of the dashboard, as the first paragraph of the page. */
function Standing({ sentences }: { sentences: readonly string[] }) {
  return (
    <PartOf part="standing">
      <p className={styles.lead} data-selectable>
        {sentences.join(' ')}
      </p>
    </PartOf>
  );
}

/** Progress as one bar, finished in ink and begun in a thinner ink, with the figures under it. */
function Progress({ counts }: { counts: DashboardCounts }) {
  const bar = progressBar(counts);
  const figures = [
    { label: 'Tasks', value: counts.total },
    { label: 'Finished', value: counts.finished },
    { label: 'In work', value: counts.inWork },
    { label: 'Not begun', value: counts.open - counts.inWork },
    { label: 'Late', value: counts.late },
  ];
  return (
    <PartOf part="progress">
      <div
        className={styles.bar}
        role="img"
        aria-label={`${counts.finished.toString()} of ${counts.total.toString()} tasks finished, ${counts.inWork.toString()} in work`}
        style={{ '--finished': bar.finished, '--in-work': bar.inWork } as CSSProperties}
      >
        <span className={styles.barFinished} />
        <span className={styles.barInWork} />
      </div>
      <dl className={styles.figures}>
        {figures.map((figure) => (
          <div key={figure.label} className={styles.figure}>
            <dt className="label">{figure.label}</dt>
            <dd className={styles.figureValue}>{figure.value}</dd>
          </div>
        ))}
      </dl>
    </PartOf>
  );
}

/** The running sprint: its name, dates and goal, and its burndown drawn small in ink. */
function SprintOf({ sources }: { sources: ReportSources }) {
  const { active, report, today } = sources;
  if (!active) {
    return (
      <PartOf part="sprint">
        <p className={styles.quiet}>No sprint is running.</p>
      </PartOf>
    );
  }
  const facts = [
    formatSprintDates(active),
    ...(active.endOn === null ? [] : [formatDaysLeft(daysLeft(active.endOn, today))]),
  ];
  return (
    <PartOf part="sprint">
      <div className={styles.sprint}>
        <div className={styles.sprintWords}>
          <h3 className={styles.sprintName}>{active.name}</h3>
          <p className={styles.sprintFacts}>{facts.join('  ·  ')}</p>
          {active.goal !== '' && (
            <p className={styles.sprintGoal} data-selectable>
              {active.goal}
            </p>
          )}
        </div>
        {report?.sprintId === active.id && <BurndownChart report={report} today={today} />}
      </div>
    </PartOf>
  );
}

interface TaskPartProps {
  part: Part;
  project: Project;
  tasks: readonly Task[];
  /** What the last column says of a task. */
  fact: (task: Task) => string;
  factTitle: string;
  emptyNote: string;
}

/** A part that names tasks: a table with hairlines only, cut short when it is long. */
function TaskPart({ part, project, tasks, fact, factTitle, emptyNote }: TaskPartProps) {
  return (
    <PartOf part={part}>
      <TaskTable
        project={project}
        tasks={tasks}
        fact={fact}
        factTitle={factTitle}
        emptyNote={emptyNote}
      />
    </PartOf>
  );
}

type TaskTableProps = Omit<TaskPartProps, 'part'>;

function TaskTable({ project, tasks, fact, factTitle, emptyNote }: TaskTableProps) {
  if (tasks.length === 0) {
    return <p className={styles.quiet}>{emptyNote}</p>;
  }
  const { lines, more } = firstLines(tasks);
  return (
    <>
      <table className={styles.table}>
        <thead>
          <tr>
            <th className={classNames('label', styles.keyColumn)}>Key</th>
            <th className="label">Task</th>
            <th className={classNames('label', styles.whoColumn)}>Who</th>
            <th className={classNames('label', styles.factColumn)}>{factTitle}</th>
          </tr>
        </thead>
        <tbody>
          {lines.map((task) => (
            <tr key={task.id}>
              <td className={styles.key}>{taskReference(project, task)}</td>
              <td>{task.title}</td>
              <td className={styles.soft}>
                {task.assignee?.kind === 'manager' ? (
                  MANAGER_ON_PAPER
                ) : (
                  <AssigneeName assignee={task.assignee} />
                )}
              </td>
              <td className={styles.fact}>{fact(task)}</td>
            </tr>
          ))}
        </tbody>
      </table>
      {more > 0 && <p className={styles.more}>{moreInWords(more)}</p>}
    </>
  );
}

/** What is late, by name, and how much waits for the manager. */
function Attention({ sources }: { sources: ReportSources }) {
  const { project, tasks, counts, today } = sources;
  const late = tasksAtRisk(project, tasks, today);
  return (
    <PartOf part="attention">
      {late.length > 0 && (
        <TaskTable
          project={project}
          tasks={late}
          fact={(task) => lateInWords(daysLate(task, today))}
          factTitle="Late by"
          emptyNote=""
        />
      )}
      <p className={styles.quiet}>{attentionInWords(late.length, counts.waiting)}</p>
    </PartOf>
  );
}

/** The manager on paper: to the reader, the manager is not "You". */
const MANAGER_ON_PAPER = 'The manager';

/** Who a holder is on paper. */
function holderName(name: string, kind: 'manager' | 'member' | 'nobody'): string {
  if (kind === 'manager') {
    return MANAGER_ON_PAPER;
  }
  return kind === 'nobody' ? 'Given to nobody' : name;
}

/** The team: each person, what they are responsible for, and what they hold. */
function Team({ sources }: { sources: ReportSources }) {
  const holders = workByHolder(sources.project, sources.tasks, sources.members);
  return (
    <PartOf part="team">
      <table className={styles.table}>
        <thead>
          <tr>
            <th className={classNames('label', styles.nameColumn)}>Who</th>
            <th className="label">Responsible for</th>
            <th className={classNames('label', styles.countColumn)}>Open</th>
            <th className={classNames('label', styles.countColumn)}>Finished</th>
          </tr>
        </thead>
        <tbody>
          {holders.map((holder) => (
            <tr key={holder.key}>
              <td>{holderName(holder.name, holder.kind)}</td>
              <td className={styles.soft}>
                {holder.kind === 'manager'
                  ? 'Runs the project'
                  : (holder.member?.responsibility ?? '')}
              </td>
              <td className={styles.count}>{holder.open}</td>
              <td className={styles.count}>{holder.finished}</td>
            </tr>
          ))}
        </tbody>
      </table>
      {sources.members.length === 0 && <p className={styles.quiet}>Nobody is on the team yet.</p>}
    </PartOf>
  );
}

/** What the specialists and the Advisor used, in tokens and turns, as the dashboard counts it. */
function Usage({ sources }: { sources: ReportSources }) {
  const table = sources.usage ? usageTable(sources.members, sources.usage) : undefined;
  return (
    <PartOf part="usage">
      {!table || table.rows.length === 0 ? (
        <p className={styles.quiet}>
          {!table ? 'Counting what was used.' : 'Nothing has used tokens yet.'}
        </p>
      ) : (
        <table className={styles.table}>
          <thead>
            <tr>
              <th className={classNames('label', styles.nameColumn)}>Who</th>
              <th className={classNames('label', styles.countColumn)}>Tokens</th>
              <th className={classNames('label', styles.countColumn)}>Turns</th>
              <th className={classNames('label', styles.factColumn)}>Model last</th>
            </tr>
          </thead>
          <tbody>
            {table.rows.map((row) => (
              <tr key={row.key}>
                <td>{row.name}</td>
                <td className={styles.count}>{formatTokens(row.usage.allTokens)}</td>
                <td className={styles.count}>{row.usage.turns}</td>
                <td className={styles.fact}>{row.usage.lastModel ?? ''}</td>
              </tr>
            ))}
          </tbody>
          <tfoot>
            <tr>
              <td>{table.total.name}</td>
              <td className={styles.count}>{formatTokens(table.total.usage.allTokens)}</td>
              <td className={styles.count}>{table.total.usage.turns}</td>
              <td />
            </tr>
          </tfoot>
        </table>
      )}
    </PartOf>
  );
}
