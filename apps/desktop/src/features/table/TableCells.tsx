import { X } from 'lucide-react';
import { Popover } from 'radix-ui';
import {
  Fragment,
  useLayoutEffect,
  useRef,
  useState,
  type CSSProperties,
  type KeyboardEvent,
  type ReactNode,
} from 'react';

import type { CalendarDate, Id, Project, Sprint, Tag, Task } from '@/shared/api/types';
import { classNames } from '@/shared/lib/classNames';
import { formatEstimate, taskReference } from '@/shared/lib/format';
import { IconButton, ValueButton } from '@/shared/ui/Button';
import { DateField } from '@/shared/ui/DateField';
import { ChoiceMenu, type Choice } from '@/shared/ui/Menu';

import { TagChips } from '../tags/TagChip';
import { TaskTags } from '../tags/TaskTags';
import { StatusMark } from '../tasks/TaskBadges';
import { TaskPicker } from '../tasks/TaskPicker';
import { statusOf } from '../tasks/taskModel';
import styles from './TableView.module.css';

/**
 * The cells of the table. Each one shows a field of a task and changes it
 * with the program's own controls: the menus, the calendar field, the task
 * picker. A cell that has a control of its own is reached through that
 * control; the others are reached as the cell itself. Only the cell the
 * keyboard stands on can be reached with Tab (`focusIndex` 0): the arrows
 * lead to the rest.
 */

/** What a cell reached with Tab is given: 0 for the one the keyboard stands on, -1 for the others. */
export type FocusIndex = 0 | -1;

/** What marks the control a cell is reached through. The table looks for it. */
export const CELL_TARGET = 'data-cell-target';

/** What stands in a cell that has no value. */
export const EMPTY = '–';

interface MenuCellProps<Value extends string> {
  /** What is chosen, for screen readers: "Status of SH-4". */
  label: string;
  choices: readonly Choice<Value>[];
  value: Value;
  onChoose: (value: Value) => void;
  focusIndex: FocusIndex;
  isEmpty?: boolean;
  /** What the cell shows. */
  children: ReactNode;
}

/** A field chosen from a menu: the status, the assignee, the priority, the sprint, the area. */
export function MenuCell<Value extends string>({
  label,
  choices,
  value,
  onChoose,
  focusIndex,
  isEmpty = false,
  children,
}: MenuCellProps<Value>) {
  return (
    <ChoiceMenu
      label={label}
      choices={choices}
      value={value}
      onChoose={(chosen) => {
        if (chosen !== value) {
          onChoose(chosen);
        }
      }}
    >
      <ValueButton
        className={styles.cellButton}
        aria-label={label}
        isEmpty={isEmpty}
        tabIndex={focusIndex}
        data-cell-target=""
      >
        {children}
      </ValueButton>
    </ChoiceMenu>
  );
}

interface CellFieldProps<Value> {
  /** What the field holds, for screen readers: "Points of SH-4". */
  label: string;
  /** The text the field starts with. */
  initial: string;
  /** Reads what was typed: the value, or nothing if it cannot be understood. */
  parse: (text: string) => Value | undefined;
  /** What a hint says when the text is not understood. */
  hint: string;
  onCommit: (value: Value) => void;
  /** Called once the field is done with: with whether the cell is to take the keyboard back. */
  onDone: (refocus: boolean) => void;
  isFigure?: boolean;
}

/**
 * A field typed into in place: the title, the points, the estimate. Enter
 * or leaving it keeps what was typed, if it is understood; Escape gives it
 * up. Either way the cell is left.
 */
