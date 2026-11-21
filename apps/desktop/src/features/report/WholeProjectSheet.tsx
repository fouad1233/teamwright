import type { ReactNode } from 'react';

import type { ProjectReport, ReportTask, UsageLine, WorkShare } from '@/shared/api/types';
import { classNames } from '@/shared/lib/classNames';
import { formatDate } from '@/shared/lib/format';

import { formatTokens } from '../work/workModel';
import { dateLine } from './reportModel';
import styles from './ReportView.module.css';
import {
  askedInWords,
  dueInWords,
  hasOpenPart,
  sprintDatesInWords,
  sprintOutcomeInWords,
  waitingInWords,
} from './wholeReportModel';

interface WholeProjectSheetProps {
  report: ProjectReport;
}

/**
 * The report of the whole project (decision 0042) on one sheet of paper:
 * what was to be built, what was done, the plan against what happened,
 * the sprints, what the team used, the manager's decisions, the results
 * handed over and what is still open. Every figure and sentence is the
 * backend's; the sheet only lays them out. A part with nothing to say is
 * left out, or says so in one sentence, never as an empty heading.
 */
export function WholeProjectSheet({ report }: WholeProjectSheetProps) {
  return (
    <article className={styles.sheet} data-paper aria-label={`${report.title}: ${report.name}`}>
      <header className={styles.head}>
        <span className={styles.mark} aria-hidden />
        <p className={classNames('label', styles.kicker)}>
          {report.title} <span className={styles.key}>{report.key}</span>
        </p>
        <h1 className={styles.title}>{report.name}</h1>
        <p className={styles.date}>{dateLine(report.madeOn)}</p>
      </header>

      {report.description !== '' && (
        <p className={styles.introduction} data-selectable>
          {report.description}
        </p>
      )}

      <Part title="Summary">
        <p className={styles.lead} data-selectable>
          {report.summary.join(' ')}
        </p>
      </Part>

      {report.brief && (
        <Part title="What was to be built">
          <p className={styles.prose} data-selectable>
            {report.brief.summary}
          </p>
          <p className={styles.quiet}>
            The plan of the Advisor, as the manager approved it on{' '}
            {formatDate(report.brief.approvedOn)}.
          </p>
        </Part>
      )}

      <Part title="What was done">
        {report.finished.length === 0 ? (
          <p className={styles.quiet}>Nothing is finished yet.</p>
        ) : (
          <>
            {report.byArea.length > 0 && <Shares heading="Area" shares={report.byArea} />}
            {report.byHolder.length > 0 && <Shares heading="Who" shares={report.byHolder} />}
            <Tasks
              tasks={report.finished}
              columns={['Finished', 'Due date']}
              cells={(task) => [
                task.finishedOn === null ? '' : formatDate(task.finishedOn),
                dueInWords(task.due, task.dueOn),
              ]}
            />
          </>
        )}
      </Part>

      <Part title="Planned against what happened">
        <ul className={styles.sentences} data-selectable>
          {report.plan.sentences.map((sentence) => (
            <li key={sentence}>{sentence}</li>
          ))}
        </ul>
      </Part>

      {report.sprints.length > 0 && (
        <Part title="Sprints">
          <table className={styles.table}>
            <thead>
              <tr>
                <th className={classNames('label', styles.nameColumn)}>Sprint</th>
                <th className="label">Goal</th>
                <th className={classNames('label', styles.wideFactColumn)}>Outcome</th>
              </tr>
            </thead>
            <tbody>
              {report.sprints.map((sprint, place) => (
                // Sprints may share a name; their order is theirs.
                <tr key={place}>
                  <td>
                    {sprint.name}
                    <span className={styles.under}>{sprintDatesInWords(sprint)}</span>
                  </td>
                  <td className={styles.soft}>{sprint.goal}</td>
                  <td className={styles.fact}>{sprintOutcomeInWords(sprint)}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </Part>
      )}

      {report.usage.total.turns > 0 && (
        <Part title="What the team used">
          <Usage heading="Who" lines={report.usage.byWho} total={report.usage.total} />
          <Usage heading="Model" lines={report.usage.byModel} />
        </Part>
      )}

      {report.decisions.length > 0 && (
        <Part title="Decisions of the manager">
          {report.decisions.map((decision, place) => (
            // A decision is never changed or removed, so its place is its own.
            <div key={place} className={styles.entry}>
              <h3 className={styles.entryTitle} data-selectable>
                {decision.question}
              </h3>
              <p className={styles.quiet}>{askedInWords(decision)}</p>
              {decision.chosen !== null && (
                <p data-selectable>
                  <span className={styles.chosen}>Chosen:</span> {decision.chosen}
                </p>
              )}
              {decision.note !== '' && (
                <p className={styles.prose} data-selectable>
                  {decision.note}
                </p>
              )}
            </div>
          ))}
        </Part>
      )}

      {report.handoffs.length > 0 && (
        <Part title="Results handed over">
          {report.handoffs.map((handoff) => (
            <div key={handoff.reference} className={styles.entry}>
              <h3 className={styles.entryTitle} data-selectable>
                <span className={styles.key}>{handoff.reference}</span> {handoff.title}
              </h3>
              <p className={styles.quiet}>
                Handed over by {handoff.author} on {formatDate(handoff.publishedOn)}.
              </p>
              <p className={styles.prose} data-selectable>
                {handoff.summary}
              </p>
              {handoff.contract !== '' && (
                <pre className={styles.contract} data-selectable>
                  {handoff.contract}
                </pre>
              )}
              {handoff.files.length > 0 && (
                <ul className={styles.files} aria-label="Files">
                  {handoff.files.map((file) => (
                    <li key={file} data-selectable>
                      {file}
                    </li>
                  ))}
                </ul>
              )}
            </div>
          ))}
        </Part>
      )}

      {hasOpenPart(report) && (
        <Part title="Still open">
          {report.open.length > 0 && (
            <Tasks
              tasks={report.open}
              columns={['Status', 'Due']}
              cells={(task) => [task.status ?? '', dueInWords(task.due, task.dueOn)]}
            />
          )}
          {report.waiting > 0 && <p className={styles.quiet}>{waitingInWords(report.waiting)}</p>}
        </Part>
      )}

      <footer className={styles.foot}>
        <span>{report.name}</span>
        <span>Made with Teamwright on {formatDate(report.madeOn)}</span>
      </footer>
    </article>
  );
}

/** A part of the sheet: its name as a small label over a hairline, and what it says. */
function Part({ title, children }: { title: string; children: ReactNode }) {
  return (
    <section className={styles.part} aria-label={title}>
      <h2 className={classNames('label', styles.partTitle)}>{title}</h2>
      {children}
    </section>
  );
}

/** What each area or holder finished and has open. */
function Shares({ heading, shares }: { heading: string; shares: readonly WorkShare[] }) {
  return (
    <table className={styles.table}>
      <thead>
        <tr>
          <th className="label">{heading}</th>
          <th className={classNames('label', styles.countColumn)}>Finished</th>
          <th className={classNames('label', styles.countColumn)}>Open</th>
        </tr>
      </thead>
      <tbody>
        {shares.map((share) => (
          <tr key={share.name}>
            <td>{share.name}</td>
            <td className={styles.count}>{share.finished}</td>
            <td className={styles.count}>{share.open}</td>
          </tr>
        ))}
      </tbody>
    </table>
  );
}

interface TasksProps {
  tasks: readonly ReportTask[];
  /** The names of the two last columns. */
  columns: [string, string];
  /** What the two last columns say of a task. */
  cells: (task: ReportTask) => [string, string];
}

/** Tasks in a table with hairlines only, every one of them: the report is the whole record. */
function Tasks({ tasks, columns, cells }: TasksProps) {
  return (
    <table className={styles.table}>
      <thead>
        <tr>
          <th className={classNames('label', styles.keyColumn)}>Key</th>
          <th className="label">Task</th>
          <th className={classNames('label', styles.whoColumn)}>Who</th>
          <th className={classNames('label', styles.factColumn)}>{columns[0]}</th>
          <th className={classNames('label', styles.factColumn)}>{columns[1]}</th>
        </tr>
      </thead>
      <tbody>
        {tasks.map((task) => {
          const [first, second] = cells(task);
          return (
            <tr key={task.taskId}>
              <td className={styles.key}>{task.reference}</td>
              <td>
                {task.title}
                {task.area !== null && <span className={styles.under}>{task.area}</span>}
              </td>
              <td className={styles.soft}>{task.holder}</td>
              <td className={styles.fact}>{first}</td>
              <td className={styles.fact}>{second}</td>
            </tr>
          );
        })}
      </tbody>
    </table>
  );
}

interface UsageProps {
  heading: string;
  lines: readonly UsageLine[];
  /** The line of everything together, under the others. */
  total?: UsageLine;
}

/** What was used, a line each, in tokens and turns. */
function Usage({ heading, lines, total }: UsageProps) {
  return (
    <table className={styles.table}>
      <thead>
        <tr>
          <th className="label">{heading}</th>
          <th className={classNames('label', styles.countColumn)}>Tokens</th>
          <th className={classNames('label', styles.countColumn)}>Turns</th>
        </tr>
      </thead>
      <tbody>
        {lines.map((line) => (
          <tr key={line.name}>
            <td>{line.name}</td>
            <td className={styles.count}>{formatTokens(line.tokens)}</td>
            <td className={styles.count}>{line.turns}</td>
          </tr>
        ))}
      </tbody>
      {total && (
        <tfoot>
          <tr>
            <td>{total.name}</td>
            <td className={styles.count}>{formatTokens(total.tokens)}</td>
            <td className={styles.count}>{total.turns}</td>
          </tr>
        </tfoot>
      )}
    </table>
  );
}
