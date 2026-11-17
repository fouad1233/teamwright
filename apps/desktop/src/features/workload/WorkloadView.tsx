import { ChevronsUpDown, X } from 'lucide-react';
import { useMemo, useState, type CSSProperties, type KeyboardEvent } from 'react';

import { useNavigation } from '@/app/navigation';
import type { Project, Task } from '@/shared/api/types';
import { classNames } from '@/shared/lib/classNames';
import { dayNumberOf, partsOfDay, weekdayOf } from '@/shared/lib/days';
import { formatMonthShort, formatPeriod, taskReference, today } from '@/shared/lib/format';
import { IconButton, ValueButton } from '@/shared/ui/Button';
import { TextInput } from '@/shared/ui/Field';
import { ChoiceMenu, type Choice } from '@/shared/ui/Menu';

import { StatusMark } from '../tasks/TaskBadges';
import { statusOf } from '../tasks/taskModel';
import {
  MANAGER_NAME,
  ManagerAvatar,
  MemberAvatar,
  NOBODY_NAME,
  AssigneeAvatar,
} from '../team/Assignee';
import { useTeamMembers } from '../team/TeamContext';
import type { KindProps } from '../views/viewKinds';
import { useWorkloadContent } from './useWorkloadContent';
import styles from './WorkloadView.module.css';
import {
  fillOf,
  formatHours,
  largestUnboundedCell,
  parseHoursPerDay,
  unlaidSentence,
  workloadOf,
  type Cell,
  type Column,
  type RowSubject,
  type Scale,
  type Share,
  type Unlaid,
  type WorkloadRow,
} from './workloadModel';

const SCALES: readonly Choice<Scale>[] = [
  { value: 'weeks', label: 'Eight weeks' },
  { value: 'days', label: 'Four weeks by day' },
];

/** The letters of the days of the week, from Monday. */
const WEEKDAY_LETTERS = ['M', 'T', 'W', 'T', 'F', 'S', 'S'] as const;

/** The cell whose tasks are listed under the table. */
interface Chosen {
  rowKey: string;
  columnKey: string;
}

/**
 * How much lies on each person, week by week: a row for every person, a
 * column for every week (or day), and in each cell the hours of their
 * estimates that fall into it, with a quiet bar against what they can take.
 */
