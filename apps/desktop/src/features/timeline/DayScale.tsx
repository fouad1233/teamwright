import type { ReactNode } from 'react';

import { classNames } from '@/shared/lib/classNames';
import { isWeekend, partsOfDay, type DayNumber } from '@/shared/lib/days';
import { formatMonth } from '@/shared/lib/format';

import ganttStyles from '../gantt/GanttView.module.css';
import { DAY_WIDTH, type ChartRange, type MonthOnChart } from '../gantt/ganttModel';

/*
 * The scale of days is the Gantt chart's: the same arithmetic
 * (`ganttModel.ts`) and the same style sheet, so that a day, a week, a month
 * and today look the same in both. Its classes read `--outline`, `--head`
 * and `--day`, which the sheet around them sets.
 */

interface DayScaleProps {
  range: ChartRange;
  months: readonly MonthOnChart[];
  todayDay: DayNumber;
  /** What stands in the corner above the column of names. */
  corner: ReactNode;
}

/** The head of a sheet of days: the months over the days, under a corner of controls. */
export function DayScale({ range, months, todayDay, corner }: DayScaleProps) {
  return (
    <div className={ganttStyles.head}>
      <div className={ganttStyles.corner}>{corner}</div>

      <div className={ganttStyles.scale} style={{ width: range.dayCount * DAY_WIDTH }}>
        <div className={ganttStyles.months}>
          {months.map(({ year, month, dayCount }) => (
            <div
              key={`${year.toString()}-${month.toString()}`}
              className={ganttStyles.month}
              style={{ width: dayCount * DAY_WIDTH }}
            >
              <span className={classNames('label', ganttStyles.monthName)}>
                {formatMonth(year, month)}
              </span>
            </div>
          ))}
        </div>
        <div className={ganttStyles.days} aria-hidden>
          {Array.from({ length: range.dayCount }, (_, offset) => {
            const day = range.firstDay + offset;
            return (
              <span
                key={day}
                className={classNames(
                  ganttStyles.day,
                  isWeekend(day) && ganttStyles.restDay,
                  day === todayDay && ganttStyles.today,
                )}
              >
                {partsOfDay(day).dayOfMonth}
              </span>
            );
          })}
        </div>
      </div>
    </div>
  );
}

interface DayBackdropProps {
  range: ChartRange;
  months: readonly MonthOnChart[];
  todayDay: DayNumber;
}

/** What lies behind the bars: the weeks, the months and the line of today. */
export function DayBackdrop({ range, months, todayDay }: DayBackdropProps) {
  return (
    <div className={ganttStyles.backdrop} style={{ width: range.dayCount * DAY_WIDTH }} aria-hidden>
      {months.slice(1).map(({ year, month, firstDay }) => (
        <span
          key={`${year.toString()}-${month.toString()}`}
          className={ganttStyles.monthLine}
          style={{ left: (firstDay - range.firstDay) * DAY_WIDTH }}
        />
      ))}
      <span
        className={ganttStyles.todayLine}
        style={{ left: (todayDay - range.firstDay) * DAY_WIDTH + DAY_WIDTH / 2 }}
      />
    </div>
  );
}

/** The class of the button that brings today into view, set on the line of the corner. */
export const TODAY_BUTTON_CLASS = ganttStyles.todayButton;
