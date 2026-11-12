import {
  useId,
  useLayoutEffect,
  useMemo,
  useRef,
  useState,
  type CSSProperties,
  type KeyboardEvent,
  type PointerEvent,
} from 'react';

import { useNavigation } from '@/app/navigation';
import type { Id, Project, Status, Task } from '@/shared/api/types';
import { classNames } from '@/shared/lib/classNames';
import { dayNumberOf, isWeekend, partsOfDay, type DayNumber } from '@/shared/lib/days';
import { formatMonth, formatPeriod, isOverdue, taskReference, today } from '@/shared/lib/format';
import { Button } from '@/shared/ui/Button';

import { SubtaskFold } from '../tasks/SubtaskFold';
import { StatusMark } from '../tasks/TaskBadges';
import { useSubtaskFolds } from '../tasks/subtaskFolds';
import { statusOf } from '../tasks/taskModel';
import { spanOf, type Span } from '../tasks/taskSpan';
import { useTaskCommands } from '../tasks/taskQueries';
import { AssigneeAvatar } from '../team/Assignee';
import styles from './GanttView.module.css';
import {
  BAR_HEIGHT,
  DAY_WIDTH,
  ROW_HEIGHT,
  barOf,
  chartRange,
  chartRows,
  draggedDates,
  linksOf,
  monthsOf,
  type ChartRange,
  type ChartRow,
  type Dates,
  type Grip,
} from './ganttModel';

/** How many days are shown to the left of today when the chart opens. */
const DAYS_LEFT_OF_TODAY = 4;

/** How far the pointer must move before a press on a bar becomes a drag, in pixels. */
const DRAG_START_DISTANCE = 4;

/**
 * The dates a task would get from the drag that is under way.
 *
 * It belongs to the tasks it was made from. As soon as they are replaced,
 * by the result of the drag or by anything else, it no longer counts. That
 * way the bar goes from the place it was dropped straight to the place the
 * new dates give it, without showing the old place in between.
 */
interface Preview {
  taskId: Id;
  dates: Dates;
  tasks: Task[];
}

interface GanttViewProps {
  project: Project;
  tasks: Task[];
}

/**
 * The plan of a project against time: one row for every task, with a bar
 * from its start to its due date, and a line wherever one task waits for
 * another. A bar can be dragged to move the task, and pulled at an end to
 * make it longer or shorter.
 */
