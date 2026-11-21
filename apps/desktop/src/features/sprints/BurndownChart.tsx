import type { CalendarDate, Project, ScopeTask, SprintReport } from '@/shared/api/types';
import { formatDate, taskReference } from '@/shared/lib/format';

import styles from './BurndownChart.module.css';
import { burndownOf, type ScopeMark } from './burndownModel';
import { formatWork } from './sprintModel';

/** The size of the plot, in pixels: small enough to stand next to the name of a sprint. */
const PLOT = { width: 208, height: 52 };

/** The room below the plot for the ticks of the days. */
const TICK_LENGTH = 4;

/** The room around the plot, so that the strokes and the marks at its edges are not cut. */
const MARGIN = 4;

/** Half the width of the mark of a change of scope. */
const MARK = 3;

/** The most days of change listed under the chart; the rest are counted. */
const MOST_CHANGES_LISTED = 3;

interface BurndownChartProps {
  project: Pick<Project, 'key'>;
  report: SprintReport;
  /** Today, for its mark. Nothing for a sprint that is over, whose chart says so. */
  today: CalendarDate | null;
}

/**
 * The burndown of a sprint: what work remained at the end of each day, in
 * ink, against the pace its dates ask for, as a hairline. Today is marked in
 * brass. A tick under the plot stands for every day.
 *
 * When tasks joined or left the sprint while it ran, the line steps up or
 * down on that day, a dotted line shows all the work the sprint held, and
 * the day gets a small triangle that points up or down: the shape says
 * which way, so it does not rest on colour. Under the chart each such day is
 * said in words, with the tasks named on hover.
 */
export function BurndownChart({ project, report, today }: BurndownChartProps) {
  const chart = burndownOf(report, PLOT, today);
  if (!chart) {
    return null;
  }
  const width = PLOT.width + 2 * MARGIN;
  const height = PLOT.height + TICK_LENGTH + 2 * MARGIN;
  const left =
    chart.remaining === null
      ? 'Not started'
      : today === null
        ? `${chart.remaining.toString()} of ${formatWork(report.total, report.unit)} left at the end`
        : `${chart.remaining.toString()} of ${formatWork(report.total, report.unit)} left`;
  const listed = chart.marks.slice(-MOST_CHANGES_LISTED);
  const unlisted = chart.marks.length - listed.length;

  return (
    <figure className={styles.chart}>
      <svg
        className={styles.plot}
        width={width}
        height={height}
        viewBox={`0 0 ${width.toString()} ${height.toString()}`}
        role="img"
        aria-label={`Burndown: ${left}`}
      >
        <g transform={`translate(${MARGIN.toString()} ${MARGIN.toString()})`}>
          <line className={styles.base} x1={0} x2={PLOT.width} y1={PLOT.height} y2={PLOT.height} />
          {chart.dayTicks.map((x) => (
            <line
              key={x}
              className={styles.tick}
              x1={x}
              x2={x}
              y1={PLOT.height}
              y2={PLOT.height + TICK_LENGTH}
            />
          ))}
          <path className={styles.ideal} d={chart.ideal} />
          {chart.scope !== null && <path className={styles.scope} d={chart.scope} />}
          {chart.todayX !== null && (
            <line
              className={styles.today}
              x1={chart.todayX}
              x2={chart.todayX}
              y1={0}
              y2={PLOT.height + TICK_LENGTH}
            />
          )}
          {chart.actual !== null && <path className={styles.actual} d={chart.actual} />}
          {chart.marks.map((mark) => (
            <path key={mark.change.date} className={styles.mark} d={triangleAt(mark)}>
              <title>{`${formatDate(mark.change.date)}: ${mark.words}`}</title>
            </path>
          ))}
          {chart.end && (
            <rect
              className={styles.end}
              x={chart.end.x - 2}
              y={chart.end.y - 2}
              width={4}
              height={4}
            />
          )}
        </g>
      </svg>
      <figcaption className={styles.caption}>
        <span className="label">Burndown</span>
        <span className={styles.left}>{left}</span>
      </figcaption>
      {chart.marks.length > 0 && (
        <ul className={styles.changes} aria-label="What changed while the sprint ran">
          {unlisted > 0 && (
            <li className={styles.change}>
              {`and ${unlisted.toString()} earlier ${unlisted === 1 ? 'day' : 'days'} of change`}
            </li>
          )}
          {listed.map((mark) => (
            <li key={mark.change.date} className={styles.change} title={namesOf(project, mark)}>
              <span className={styles.changeMark} aria-hidden>
                {mark.direction === 'up' ? '▲' : '▼'}
              </span>
              <span className={styles.changeDay}>{formatDate(mark.change.date)}</span>
              <span>{mark.words}</span>
            </li>
          ))}
        </ul>
      )}
    </figure>
  );
}

/** A small triangle above the line on the day of a change, pointing the way the work went. */
function triangleAt(mark: ScopeMark): string {
  const { x, y } = mark;
  const top = y - MARK * 3;
  return mark.direction === 'up'
    ? `M${(x - MARK).toString()} ${(top + MARK * 2).toString()}L${x.toString()} ${top.toString()}L${(x + MARK).toString()} ${(top + MARK * 2).toString()}Z`
    : `M${(x - MARK).toString()} ${top.toString()}L${x.toString()} ${(top + MARK * 2).toString()}L${(x + MARK).toString()} ${top.toString()}Z`;
}

/** The tasks of a change, by reference and title, for the hover of its line. */
function namesOf(project: Pick<Project, 'key'>, mark: ScopeMark): string {
  const line = (verb: string, tasks: readonly ScopeTask[]) =>
    tasks.map((task) => `${verb} ${taskReference(project, task)} ${task.title}`);
  return [...line('Joined:', mark.change.joined), ...line('Left:', mark.change.left)].join('\n');
}