export function WorkloadView({ view, project, tasks, everyTask }: KindProps) {
  const navigation = useNavigation();
  const members = useTeamMembers();
  const [content, setContent] = useWorkloadContent(view);
  const [chosen, setChosen] = useState<Chosen | null>(null);
  const [showsUnlaid, setShowsUnlaid] = useState(false);
  const todayDay = dayNumberOf(today());

  const workload = useMemo(
    () => workloadOf(tasks, { project, members, everyTask, content, today: todayDay }),
    [tasks, project, members, everyTask, content, todayDay],
  );
  const largest = largestUnboundedCell(workload.rows);
  const sentence = unlaidSentence(workload.unlaid);
  const hasSpecialists = workload.rows.some(
    (row) => row.subject.kind === 'member' && row.subject.member.kind === 'agent',
  );

  const chosenRow = chosen && workload.rows.find((row) => row.key === chosen.rowKey);
  const chosenCell =
    chosen && chosenRow?.cells.find((cell) => cell.column.key === chosen.columnKey);

  function choose(row: WorkloadRow, cell: Cell) {
    const isChosen = chosen?.rowKey === row.key && chosen.columnKey === cell.column.key;
    setChosen(isChosen ? null : { rowKey: row.key, columnKey: cell.column.key });
  }

  function setHoursPerDay(rowKey: string, hours: number) {
    setContent({ ...content, hoursPerDay: { ...content.hoursPerDay, [rowKey]: hours } });
  }

  if (tasks.length === 0) {
    return (
      <div className={classNames(styles.empty, 'pattern-band')}>
        <p className={styles.emptyLead}>How much lies on each person, week by week.</p>
        <p className={styles.emptyNote}>
          Each person gets a row, and the hours of their tasks are spread over the weeks from start
          to due date. Give a task an estimate, a due date and someone to do it, and it appears
          here.
        </p>
      </div>
    );
  }

  return (
    <div
      className={styles.workload}
      data-scale={content.scale}
      style={{ '--columns': workload.columns.length } as CSSProperties}
    >
      <div className={styles.sheet} role="table" aria-label="Hours by person">
        <div className={styles.head} role="row">
          <div className={styles.corner} role="columnheader">
            <ChoiceMenu
              label="Columns"
              choices={SCALES}
              value={content.scale}
              onChoose={(scale) => {
                setChosen(null);
                setContent({ ...content, scale });
              }}
            >
              <ValueButton className={styles.scale} aria-label="Columns">
                {SCALES.find((choice) => choice.value === content.scale)?.label}
                <ChevronsUpDown aria-hidden />
              </ValueButton>
            </ChoiceMenu>
          </div>
          {workload.columns.map((column) => (
            <ColumnHead key={column.key} column={column} />
          ))}
          <div className={classNames(styles.columnHead, styles.totalHead)} role="columnheader">
            <span className="label">Total</span>
          </div>
        </div>

        {workload.rows.map((row) => (
          <div key={row.key} className={styles.row} role="row">
            <div className={styles.names} role="rowheader">
              <div className={styles.subject}>
                <SubjectHead subject={row.subject} />
              </div>
              {row.hoursPerDay === null ? (
                <SubjectLine subject={row.subject} />
              ) : (
                <HoursPerDay
                  hours={row.hoursPerDay}
                  onChange={(hours) => {
                    setHoursPerDay(row.key, hours);
                  }}
                />
              )}
            </div>

            {row.cells.map((cell) => (
              <CellButton
                key={cell.column.key}
                cell={cell}
                largest={largest}
                isChosen={chosen?.rowKey === row.key && chosen.columnKey === cell.column.key}
                label={`${nameOf(row.subject)}, ${columnName(cell.column)}`}
                onPress={() => {
                  choose(row, cell);
                }}
              />
            ))}

            <div className={styles.total} role="cell">
              <span className={styles.figure}>{formatHours(row.totalMinutes)}</span>
              {row.totalCapacityMinutes !== null && (
                <span className={styles.capacity}>of {formatHours(row.totalCapacityMinutes)}</span>
              )}
            </div>
          </div>
        ))}
      </div>

      {chosenRow && chosenCell && (
        <section className={styles.account} aria-label="The tasks behind the cell">
          <header className={styles.accountHead}>
            <h3 className={classNames('label', styles.accountTitle)}>
              {nameOf(chosenRow.subject)}
              <span className={styles.accountSeparator} aria-hidden />
              {columnName(chosenCell.column)}
            </h3>
            <span className={styles.accountFigures}>
              {formatHours(chosenCell.minutes)}h
              {chosenCell.capacityMinutes !== null &&
                ` of ${formatHours(chosenCell.capacityMinutes)}h`}
            </span>
            <IconButton
              label="Close"
              onClick={() => {
                setChosen(null);
              }}
            >
              <X />
            </IconButton>
          </header>
          {chosenCell.shares.length === 0 ? (
            <p className={styles.accountEmpty}>Nothing lies here.</p>
          ) : (
            <ul className={styles.lines}>
              {chosenCell.shares.map((share) => (
                <li key={share.task.id}>
                  <ShareLine
                    project={project}
                    share={share}
                    onOpen={() => {
                      navigation.openTask(share.task.id);
                    }}
                  />
                </li>
              ))}
            </ul>
          )}
        </section>
      )}

      <footer className={styles.notes}>
        {hasSpecialists && (
          <p className={styles.note}>
            A specialist has no working day: its row shows what waits for it, without a limit.
          </p>
        )}
        {sentence !== null && (
          <p className={styles.note}>
            {sentence}{' '}
            <button
              type="button"
              className={styles.toggle}
              aria-expanded={showsUnlaid}
              onClick={() => {
                setShowsUnlaid(!showsUnlaid);
              }}
            >
              {showsUnlaid ? 'Hide them' : 'Show them'}
            </button>
          </p>
        )}
        {showsUnlaid && (
          <ul className={classNames(styles.lines, styles.unlaid)}>
            {workload.unlaid.map(({ task, reason }) => (
              <li key={task.id}>
                <UnlaidLine
                  project={project}
                  task={task}
                  reason={reason}
                  onOpen={() => {
                    navigation.openTask(task.id);
                  }}
                />
              </li>
            ))}
          </ul>
        )}
      </footer>
    </div>
  );
}

