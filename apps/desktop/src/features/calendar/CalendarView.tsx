import { ChevronLeft, ChevronRight } from 'lucide-react';
import { Popover } from 'radix-ui';
import { useMemo, useState, type CSSProperties } from 'react';

import { useNavigation } from '@/app/navigation';
import type { Id, Project, Task } from '@/shared/api/types';
import { classNames } from '@/shared/lib/classNames';
import {
  calendarDateOfDay,
  dayNumberOf,
  isWeekend,
  partsOfDay,
  type DayNumber,
} from '@/shared/lib/days';
import {
  formatDay,
  formatMonth,
  formatMonthShort,
  formatPeriod,
  isOverdue,
  taskReference,
  today,
} from '@/shared/lib/format';
import { Button, IconButton } from '@/shared/ui/Button';

import { StatusMark } from '../tasks/TaskBadges';
import { statusOf } from '../tasks/taskModel';
import { spanOf } from '../tasks/taskSpan';
import styles from './CalendarView.module.css';
import {
  addMonths,
  lanesUsed,
  layoutWeek,
  monthOf,
  tasksOn,
  weeksOfMonth,
  type Month,
  type Piece,
} from './calendarModel';

const WEEKDAYS = ['Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat', 'Sun'];

/**
 * A week grows with what is in it: it has as many lines of tasks as it
 * needs, between these two numbers. A week with little in it keeps some
 * height, and a day with more tasks than the most lines gets a note that
 * opens the whole list.
 */
const FEWEST_LANES = 2;
const MOST_LANES = 6;

interface CalendarViewProps {
  project: Project;
  tasks: Task[];
}

/**
 * The tasks of a project on a calendar, one month at a time. A task lies on
 * the days from its start to its due date, and a task with one date on that
 * day. Tasks without any date are not on the calendar.
 */
export function CalendarView({ project, tasks }: CalendarViewProps) {
  const navigation = useNavigation();
  const todayDay = dayNumberOf(today());
  const [month, setMonth] = useState<Month>(() => monthOf(todayDay));

  const weeks = useMemo(() => weeksOfMonth(month), [month]);
  const layouts = useMemo(
    () => weeks.map((days) => layoutWeek(days, tasks, MOST_LANES)),
    [weeks, tasks],
  );
  const undatedCount = useMemo(
    () => tasks.filter((task) => spanOf(task) === undefined).length,
    [tasks],
  );

  return (
    <div className={styles.calendar}>
      <header className={styles.bar}>
        <h2 className={styles.month}>{formatMonth(month.year, month.month)}</h2>
        <IconButton
          label="Previous month"
          onClick={() => {
            setMonth(addMonths(month, -1));
          }}
        >
          <ChevronLeft />
        </IconButton>
        <IconButton
          label="Next month"
          onClick={() => {
            setMonth(addMonths(month, 1));
          }}
        >
          <ChevronRight />
        </IconButton>
        <Button
          variant="ghost"
          onClick={() => {
            setMonth(monthOf(todayDay));
          }}
        >
          Today
        </Button>
        <span className={styles.space} />
        {undatedCount > 0 && (
          <span className={styles.undated}>
            {undatedCount} {undatedCount === 1 ? 'task' : 'tasks'} without a date
          </span>
        )}
      </header>

      <div className={styles.weekdays} aria-hidden>
        {WEEKDAYS.map((weekday) => (
          <span key={weekday} className="label">
            {weekday}
          </span>
        ))}
      </div>

      <div className={styles.weeks}>
        {weeks.map((days, week) => {
          const layout = layouts[week];
          return (
            <div
              key={days[0]}
              className={styles.week}
              style={{ '--lanes': lanesUsed(layout?.pieces ?? [], FEWEST_LANES) } as CSSProperties}
            >
              {days.map((day, column) => (
                <DayCell
                  key={day}
                  day={day}
                  column={column}
                  isInMonth={monthOf(day).month === month.month}
                  isToday={day === todayDay}
                />
              ))}

              {layout?.pieces.map((piece) => (
                <TaskPiece
                  key={piece.task.id}
                  project={project}
                  piece={piece}
                  onOpen={navigation.openTask}
                />
              ))}

              {layout?.crowdedOut.map(
                (hidden, column) =>
                  hidden.length > 0 && (
                    <MoreTasks
                      key={days[column]}
                      project={project}
                      day={days[column] ?? 0}
                      column={column}
                      hiddenCount={hidden.length}
                      tasks={tasks}
                      onOpen={navigation.openTask}
                    />
                  ),
              )}
            </div>
          );
        })}
      </div>
    </div>
  );
}