export function GanttView({ project, tasks }: GanttViewProps) {
  const navigation = useNavigation();
  const commands = useTaskCommands(project.id);
  const folds = useSubtaskFolds(project.id);
  const arrowId = useId();
  const scroller = useRef<HTMLDivElement>(null);
  const shownFirstDay = useRef<DayNumber | null>(null);
  const [preview, setPreview] = useState<Preview | null>(null);
  const todayDay = dayNumberOf(today());

  const range = useMemo(
    () =>
      chartRange(
        tasks.flatMap((task) => spanOf(task) ?? []),
        todayDay,
      ),
    [tasks, todayDay],
  );
  const storedRows = useMemo(() => chartRows(tasks, folds.isOpen), [tasks, folds]);
  const dragged = preview?.tasks === tasks ? preview : null;
  // While a bar is dragged its row stays where it is, and only its days
  // change, so that the lines of its dependencies follow it.
  const rows = useMemo(
    () =>
      dragged
        ? storedRows.map((row) =>
            row.task.id === dragged.taskId ? { ...row, span: spanOf(dragged.dates) } : row,
          )
        : storedRows,
    [storedRows, dragged],
  );
  const links = useMemo(() => linksOf(rows, range), [rows, range]);
  const months = useMemo(() => monthsOf(range), [range]);
  const chartWidth = range.dayCount * DAY_WIDTH;
  const todayLeft = (todayDay - range.firstDay) * DAY_WIDTH;

  // The chart opens with today near its left edge. When a task gets a date
  // before the first day of the chart, the chart grows to the left: the
  // scroll position grows with it, so the days in view stay where they are.
  useLayoutEffect(() => {
    const element = scroller.current;
    if (!element) {
      return;
    }
    if (shownFirstDay.current === null) {
      element.scrollLeft = Math.max(todayLeft - DAYS_LEFT_OF_TODAY * DAY_WIDTH, 0);
    } else {
      element.scrollLeft += (shownFirstDay.current - range.firstDay) * DAY_WIDTH;
    }
    shownFirstDay.current = range.firstDay;
  }, [range.firstDay, todayLeft]);

  function showToday() {
    scroller.current?.scrollTo({
      left: Math.max(todayLeft - DAYS_LEFT_OF_TODAY * DAY_WIDTH, 0),
      behavior: 'smooth',
    });
  }

  if (tasks.length === 0) {
    return (
      <p className={styles.note}>
        No tasks yet. A task with a start or a due date appears here as a bar.
      </p>
    );
  }

  return (
    <div
      ref={scroller}
      className={styles.gantt}
      style={
        {
          '--day': `${DAY_WIDTH.toString()}px`,
          '--row': `${ROW_HEIGHT.toString()}px`,
          '--bar': `${BAR_HEIGHT.toString()}px`,
        } as CSSProperties
      }
    >
      <div className={styles.sheet}>
        <div className={styles.head}>
          <div className={styles.corner}>
            <span className="label">Task</span>
            <Button variant="ghost" className={styles.todayButton} onClick={showToday}>
              Today
            </Button>
          </div>

          <div className={styles.scale} style={{ width: chartWidth }}>
            <div className={styles.months}>
              {months.map(({ year, month, dayCount }) => (
                <div
                  key={`${year.toString()}-${month.toString()}`}
                  className={styles.month}
                  style={{ width: dayCount * DAY_WIDTH }}
                >
                  <span className={classNames('label', styles.monthName)}>
                    {formatMonth(year, month)}
                  </span>
                </div>
              ))}
            </div>
            <div className={styles.days} aria-hidden>
              {Array.from({ length: range.dayCount }, (_, offset) => {
                const day = range.firstDay + offset;
                return (
                  <span
                    key={day}
                    className={classNames(
                      styles.day,
                      isWeekend(day) && styles.restDay,
                      day === todayDay && styles.today,
                    )}
                  >
                    {partsOfDay(day).dayOfMonth}
                  </span>
                );
              })}
            </div>
          </div>
        </div>

        <div className={styles.body}>
          {/* What lies behind the bars: the weeks, the months and today. */}
          <div className={styles.backdrop} style={{ width: chartWidth }} aria-hidden>
            {months.slice(1).map(({ year, month, firstDay }) => (
              <span
                key={`${year.toString()}-${month.toString()}`}
                className={styles.monthLine}
                style={{ left: (firstDay - range.firstDay) * DAY_WIDTH }}
              />
            ))}
            <span className={styles.todayLine} style={{ left: todayLeft + DAY_WIDTH / 2 }} />
          </div>

          {rows.map((row) => (
            <Row
              key={row.task.id}
              project={project}
              row={row}
              dates={dragged?.taskId === row.task.id ? dragged.dates : row.task}
              range={range}
              todayDay={todayDay}
              isOpenInPanel={navigation.taskId === row.task.id}
              onOpen={() => {
                navigation.openTask(row.task.id);
              }}
              onToggle={() => {
                folds.toggle(row.task.id);
              }}
              onPreview={(dates) => {
                setPreview(dates && { taskId: row.task.id, dates, tasks });
              }}
              onReschedule={({ startOn, dueOn }) => {
                void commands.reschedule(row.task.id, startOn, dueOn);
              }}
            />
          ))}

          {/* The rest of the sheet, so that the outline and the weeks run to its end. */}
          <div className={styles.rest}>
            <div className={styles.restOfOutline} />
          </div>

          <svg
            className={styles.links}
            width={chartWidth}
            height={rows.length * ROW_HEIGHT}
            aria-hidden
          >
            <defs>
              <marker
                id={arrowId}
                viewBox="0 0 6 6"
                refX="5.5"
                refY="3"
                markerWidth="6"
                markerHeight="6"
                orient="auto"
              >
                <path d="M0 0.5 5.5 3 0 5.5Z" fill="context-stroke" />
              </marker>
            </defs>
            {links.map((link) => (
              <path
                key={`${link.blockerId}>${link.waitingId}`}
                d={link.path}
                className={classNames(styles.link, link.isBroken && styles.brokenLink)}
                markerEnd={`url(#${arrowId})`}
              />
            ))}
          </svg>
        </div>
      </div>
    </div>
  );
}