/** The head of a column: the month and the days of a week, or the letter and number of a day. */
function ColumnHead({ column }: { column: Column }) {
  const first = partsOfDay(column.firstDay);
  const className = classNames(
    styles.columnHead,
    column.holdsToday && styles.holdsToday,
    column.isRestDay && styles.restDay,
    column.isPast && styles.past,
    column.kind === 'day' && weekdayOf(column.firstDay) === 0 && styles.weekStart,
  );
  if (column.kind === 'week') {
    const last = partsOfDay(column.lastDay);
    return (
      <div className={className} role="columnheader" aria-label={columnName(column)}>
        <span className="label">
          {column.holdsToday ? 'This week' : formatMonthShort(first.year, first.month)}
        </span>
        <span className={styles.dates}>
          {first.dayOfMonth}–{last.dayOfMonth}
          {last.month !== first.month && ` ${formatMonthShort(last.year, last.month)}`}
        </span>
      </div>
    );
  }
  return (
    <div className={className} role="columnheader" aria-label={columnName(column)}>
      <span className="label">{WEEKDAY_LETTERS[weekdayOf(column.firstDay)]}</span>
      <span className={styles.dates}>{first.dayOfMonth}</span>
    </div>
  );
}

/** A column in words, for a screen reader and the head of the list of a cell. */
function columnName(column: Column): string {
  const first = partsOfDay(column.firstDay);
  const start = `${first.dayOfMonth.toString()} ${formatMonthShort(first.year, first.month)}`;
  if (column.kind === 'day') {
    return start;
  }
  const last = partsOfDay(column.lastDay);
  return `${start} – ${last.dayOfMonth.toString()} ${formatMonthShort(last.year, last.month)}`;
}

interface CellButtonProps {
  cell: Cell;
  /** The fullest cell of the rows without a limit, which their bars are measured against. */
  largest: number;
  isChosen: boolean;
  label: string;
  onPress: () => void;
}

/**
 * A cell: the hours that fall into it, what the person can take there, and
 * under them a hairline bar. Against a limit, the bar is hollow up to it and
 * filled with ink as far as the hours go; past the limit it turns to the
 * colour of danger. Without a limit, the bar is measured against the
 * fullest cell of the specialists.
 */
function CellButton({ cell, largest, isChosen, label, onPress }: CellButtonProps) {
  const isBounded = cell.capacityMinutes !== null;
  const fill = isBounded ? fillOf(cell) : largest > 0 ? cell.minutes / largest : 0;
  const isOver = isBounded && fill > 1;
  const hasWork = cell.minutes > 0;
  const tells = isBounded
    ? `${formatHours(cell.minutes)} of ${formatHours(cell.capacityMinutes ?? 0)} hours`
    : `${formatHours(cell.minutes)} hours`;

  return (
    <button
      type="button"
      role="cell"
      className={classNames(
        styles.cell,
        cell.column.isRestDay && styles.restDay,
        cell.column.isPast && styles.past,
        cell.column.holdsToday && styles.holdsToday,
        cell.column.kind === 'day' && weekdayOf(cell.column.firstDay) === 0 && styles.weekStart,
        isChosen && styles.isChosen,
        isOver && styles.isOver,
      )}
      aria-pressed={isChosen}
      aria-label={`${label}: ${tells}`}
      onClick={onPress}
    >
      <span className={styles.cellFigures}>
        {hasWork && <span className={styles.figure}>{formatHours(cell.minutes)}</span>}
        {hasWork && isBounded && cell.column.kind === 'week' && (
          <span className={styles.capacity}>of {formatHours(cell.capacityMinutes ?? 0)}</span>
        )}
      </span>
      {(hasWork || (isBounded && (cell.capacityMinutes ?? 0) > 0)) && (
        <span
          className={classNames(styles.bar, isBounded ? styles.bounded : styles.unbounded)}
          aria-hidden
        >
          <span
            className={styles.fill}
            style={{ width: `${(Math.min(fill, 1) * 100).toString()}%` }}
          />
        </span>
      )}
    </button>
  );
}

interface HoursPerDayProps {
  hours: number;
  onChange: (hours: number) => void;
}

