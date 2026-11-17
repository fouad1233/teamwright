import { ChevronsUpDown } from 'lucide-react';
import {
  useLayoutEffect,
  useMemo,
  useRef,
  useState,
  type CSSProperties,
  type KeyboardEvent,
  type PointerEvent,
} from 'react';

import { useNavigation } from '@/app/navigation';
import type { Area, Id, Project, Status, Task } from '@/shared/api/types';
import { classNames } from '@/shared/lib/classNames';
import { dayNumberOf, type DayNumber } from '@/shared/lib/days';
import { formatPeriod, isOverdue, taskReference, today } from '@/shared/lib/format';
import { Button, ValueButton } from '@/shared/ui/Button';
import { ChoiceMenu, type Choice } from '@/shared/ui/Menu';

import { useAreas } from '../areas/areaQueries';
import {
  DAY_WIDTH,
  barOf,
  chartRange,
  draggedDates,
  monthsOf,
  type ChartRange,
  type Dates,
  type Grip,
} from '../gantt/ganttModel';
import { StatusMark } from '../tasks/TaskBadges';
import { statusOf } from '../tasks/taskModel';
import { spanOf, type Span } from '../tasks/taskSpan';
import { useTaskCommands } from '../tasks/taskQueries';
import {
  AssigneeAvatar,
  MANAGER_NAME,
  ManagerAvatar,
  MemberAvatar,
  NOBODY_NAME,
} from '../team/Assignee';
import { useTeamMembers } from '../team/TeamContext';
import type { KindProps } from '../views/viewKinds';
import { DayBackdrop, DayScale, TODAY_BUTTON_CLASS } from './DayScale';
import styles from './TimelineView.module.css';
import {
  BAR_HEIGHT,
  LANE_HEIGHT,
  POINT_DAYS,
  ROW_PADDING,
  UNDATED_SHOWN,
  barTopOf,
  timelineRows,
  trackHeightOf,
  type Grouping,
  type PlacedTask,
  type RowSubject,
  type TimelineRow,
} from './timelineModel';

/** How many days are shown to the left of today when the timeline opens. */
const DAYS_LEFT_OF_TODAY = 4;

/** How far the pointer must move before a press on a bar becomes a drag, in pixels. */
const DRAG_START_DISTANCE = 4;

/**
 * How wide a bar must be for its title to begin in it, in pixels. A
 * narrower bar shows only the mark of its status: a few letters cut off
 * say less than the title the pointer shows.
 */
const TITLE_FITS_FROM = 60;

/** The areas while they are loading. */
const NO_AREAS: readonly Area[] = [];

const GROUPINGS: readonly Choice<Grouping>[] = [
  { value: 'person', label: 'By person' },
  { value: 'status', label: 'By status' },
  { value: 'area', label: 'By area' },
];

/**
 * The dates a task would get from the drag that is under way. As in the
 * Gantt chart, it belongs to the tasks it was made from, and stops counting
 * as soon as they are replaced.
 */
interface Preview {
  taskId: Id;
  dates: Dates;
  tasks: Task[];
}

/**
 * Who works on what, along the days: a row for every person, with their
 * tasks as bars on the scale of the Gantt chart. Bars that overlap in time
 * lie on lines of their own within the row. A task without dates is listed
 * under the name of its row. The rows can also be the statuses or the areas.
 */