export function CellField<Value>({
  label,
  initial,
  parse,
  hint,
  onCommit,
  onDone,
  isFigure = false,
}: CellFieldProps<Value>) {
  const [draft, setDraft] = useState(initial);
  const isDone = useRef(false);
  const parsed = parse(draft);
  const isUnderstood = parsed !== undefined;

  function finish(keep: boolean, refocus: boolean) {
    if (isDone.current) {
      return;
    }
    isDone.current = true;
    if (keep && parsed !== undefined) {
      onCommit(parsed);
    }
    onDone(refocus);
  }

  function handleKeyDown(event: KeyboardEvent<HTMLInputElement>) {
    if (event.key === 'Enter') {
      event.preventDefault();
      // What cannot be understood is not kept: the field stays, marked.
      if (isUnderstood) {
        finish(true, true);
      }
    } else if (event.key === 'Escape') {
      // Escape leaves the field. It must not also close the panel of a task.
      event.preventDefault();
      event.stopPropagation();
      finish(false, true);
    }
  }

  return (
    <input
      className={classNames(
        styles.field,
        isFigure && styles.figureField,
        !isUnderstood && styles.notUnderstood,
      )}
      value={draft}
      aria-label={label}
      aria-invalid={!isUnderstood}
      title={isUnderstood ? undefined : hint}
      autoFocus
      data-editing
      onFocus={(event) => {
        event.currentTarget.select();
      }}
      onChange={(event) => {
        setDraft(event.target.value);
      }}
      onBlur={() => {
        finish(true, false);
      }}
      onKeyDown={handleKeyDown}
    />
  );
}

/** The title of a task, set in one step for every task it is part of. */
export function TitleText({ task, depth }: { task: Task; depth: number }) {
  return (
    <span
      className={classNames(styles.titleLine, depth > 0 && styles.subtask)}
      style={{ '--depth': depth } as CSSProperties}
    >
      <span className={styles.titleText}>{task.title}</span>
    </span>
  );
}

/** A figure, or the mark of an empty cell. */
export function FigureText({ value }: { value: string | null }) {
  return value === null ? (
    <span className={styles.empty}>{EMPTY}</span>
  ) : (
    <span className={styles.figure}>{value}</span>
  );
}

/** The estimate of a task as a figure. */
export function estimateText(task: Task): string | null {
  return task.estimateMinutes === null ? null : formatEstimate(task.estimateMinutes);
}

interface DateCellProps {
  /** What the date is, for screen readers: "Due date of SH-4". */
  label: string;
  value: CalendarDate | null;
  earliest: CalendarDate | null;
  latest: CalendarDate | null;
  /** Whether the date is a due date that has passed with the task unfinished. */
  isLate?: boolean;
  onChange: (value: CalendarDate | null) => void;
}

/**
 * A date, chosen from the program's calendar field. The cell itself is
 * reached with the keyboard and opens the field with Enter, so the field's
 * own button is taken out of the order of Tab.
 */
export function DateCell({
  label,
  value,
  earliest,
  latest,
  isLate = false,
  onChange,
}: DateCellProps) {
  const holder = useRef<HTMLSpanElement>(null);

  useLayoutEffect(() => {
    for (const button of holder.current?.querySelectorAll('button') ?? []) {
      button.tabIndex = -1;
    }
  });

  return (
    <span
      ref={holder}
      className={classNames(styles.date, value === null && styles.noDate, isLate && styles.late)}
    >
      <DateField
        label={label}
        value={value}
        earliest={earliest}
        latest={latest}
        onChange={(chosen) => {
          if (chosen !== value) {
            onChange(chosen);
          }
        }}
      />
    </span>
  );
}

/** The sprint menu: the backlog, the sprints that take tasks, and the one the task is in. */
export function sprintChoices(sprints: readonly Sprint[], task: Task): Choice<string>[] {
  return [
    { value: BACKLOG, label: 'Backlog' },
    ...sprints
      .filter((sprint) => sprint.state !== 'completed' || sprint.id === task.sprintId)
      .map((sprint) => ({ value: sprint.id, label: sprint.name })),
  ];
}

/** The value of the sprint menu that stands for the backlog. */
export const BACKLOG = 'backlog';

/** A field a subtask takes from its task, so that it cannot be changed here. */
export function InheritedText({ text, from }: { text: string; from: string }) {
  return (
    <span className={styles.inherited} title={`A subtask goes where ${from} goes`}>
      {text}
    </span>
  );
}

interface TagsCellProps {
  project: Project;
  task: Task;
  /** The tags the task carries, by name. */
  tags: readonly Tag[];
  focusIndex: FocusIndex;
}

/**
 * The tags of a task, as many as fit and "+N" for the rest. A press opens a
 * small sheet in which tags are put on, taken off, or made.
 */