/** The hours a day a person can work, changed in place. */
function HoursPerDay({ hours, onChange }: HoursPerDayProps) {
  const [text, setText] = useState<string | null>(null);

  function finish() {
    if (text !== null) {
      const parsed = parseHoursPerDay(text);
      if (parsed !== undefined && parsed !== hours) {
        onChange(parsed);
      }
    }
    setText(null);
  }

  function handleKeyDown(event: KeyboardEvent<HTMLInputElement>) {
    if (event.key === 'Enter') {
      finish();
    } else if (event.key === 'Escape') {
      // Escape puts the hours back. It must not also close the panel.
      event.stopPropagation();
      setText(null);
    }
  }

  if (text !== null) {
    const isUnderstood = parseHoursPerDay(text) !== undefined;
    return (
      <span className={styles.hoursLine}>
        <TextInput
          className={styles.hoursInput}
          value={text}
          autoFocus
          inputMode="decimal"
          aria-label="Hours a day"
          aria-invalid={!isUnderstood}
          onChange={(event) => {
            setText(event.target.value);
          }}
          onFocus={(event) => {
            event.target.select();
          }}
          onBlur={finish}
          onKeyDown={handleKeyDown}
        />
        <span className={styles.hoursUnit}>h a day</span>
      </span>
    );
  }

  return (
    <ValueButton
      className={styles.hoursButton}
      aria-label={`${formatHours(hours * 60)} hours a day. Change`}
      onClick={() => {
        setText(formatHours(hours * 60));
      }}
    >
      {formatHours(hours * 60)}h a day
    </ValueButton>
  );
}

/** What a row is called, in words. */
function nameOf(subject: RowSubject): string {
  switch (subject.kind) {
    case 'manager':
      return MANAGER_NAME;
    case 'member':
      return subject.member.name;
    case 'nobody':
      return NOBODY_NAME;
    case 'other':
      return 'Not on the team';
  }
}

/** The name of a row, with the picture of the person. */
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
    case 'other':
      return (
        <>
          <AssigneeAvatar assignee={null} />
          <span className={classNames(styles.subjectName, styles.quiet)}>{nameOf(subject)}</span>
        </>
      );
  }
}

/** A quiet line under a name that has no hours a day: what the person is on the team for. */
function SubjectLine({ subject }: { subject: RowSubject }) {
  const line = subject.kind === 'member' ? subject.member.responsibility : '';
  return line ? <p className={styles.subjectLine}>{line}</p> : null;
}

interface ShareLineProps {
  project: Project;
  share: Share;
  onOpen: () => void;
}

/** A task behind a cell: its status, key and title, its hours here and in all, and its dates. */
function ShareLine({ project, share, onOpen }: ShareLineProps) {
  const { task } = share;
  const status = statusOf(project, task);
  return (
    <button type="button" className={styles.line} onClick={onOpen}>
      <span className={styles.lineMark}>{status && <StatusMark status={status} />}</span>
      <span className={styles.reference}>{taskReference(project, task)}</span>
      <span className={styles.lineTitle}>{task.title}</span>
      <span className={styles.lineHours}>
        {formatHours(share.minutes)}h
        <span className={styles.lineOf}> of {formatHours(task.estimateMinutes ?? 0)}h</span>
      </span>
      <span className={classNames(styles.lineFact, share.isLate && styles.late)}>
        {formatPeriod(task.startOn, task.dueOn)}
      </span>
    </button>
  );
}

interface UnlaidLineProps {
  project: Project;
  task: Task;
  reason: Unlaid;
  onOpen: () => void;
}

/** A task that cannot be laid on the days, and why. */
function UnlaidLine({ project, task, reason, onOpen }: UnlaidLineProps) {
  const status = statusOf(project, task);
  return (
    <button type="button" className={styles.line} onClick={onOpen}>
      <span className={styles.lineMark}>{status && <StatusMark status={status} />}</span>
      <span className={styles.reference}>{taskReference(project, task)}</span>
      <span className={styles.lineTitle}>{task.title}</span>
      <span className={styles.lineHours}>
        {task.estimateMinutes === null ? '' : `${formatHours(task.estimateMinutes)}h`}
      </span>
      <span className={styles.lineFact}>
        {reason === 'no estimate' ? 'No estimate' : 'No due date'}
      </span>
    </button>
  );
}