export function TimelineView({ project, tasks }: KindProps) {
  const navigation = useNavigation();
  const commands = useTaskCommands(project.id);
  const members = useTeamMembers();
  const areas = useAreas(project.id).data ?? NO_AREAS;
  const scroller = useRef<HTMLDivElement>(null);
  const shownFirstDay = useRef<DayNumber | null>(null);
  const [grouping, setGrouping] = useState<Grouping>('person');
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
  const months = useMemo(() => monthsOf(range), [range]);
  const rows = useMemo(
    () => timelineRows(tasks, grouping, { project, members, areas }),
    [tasks, grouping, project, members, areas],
  );
  const dragged = preview?.tasks === tasks ? preview : null;
  const todayLeft = (todayDay - range.firstDay) * DAY_WIDTH;

  // The timeline opens with today near its left edge, and keeps the days in
  // view where they are when it grows to the left, as the Gantt chart does.
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
      <div className={classNames(styles.empty, 'pattern-band')}>
        <p className={styles.emptyLead}>Who works on what, along the days.</p>
        <p className={styles.emptyNote}>
          Each person gets a row, and their tasks lie on it as bars from start to due date. Give a
          task to someone and give it dates, and it appears here.
        </p>
      </div>
    );
  }

  return (
    <div
      ref={scroller}
      className={styles.timeline}
      style={
        {
          '--day': `${DAY_WIDTH.toString()}px`,
          '--lane': `${LANE_HEIGHT.toString()}px`,
          '--bar': `${BAR_HEIGHT.toString()}px`,
          '--row-padding': `${ROW_PADDING.toString()}px`,
        } as CSSProperties
      }
    >
      <div className={styles.sheet}>
        <DayScale
          range={range}
          months={months}
          todayDay={todayDay}
          corner={
            <>
              <ChoiceMenu label="Rows" choices={GROUPINGS} value={grouping} onChoose={setGrouping}>
                <ValueButton className={styles.grouping} aria-label="Rows">
                  {GROUPINGS.find((choice) => choice.value === grouping)?.label}
                  <ChevronsUpDown aria-hidden />
                </ValueButton>
              </ChoiceMenu>
              <Button variant="ghost" className={TODAY_BUTTON_CLASS} onClick={showToday}>
                Today
              </Button>
            </>
          }
        />

        <div className={styles.body}>
          <DayBackdrop range={range} months={months} todayDay={todayDay} />

          {rows.map((row) => (
            <Row
              key={row.key}
              project={project}
              row={row}
              range={range}
              todayDay={todayDay}
              dragged={dragged}
              openTaskId={navigation.taskId}
              onOpen={(taskId) => {
                navigation.openTask(taskId);
              }}
              onPreview={(taskId, dates) => {
                setPreview(dates && { taskId, dates, tasks });
              }}
              onReschedule={(taskId, { startOn, dueOn }) => {
                void commands.reschedule(taskId, startOn, dueOn);
              }}
            />
          ))}

          {/* The rest of the sheet, so that the column of names and the weeks run to its end. */}
          <div className={styles.rest}>
            <div className={styles.restOfNames} />
          </div>
        </div>
      </div>
    </div>
  );
}

interface RowProps {
  project: Project;
  row: TimelineRow;
  range: ChartRange;
  todayDay: DayNumber;
  dragged: Preview | null;
  /** The task the panel at the side shows. */
  openTaskId: Id | null;
  onOpen: (taskId: Id) => void;
  onPreview: (taskId: Id, dates: Dates | null) => void;
  onReschedule: (taskId: Id, dates: Dates) => void;
}