interface RowProps {
  project: Project;
  row: ChartRow;
  /** The dates the bar stands for: those of the task, or those it is being dragged to. */
  dates: Dates;
  range: ChartRange;
  todayDay: DayNumber;
  /** Whether the panel at the side shows this task. */
  isOpenInPanel: boolean;
  onOpen: () => void;
  onToggle: () => void;
  onPreview: (dates: Dates | null) => void;
  onReschedule: (dates: Dates) => void;
}

/** One task of the chart: its name in the outline, and its bar against the days. */
function Row({
  project,
  row,
  dates,
  range,
  todayDay,
  isOpenInPanel,
  onOpen,
  onToggle,
  onPreview,
  onReschedule,
}: RowProps) {
  const { task, depth, subtasks, isOpen, span } = row;
  const status = statusOf(project, task);
  const reference = taskReference(project, task);

  return (
    <div className={classNames(styles.row, isOpenInPanel && styles.openRow)}>
      <div
        className={styles.name}
        style={{ '--depth': depth } as CSSProperties}
        role="button"
        tabIndex={0}
        onClick={onOpen}
        onKeyDown={(event) => {
          if (event.key === 'Enter') {
            onOpen();
          }
        }}
      >
        {status && (
          <span className={styles.mark} role="img" aria-label={status.name} title={status.name}>
            <StatusMark status={status} />
          </span>
        )}
        <span className={styles.reference}>{reference}</span>
        <span className={styles.title}>{task.title}</span>
        {subtasks.length > 0 && (
          <SubtaskFold
            isCompact
            className={styles.fold}
            project={project}
            subtasks={subtasks}
            isOpen={isOpen}
            onToggle={onToggle}
          />
        )}
        <span className={styles.assignee}>
          <AssigneeAvatar assignee={task.assignee} />
        </span>
      </div>

      <div className={styles.track} style={{ width: range.dayCount * DAY_WIDTH }}>
        {span && (
          <Bar
            task={task}
            span={span}
            dates={dates}
            status={status}
            range={range}
            todayDay={todayDay}
            label={`${reference} ${task.title}`}
            onOpen={onOpen}
            onPreview={onPreview}
            onReschedule={onReschedule}
          />
        )}
      </div>
    </div>
  );
}

/** A bar while it is held: where it is held, and how far it has come. */
interface Drag {
  grip: Grip;
  /** Where the pointer was when the bar was taken, in pixels from the left of the window. */
  startedAt: number;
  /** How many days the bar has been dragged. Negative to the left. */
  days: number;
  /** Whether the pointer has moved far enough for this to be a drag and not a press. */
  hasMoved: boolean;
}

interface BarProps {
  task: Task;
  /** The days the bar is drawn on. While it is dragged, these are the days it would get. */
  span: Span;
  /** The dates that go with those days. */
  dates: Dates;
  status: Status | undefined;
  range: ChartRange;
  todayDay: DayNumber;
  /** What the bar stands for, for people who cannot see it. */
  label: string;
  onOpen: () => void;
  /** Called with the dates the task would get while the bar is dragged, and with nothing when that is over. */
  onPreview: (dates: Dates | null) => void;
  onReschedule: (dates: Dates) => void;
}

