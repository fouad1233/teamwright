import type { CalendarDate, SprintReport } from '@/shared/api/types';

import styles from './BurndownChart.module.css';
import { burndownOf } from './burndownModel';
import { formatWork } from './sprintModel';

/** The size of the plot, in pixels: small enough to stand next to the name of a sprint. */
const PLOT = { width: 208, height: 52 };

/** The room below the plot for the ticks of the days. */
const TICK_LENGTH = 4;

/** The room around the plot, so that the strokes at its edges are not cut. */
const MARGIN = 2;

interface BurndownChartProps {
  report: SprintReport;
  today: CalendarDate;
}

/**
 * The burndown of a sprint: what work remained at the end of each day, in
 * ink, against the pace its dates ask for, as a hairline. Today is marked in
 * brass. A tick under the plot stands for every day.
 */
export function BurndownChart({ report, today }: BurndownChartProps) {
  const chart = burndownOf(report, PLOT, today);
  if (!chart) {
    return null;
  }
  const width = PLOT.width + 2 * MARGIN;
  const height = PLOT.height + TICK_LENGTH + 2 * MARGIN;
  const left =
    chart.remaining === null
      ? 'Not started'
      : `${chart.remaining.toString()} of ${formatWork(report.total, report.unit)} left`;

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
    </figure>
  );
}