interface DayCellProps {
  day: DayNumber;
  /** The column of the week the day is in: `0` for Monday. */
  column: number;
  isInMonth: boolean;
  isToday: boolean;
}

/** One day of the calendar: its number, on the ground the tasks lie on. */
function DayCell({ day, column, isInMonth, isToday }: DayCellProps) {
  const { year, month, dayOfMonth } = partsOfDay(day);
  return (
    <div
      className={classNames(
        styles.day,
        !isInMonth && styles.otherMonth,
        isWeekend(day) && styles.restDay,
        isToday && styles.today,
      )}
      style={{ gridColumn: column + 1 }}
    >
      <span className={styles.dayNumber}>
        {/* The first of a month says which month begins. */}
        {dayOfMonth === 1
          ? `${dayOfMonth.toString()} ${formatMonthShort(year, month)}`
          : dayOfMonth}
      </span>
    </div>
  );
}

interface TaskPieceProps {
  project: Project;
  piece: Piece;
  onOpen: (taskId: Id) => void;
}

/** A task on the calendar, or the part of it that lies in one week. */
function TaskPiece({ project, piece, onOpen }: TaskPieceProps) {
  const { task, firstColumn, columnCount, lane, beginsHere, endsHere } = piece;
  const status = statusOf(project, task);

  return (
    <button
      type="button"
      className={classNames(
        styles.piece,
        !beginsHere && styles.comesOver,
        !endsHere && styles.goesOn,
        endsHere && isOverdue(task) && styles.late,
      )}
      style={{
        gridColumn: `${(firstColumn + 1).toString()} / span ${columnCount.toString()}`,
        gridRow: lane + 2,
      }}
      data-tone={status?.color}
      data-progress={status?.category}
      title={`${taskReference(project, task)} · ${task.title} · ${formatPeriod(task.startOn, task.dueOn)}`}
      onClick={() => {
        onOpen(task.id);
      }}
    >
      {status && <StatusMark status={status} />}
      <span className={styles.pieceTitle}>{task.title}</span>
    </button>
  );
}

interface MoreTasksProps {
  project: Project;
  day: DayNumber;
  column: number;
  /** How many tasks of the day found no line. */
  hiddenCount: number;
  /** Every task of the project. The list shows all tasks of the day, not only the hidden ones. */
  tasks: Task[];
  onOpen: (taskId: Id) => void;
}

/** The note under a crowded day. It opens the list of all tasks of that day. */
function MoreTasks({ project, day, column, hiddenCount, tasks, onOpen }: MoreTasksProps) {
  const [isOpen, setIsOpen] = useState(false);

  return (
    <Popover.Root open={isOpen} onOpenChange={setIsOpen}>
      <Popover.Trigger asChild>
        <button type="button" className={styles.more} style={{ gridColumn: column + 1 }}>
          +{hiddenCount} more
        </button>
      </Popover.Trigger>
      <Popover.Portal>
        <Popover.Content className={styles.dayList} align="start" sideOffset={4}>
          <h3 className="label">{formatDay(calendarDateOfDay(day))}</h3>
          <ul className={styles.dayTasks}>
            {tasksOn(day, tasks).map((task) => {
              const status = statusOf(project, task);
              return (
                <li key={task.id}>
                  <button
                    type="button"
                    className={styles.dayTask}
                    title={status?.name}
                    onClick={() => {
                      setIsOpen(false);
                      onOpen(task.id);
                    }}
                  >
                    {status && <StatusMark status={status} />}
                    <span className={styles.dayTaskReference}>{taskReference(project, task)}</span>
                    <span className={styles.dayTaskTitle}>{task.title}</span>
                  </button>
                </li>
              );
            })}
          </ul>
        </Popover.Content>
      </Popover.Portal>
    </Popover.Root>
  );
}