/**
 * The bar of a task. A press opens the task. A drag moves it, and a drag at
 * one of its ends changes only that date. With the keyboard, the arrow keys
 * move it by a day, with Shift they change the due date, and Enter opens the
 * task.
 */
function Bar({
  task,
  span,
  dates,
  status,
  range,
  todayDay,
  label,
  onOpen,
  onPreview,
  onReschedule,
}: BarProps) {
  const [drag, setDrag] = useState<Drag | null>(null);
  const bar = barOf(span, range);
  const period = formatPeriod(dates.startOn, dates.dueOn);
  const isLate = drag === null && isOverdue(task) && span.lastDay < todayDay;

  function hold(grip: Grip) {
    return (event: PointerEvent<HTMLElement>) => {
      if (event.button !== 0) {
        return;
      }
      // An end is part of the bar. A press on it must not also take the bar.
      event.stopPropagation();
      // The pointer keeps belonging to the bar when it leaves it.
      event.currentTarget.setPointerCapture(event.pointerId);
      setDrag({ grip, startedAt: event.clientX, days: 0, hasMoved: false });
    };
  }

  function follow(event: PointerEvent<HTMLElement>) {
    if (!drag) {
      return;
    }
    const distance = event.clientX - drag.startedAt;
    const days = Math.round(distance / DAY_WIDTH);
    const hasMoved = drag.hasMoved || Math.abs(distance) >= DRAG_START_DISTANCE;
    if (days !== drag.days) {
      onPreview(draggedDates(task, drag.grip, days));
    }
    if (days !== drag.days || hasMoved !== drag.hasMoved) {
      setDrag({ ...drag, days, hasMoved });
    }
  }

  function release() {
    if (!drag) {
      return;
    }
    setDrag(null);
    if (!drag.hasMoved) {
      onPreview(null);
      onOpen();
    } else if (drag.days === 0) {
      onPreview(null);
    } else {
      // The preview stays until the new dates have arrived.
      onReschedule(draggedDates(task, drag.grip, drag.days));
    }
  }

  function giveUp() {
    setDrag(null);
    onPreview(null);
  }

  function handleKeyDown(event: KeyboardEvent<HTMLElement>) {
    if (event.key === 'Escape' && drag) {
      // Escape puts the bar back. It must not also close the panel.
      event.stopPropagation();
      giveUp();
    } else if (event.key === 'Enter') {
      onOpen();
    } else if (event.key === 'ArrowLeft' || event.key === 'ArrowRight') {
      event.preventDefault();
      const days = event.key === 'ArrowLeft' ? -1 : 1;
      onReschedule(draggedDates(task, event.shiftKey ? 'end' : 'whole', days));
    }
  }

  return (
    <>
      <div
        className={classNames(styles.bar, drag?.hasMoved && styles.dragged)}
        style={{ left: bar.left, width: bar.width }}
        data-tone={status?.color}
        data-kind={span.kind}
        data-progress={status?.category}
        role="button"
        tabIndex={0}
        aria-label={`${label}, ${period}`}
        title={drag ? undefined : period}
        onPointerDown={hold('whole')}
        onPointerMove={follow}
        onPointerUp={release}
        onPointerCancel={giveUp}
        onKeyDown={handleKeyDown}
      >
        {span.kind === 'planned' && (
          <span className={styles.startGrip} onPointerDown={hold('start')} />
        )}
        {span.kind !== 'deadline' && (
          <span className={styles.endGrip} onPointerDown={hold('end')} />
        )}
      </div>

      {drag?.hasMoved && (
        // The dates of the bar while it is dragged, set over it like a dimension.
        <span className={styles.dragDates} style={{ left: bar.left }}>
          {period}
        </span>
      )}

      {isLate && (
        // From the end of the bar to the line of today: how late the task is.
        <span
          className={styles.lateness}
          style={{
            left: bar.left + bar.width,
            width: (todayDay - range.firstDay + 0.5) * DAY_WIDTH - (bar.left + bar.width),
          }}
          title="Overdue"
        />
      )}
    </>
  );
}