/** One row: who or what it is for, its tasks without dates, and its bars against the days. */
function Row({
  project,
  row,
  range,
  todayDay,
  dragged,
  openTaskId,
  onOpen,
  onPreview,
  onReschedule,
}: RowProps) {
  const [isUnfolded, setIsUnfolded] = useState(false);
  const count = row.placed.length + row.undated.length;
  // A list one longer than the short one is shown whole: "1 more" would take
  // the same line as the task it hides.
  const canFold = row.undated.length > UNDATED_SHOWN + 1;
  const undatedShown = canFold && !isUnfolded ? row.undated.slice(0, UNDATED_SHOWN) : row.undated;

  return (
    <div className={styles.row}>
      <div className={styles.names}>
        <div className={styles.subject}>
          <SubjectHead subject={row.subject} />
          <span className={styles.count} title={`${count.toString()} ${tasksWord(count)}`}>
            {count}
          </span>
        </div>
        <SubjectLine subject={row.subject} />

        {row.undated.length > 0 && (
          <div className={styles.undated}>
            <span className={classNames('label', styles.undatedLabel)}>No dates</span>
            <ul className={styles.undatedList}>
              {undatedShown.map((task) => (
                <li key={task.id}>
                  <UndatedTask
                    project={project}
                    task={task}
                    isOpenInPanel={openTaskId === task.id}
                    onOpen={() => {
                      onOpen(task.id);
                    }}
                  />
                </li>
              ))}
            </ul>
            {canFold && (
              <button
                type="button"
                className={styles.more}
                aria-expanded={isUnfolded}
                onClick={() => {
                  setIsUnfolded(!isUnfolded);
                }}
              >
                {isUnfolded ? 'Fewer' : `${(row.undated.length - UNDATED_SHOWN).toString()} more`}
              </button>
            )}
          </div>
        )}
      </div>

      <div
        className={styles.track}
        style={{ width: range.dayCount * DAY_WIDTH, minHeight: trackHeightOf(row.laneCount) }}
      >
        {row.placed.map((placed) => {
          const isDragged = dragged?.taskId === placed.task.id;
          return (
            <Bar
              key={placed.task.id}
              project={project}
              placed={placed}
              span={isDragged ? (spanOf(dragged.dates) ?? placed.span) : placed.span}
              dates={isDragged ? dragged.dates : placed.task}
              range={range}
              todayDay={todayDay}
              isOpenInPanel={openTaskId === placed.task.id}
              onOpen={() => {
                onOpen(placed.task.id);
              }}
              onPreview={(dates) => {
                onPreview(placed.task.id, dates);
              }}
              onReschedule={(dates) => {
                onReschedule(placed.task.id, dates);
              }}
            />
          );
        })}
      </div>
    </div>
  );
}

function tasksWord(count: number): string {
  return count === 1 ? 'task' : 'tasks';
}

/** The name of a row, with the picture of the person or the mark of the status. */
function SubjectHead({ subject }: { subject: RowSubject }) {
  switch (subject.kind) {
    case 'manager':
      return (
        <>
          <ManagerAvatar />
          <span className={styles.subjectName}>{MANAGER_NAME}</span>
        </>
      );
    case 'member':
      return (
        <>
          <MemberAvatar member={subject.member} />
          <span className={styles.subjectName}>{subject.member.name}</span>
        </>
      );
    case 'nobody':
      return (
        <>
          <AssigneeAvatar assignee={null} />
          <span className={classNames(styles.subjectName, styles.quiet)}>{NOBODY_NAME}</span>
        </>
      );
    case 'status':
      return (
        <>
          <span className={styles.statusMark}>
            <StatusMark status={subject.status} />
          </span>
          <span className={styles.subjectName}>{subject.status.name}</span>
        </>
      );
    case 'area':
      return <span className={styles.subjectName}>{subject.area.name}</span>;
    case 'other':
      return (
        <span className={classNames(styles.subjectName, styles.quiet)}>
          {otherName(subject.grouping)}
        </span>
      );
  }
}

function otherName(grouping: Grouping): string {
  switch (grouping) {
    case 'person':
      return 'Not on the team';
    case 'status':
      return 'Another status';
    case 'area':
      return 'Another area';
  }
}

/** A quiet line under the name of a person: what they are on the team for. */
function SubjectLine({ subject }: { subject: RowSubject }) {
  const line =
    subject.kind === 'manager'
      ? 'Runs the project'
      : subject.kind === 'member'
        ? subject.member.responsibility
        : '';
  return line ? <p className={styles.subjectLine}>{line}</p> : null;
}

interface UndatedTaskProps {
  project: Project;
  task: Task;
  isOpenInPanel: boolean;
  onOpen: () => void;
}