export function TagsCell({ project, task, tags, focusIndex }: TagsCellProps) {
  const reference = taskReference(project, task);
  return (
    <Popover.Root>
      <Popover.Trigger asChild>
        <ValueButton
          className={styles.cellButton}
          aria-label={`Tags of ${reference}`}
          isEmpty={tags.length === 0}
          tabIndex={focusIndex}
          data-cell-target=""
        >
          {tags.length === 0 ? EMPTY : <TagChips tags={tags} most={2} className={styles.tags} />}
        </ValueButton>
      </Popover.Trigger>
      <Popover.Portal>
        <Popover.Content
          className={styles.sheet}
          align="start"
          sideOffset={4}
          aria-label={`Tags of ${reference}`}
        >
          <header className={styles.sheetHead}>
            <span className="label">Tags of {reference}</span>
          </header>
          <TaskTags project={project} task={task} />
        </Popover.Content>
      </Popover.Portal>
    </Popover.Root>
  );
}

interface WaitsForCellProps {
  project: Project;
  task: Task;
  /** Every task of the project, to find the ones it waits for and those it could. */
  everyTask: readonly Task[];
  tasksById: ReadonlyMap<Id, Task>;
  focusIndex: FocusIndex;
  onAdd: (blockerId: Id) => void;
  onRemove: (blockerId: Id) => void;
  onOpen: (taskId: Id) => void;
}

/**
 * The tasks a task waits for, by their references; a finished one is quiet.
 * A press opens a small sheet in which one is opened, let go, or added.
 */
export function WaitsForCell({
  project,
  task,
  everyTask,
  tasksById,
  focusIndex,
  onAdd,
  onRemove,
  onOpen,
}: WaitsForCellProps) {
  const reference = taskReference(project, task);
  const blockers = task.blockedBy
    .map((blockerId) => tasksById.get(blockerId))
    .filter((blocker): blocker is Task => blocker !== undefined)
    .sort((first, second) => first.number - second.number);
  const candidates = everyTask
    .filter((other) => other.id !== task.id && !task.blockedBy.includes(other.id))
    .sort((first, second) => first.number - second.number);

  return (
    <Popover.Root>
      <Popover.Trigger asChild>
        <ValueButton
          className={styles.cellButton}
          aria-label={`What ${reference} waits for`}
          isEmpty={blockers.length === 0}
          tabIndex={focusIndex}
          data-cell-target=""
        >
          {blockers.length === 0 ? (
            EMPTY
          ) : (
            <span className={styles.references}>
              {blockers.map((blocker, index) => (
                <Fragment key={blocker.id}>
                  {index > 0 && ' '}
                  <span className={classNames(blocker.completedAt !== null && styles.finished)}>
                    {taskReference(project, blocker)}
                  </span>
                </Fragment>
              ))}
            </span>
          )}
        </ValueButton>
      </Popover.Trigger>
      <Popover.Portal>
        <Popover.Content
          className={styles.sheet}
          align="start"
          sideOffset={4}
          aria-label={`What ${reference} waits for`}
        >
          <header className={styles.sheetHead}>
            <span className="label">{reference} waits for</span>
            {candidates.length > 0 && (
              <TaskPicker
                label="Add a task to wait for"
                project={project}
                candidates={candidates}
                onPick={onAdd}
              />
            )}
          </header>
          {blockers.length === 0 ? (
            <p className={styles.sheetNote}>Nothing. This task can start at any time.</p>
          ) : (
            <ul className={styles.blockers}>
              {blockers.map((blocker) => {
                const status = statusOf(project, blocker);
                return (
                  <li key={blocker.id} className={styles.blocker}>
                    <button
                      type="button"
                      className={styles.blockerLink}
                      title={status?.name}
                      onClick={() => {
                        onOpen(blocker.id);
                      }}
                    >
                      {status && <StatusMark status={status} />}
                      <span className={styles.blockerReference}>
                        {taskReference(project, blocker)}
                      </span>
                      <span className={styles.blockerTitle}>{blocker.title}</span>
                    </button>
                    <IconButton
                      label="Stop waiting for this task"
                      onClick={() => {
                        onRemove(blocker.id);
                      }}
                    >
                      <X />
                    </IconButton>
                  </li>
                );
              })}
            </ul>
          )}
        </Popover.Content>
      </Popover.Portal>
    </Popover.Root>
  );
}