/** A task without dates, in the short list under the name of its row. */
function UndatedTask({ project, task, isOpenInPanel, onOpen }: UndatedTaskProps) {
  const status = statusOf(project, task);
  return (
    <button
      type="button"
      className={classNames(styles.undatedTask, isOpenInPanel && styles.isOpen)}
      data-progress={status?.category}
      onClick={onOpen}
    >
      {status && <StatusMark status={status} />}
      <span className={styles.reference}>{taskReference(project, task)}</span>
      <span className={styles.undatedTitle}>{task.title}</span>
    </button>
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
  project: Project;
  placed: PlacedTask;
  /** The days the bar is drawn on. While it is dragged, these are the days it would get. */
  span: Span;
  /** The dates that go with those days. */
  dates: Dates;
  range: ChartRange;
  todayDay: DayNumber;
  isOpenInPanel: boolean;
  onOpen: () => void;
  onPreview: (dates: Dates | null) => void;
  onReschedule: (dates: Dates) => void;
}

/**
 * The bar of a task, with the mark of its status, its key and its title. It
 * behaves as a bar of the Gantt chart: a press opens the task, a drag moves
 * it, a drag at an end changes only that date. With the keyboard, the arrow
 * keys move it by a day, with Shift they change the due date, and Enter
 * opens the task. A task with only one date is a mark on its day.
 */
function Bar({
  project,
  placed,
  span,
  dates,
  range,
  todayDay,
  isOpenInPanel,
  onOpen,
  onPreview,
  onReschedule,
}: BarProps) {
  const { task, lane } = placed;
  const [drag, setDrag] = useState<Drag | null>(null);
  const status = statusOf(project, task);
  const reference = taskReference(project, task);
  const period = formatPeriod(dates.startOn, dates.dueOn);
  const isPoint = span.kind !== 'planned';
  const box = isPoint
    ? {
        left: (span.firstDay - range.firstDay) * DAY_WIDTH,
        width: POINT_DAYS * DAY_WIDTH,
      }
    : barOf(span, range);
  const isLate = drag === null && isOverdue(task) && span.lastDay < todayDay;

  function hold(grip: Grip) {
    return (event: PointerEvent<HTMLElement>) => {
      if (event.button !== 0) {
        return;
      }
      // An end is part of the bar. A press on it must not also take the bar.
      event.stopPropagation();
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
        className={classNames(
          isPoint ? styles.point : styles.bar,
          drag?.hasMoved && styles.dragged,
          isLate && styles.late,
          isOpenInPanel && styles.isOpen,
        )}
        style={{ left: box.left, width: box.width, top: barTopOf(lane) }}
        data-tone={status?.color}
        data-kind={span.kind}
        data-progress={status?.category}
        role="button"
        tabIndex={0}
        aria-label={`${reference} ${task.title}, ${period}`}
        title={drag ? undefined : `${reference} ${task.title}\n${period}`}
        onPointerDown={hold('whole')}
        onPointerMove={follow}
        onPointerUp={release}
        onPointerCancel={giveUp}
        onKeyDown={handleKeyDown}
      >
        {isPoint ? (
          <>
            <span className={styles.pointMark} />
            <span className={styles.reference}>{reference}</span>
          </>
        ) : (
          <>
            {status && <StatusOnBar status={status} />}
            {box.width >= TITLE_FITS_FROM && <span className={styles.barTitle}>{task.title}</span>}
            <span className={styles.startGrip} onPointerDown={hold('start')} />
            <span className={styles.endGrip} onPointerDown={hold('end')} />
          </>
        )}
      </div>

      {drag?.hasMoved && (
        // The dates of the bar while it is dragged, set over it like a dimension.
        <span className={styles.dragDates} style={{ left: box.left, top: barTopOf(lane) }}>
          {period}
        </span>
      )}
    </>
  );
}

/** The mark of a status, inside a bar. */
function StatusOnBar({ status }: { status: Status }) {
  return (
    <span className={styles.barMark}>
      <StatusMark status={status} />
    </span>
  );
}
