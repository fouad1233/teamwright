import { ArrowDown, ArrowUp, ChevronDown } from 'lucide-react';
import {
  useLayoutEffect,
  useMemo,
  useRef,
  useState,
  type CSSProperties,
  type FocusEvent,
  type KeyboardEvent,
} from 'react';

import { useNavigation } from '@/app/navigation';
import type { Id, Priority } from '@/shared/api/types';
import { classNames } from '@/shared/lib/classNames';
import { isOverdue, parseEstimate, taskReference } from '@/shared/lib/format';
import { Button } from '@/shared/ui/Button';
import { CheckMenu } from '@/shared/ui/CheckMenu';
import type { Choice } from '@/shared/ui/Menu';

import { useAreaCommands, useAreas } from '../areas/areaQueries';
import { formatWork, parsePoints } from '../sprints/sprintModel';
import { tagsOf } from '../tags/tagModel';
import { useTags } from '../tags/tagQueries';
import { useSprintCommands, useSprints } from '../sprints/sprintQueries';
import {
  PRIORITIES,
  PriorityFlag,
  StatusLabel,
  StatusMark,
  priorityLabel,
} from '../tasks/TaskBadges';
import { indexById, statusOf } from '../tasks/taskModel';
import { useTaskCommands } from '../tasks/taskQueries';
import {
  AssigneeAvatar,
  AssigneeName,
  MANAGER_NAME,
  MemberAvatar,
  NOBODY_NAME,
} from '../team/Assignee';
import { useTeamMembers } from '../team/TeamContext';
import type { KindProps } from '../views/viewKinds';
import { isFiltered } from '../views/viewModel';
import {
  BACKLOG,
  CELL_TARGET,
  CellField,
  DateCell,
  EMPTY,
  FigureText,
  InheritedText,
  MenuCell,
  TitleText,
  TagsCell,
  WaitsForCell,
  estimateText,
  sprintChoices,
  type FocusIndex,
} from './TableCells';
import styles from './TableView.module.css';
import {
  COLUMNS,
  MANAGER,
  UNASSIGNED,
  assigneeOf,
  assigneeValue,
  describeSort,
  formatHours,
  nextCell,
  shownColumns,
  tableRows,
  totalsOf,
  withColumnToggled,
  withSortBy,
  type CellPosition,
  type Column,
  type ColumnId,
  type SortContext,
  type TableRow,
} from './tableModel';
import { useTableSettings } from './useTableSettings';

/** Moving a task to this position puts it at the bottom of a status. */
const BOTTOM = Number.MAX_SAFE_INTEGER;

/** The columns whose cells are typed into. */
const TYPED: ReadonlySet<ColumnId> = new Set(['title', 'points', 'estimate']);

/** The columns whose cells open the calendar field. */
const DATED: ReadonlySet<ColumnId> = new Set(['start', 'due']);

/** A cell of a task, by the task and the column, so that it follows the task when the rows move. */
interface CellKey {
  taskId: Id;
  column: ColumnId;
}

/**
 * The table: one task a row and every field a column, changed in place.
 * A view of its own for every table, so that two tables do not share what
 * the keyboard stands on.
 */
export function TableView(props: KindProps) {
  return <TableSheet key={props.view.id} {...props} />;
}

function TableSheet({ view, project, tasks, everyTask }: KindProps) {
  const navigation = useNavigation();
  const commands = useTaskCommands(project.id);
  const sprintCommands = useSprintCommands(project.id);
  const areaCommands = useAreaCommands(project.id);
  const sprints = useSprints(project.id).data;
  const areas = useAreas(project.id).data;
  const tags = useTags(project.id).data;
  const team = useTeamMembers();
  const [settings, changeSettings] = useTableSettings(view);
  const [active, setActive] = useState<CellKey | null>(null);
  const [editing, setEditing] = useState<CellKey | null>(null);
  // Bumped whenever the keyboard is to land on the active cell after the
  // next drawing.
  const [focusRequest, setFocusRequest] = useState(0);
  const grid = useRef<HTMLTableElement>(null);
  const scroller = useRef<HTMLDivElement>(null);

  const columns = shownColumns(settings);
  const tasksById = useMemo(() => indexById(everyTask), [everyTask]);

  const context = useMemo<SortContext>(
    () => ({
      statusOrder: new Map(project.statuses.map((status, index) => [status.id, index])),
      sprintOrder: new Map((sprints ?? []).map((sprint, index) => [sprint.id, index])),
      areaOrder: new Map((areas ?? []).map((area, index) => [area.id, index])),
      assigneeName: (assignee) =>
        assignee.kind === 'manager'
          ? MANAGER_NAME
          : (team.find((member) => member.id === assignee.memberId)?.name ?? ''),
      tagNames: (task) => {
        const carried = tagsOf(task, tags ?? []);
        return carried.length === 0 ? null : carried.map((tag) => tag.name).join(', ');
      },
    }),
    [project.statuses, sprints, areas, tags, team],
  );
  const rows = useMemo(
    () => tableRows(tasks, settings.sort, context),
    [tasks, settings.sort, context],
  );
  const totals = useMemo(() => totalsOf(tasks), [tasks]);

  // The cell the keyboard stands on: the one chosen last, wherever its task
  // has gone, or the first.
  const activeRow = Math.max(
    rows.findIndex((row) => row.task.id === active?.taskId),
    0,
  );
  const activeColumn = Math.max(
    columns.findIndex((column) => column.id === active?.column),
    0,
  );

  useLayoutEffect(() => {
    if (focusRequest === 0) {
      return;
    }
    const cell = grid.current?.querySelector<HTMLElement>(
      `td[data-row="${activeRow.toString()}"][data-col="${activeColumn.toString()}"]`,
    );
    if (!cell) {
      return;
    }
    const target = cell.querySelector<HTMLElement>(`[${CELL_TARGET}]`) ?? cell;
    target.focus({ preventScroll: true });
    cell.scrollIntoView({ block: 'nearest', inline: 'nearest' });
    // Only a new request moves the keyboard, not every drawing.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [focusRequest]);

  function keyOf(position: CellPosition): CellKey | null {
    const row = rows[position.row];
    const column = columns[position.column];
    return row && column ? { taskId: row.task.id, column: column.id } : null;
  }

  function moveTo(position: CellPosition) {
    const key = keyOf(position);
    if (key) {
      setActive(key);
      setFocusRequest((request) => request + 1);
    }
  }

  function positionOf(element: Element | null): { cell: HTMLElement; at: CellPosition } | null {
    const cell = element?.closest<HTMLElement>('td[data-row]');
    if (!cell) {
      return null;
    }
    return {
      cell,
      at: { row: Number(cell.dataset.row), column: Number(cell.dataset.col) },
    };
  }

  /** Enter on a cell that has no control of its own: type into it, or open its calendar. */
  function activate(cell: HTMLElement, at: CellPosition) {
    const key = keyOf(at);
    if (!key) {
      return;
    }
    if (TYPED.has(key.column)) {
      setEditing(key);
    } else if (DATED.has(key.column)) {
      cell.querySelector('button')?.click();
    }
  }

  function handleKeyDown(event: KeyboardEvent<HTMLTableSectionElement>) {
    const target = event.target as HTMLElement;
    // Keys pressed in a menu or a calendar opened from a cell reach here
    // through the components, though not through the page: they are theirs.
    // So are the keys of a field that is being typed into.
    if (!event.currentTarget.contains(target) || target.closest('[data-editing]')) {
      return;
    }
    const found = positionOf(target);
    if (!found) {
      return;
    }
    const next = nextCell(found.at, event.key, event.ctrlKey || event.metaKey, {
      rows: rows.length,
      columns: columns.length,
    });
    if (next) {
      // Taken before a control of the cell sees it: an arrow moves to the
      // next cell, and does not open a menu.
      event.preventDefault();
      event.stopPropagation();
      moveTo(next);
    } else if ((event.key === 'Enter' || event.key === 'F2') && target === found.cell) {
      event.preventDefault();
      activate(found.cell, found.at);
    }
  }

  function handleFocus(event: FocusEvent<HTMLTableSectionElement>) {
    if (!event.currentTarget.contains(event.target)) {
      return;
    }
    const found = positionOf(event.target);
    const key = found && keyOf(found.at);
    if (key && (key.taskId !== active?.taskId || key.column !== active.column)) {
      setActive(key);
    }
  }

  function doneEditing(refocus: boolean) {
    setEditing(null);
    if (refocus) {
      setFocusRequest((request) => request + 1);
    }
  }

  const statusChoices: Choice<Id>[] = project.statuses.map((status) => ({
    value: status.id,
    label: status.name,
    icon: <StatusMark status={status} />,
  }));
  const assigneeChoices: Choice<string>[] = [
    { value: UNASSIGNED, label: NOBODY_NAME, icon: <AssigneeAvatar assignee={null} /> },
    {
      value: MANAGER,
      label: MANAGER_NAME,
      icon: <AssigneeAvatar assignee={{ kind: 'manager' }} />,
    },
    ...team.map((member) => ({
      value: member.id,
      label: member.name,
      icon: <MemberAvatar member={member} />,
    })),
  ];
  const priorityChoices: Choice<Priority>[] = PRIORITIES.map((priority) => ({
    value: priority,
    label: priorityLabel(priority),
    icon: <PriorityFlag priority={priority} />,
  }));
  const areaChoices: Choice<Id>[] = (areas ?? []).map((area) => ({
    value: area.id,
    label: area.name,
  }));

  /** What stands in one cell of a row. */
  function cellOf(row: TableRow, column: Column, focusIndex: FocusIndex) {
    const { task } = row;
    const reference = taskReference(project, task);
    const parent = task.parentId === null ? undefined : tasksById.get(task.parentId);
    const isEditing = editing?.taskId === task.id && editing.column === column.id;

    switch (column.id) {
      case 'reference':
        return (
          <button
            type="button"
            className={styles.reference}
            tabIndex={focusIndex}
            data-cell-target=""
            title={`Open ${reference}`}
            onClick={() => {
              navigation.openTask(task.id);
            }}
          >
            {reference}
          </button>
        );

      case 'title':
        return isEditing ? (
          <span className={styles.titleLine} style={{ '--depth': row.depth } as CSSProperties}>
            <CellField
              label={`Title of ${reference}`}
              initial={task.title}
              parse={(text) => (text.trim() === '' ? undefined : text.trim())}
              hint="A task needs a title"
              onCommit={(title) => {
                if (title !== task.title) {
                  void commands.rename(task.id, title);
                }
              }}
              onDone={doneEditing}
            />
          </span>
        ) : (
          <TitleText task={task} depth={row.depth} />
        );

      case 'status': {
        const status = statusOf(project, task);
        return (
          <MenuCell
            label={`Status of ${reference}`}
            choices={statusChoices}
            value={task.statusId}
            focusIndex={focusIndex}
            onChoose={(statusId) => {
              void commands.move(task.id, statusId, BOTTOM);
            }}
          >
            {status && <StatusLabel status={status} />}
          </MenuCell>
        );
      }

      case 'assignee':
        return (
          <MenuCell
            label={`Assignee of ${reference}`}
            choices={assigneeChoices}
            value={assigneeValue(task.assignee)}
            focusIndex={focusIndex}
            isEmpty={task.assignee === null}
            onChoose={(value) => {
              void commands.assign(task.id, assigneeOf(value));
            }}
          >
            <AssigneeAvatar assignee={task.assignee} />
            <AssigneeName assignee={task.assignee} />
          </MenuCell>
        );

      case 'priority':
        return (
          <MenuCell
            label={`Priority of ${reference}`}
            choices={priorityChoices}
            value={task.priority}
            focusIndex={focusIndex}
            isEmpty={task.priority === 'normal'}
            onChoose={(priority) => {
              void commands.prioritize(task.id, priority);
            }}
          >
            {/* Normal is what most tasks are: only a priority that differs is written. */}
            {task.priority === 'normal' ? (
              EMPTY
            ) : (
              <PriorityFlag priority={task.priority} withLabel />
            )}
          </MenuCell>
        );

      case 'points':
        return isEditing ? (
          <CellField
            label={`Points of ${reference}`}
            initial={task.points === null ? '' : task.points.toString()}
            parse={parsePoints}
            hint="Story points: a whole number"
            isFigure
            onCommit={(points) => {
              if (points !== task.points) {
                void sprintCommands.setPoints(task.id, points);
              }
            }}
            onDone={doneEditing}
          />
        ) : (
          <FigureText value={task.points === null ? null : task.points.toString()} />
        );

      case 'estimate':
        return isEditing ? (
          <CellField
            label={`Estimate of ${reference}`}
            initial={estimateText(task) ?? ''}
            parse={parseEstimate}
            hint="Write it like 90m, 2h or 1h 30m"
            isFigure
            onCommit={(minutes) => {
              if (minutes !== task.estimateMinutes) {
                void commands.estimate(task.id, minutes);
              }
            }}
            onDone={doneEditing}
          />
        ) : (
          <FigureText value={estimateText(task)} />
        );

      case 'start':
        return (
          <DateCell
            label={`Start date of ${reference}`}
            value={task.startOn}
            earliest={null}
            latest={task.dueOn}
            onChange={(startOn) => {
              void commands.reschedule(task.id, startOn, task.dueOn);
            }}
          />
        );

      case 'due':
        return (
          <DateCell
            label={`Due date of ${reference}`}
            value={task.dueOn}
            earliest={task.startOn}
            latest={null}
            isLate={isOverdue(task)}
            onChange={(dueOn) => {
              void commands.reschedule(task.id, task.startOn, dueOn);
            }}
          />
        );

      case 'sprint': {
        const current = sprints?.find((sprint) => sprint.id === task.sprintId);
        // While the sprints load, a task in a sprint is not called "Backlog".
        const name = current?.name ?? (task.sprintId === null ? 'Backlog' : '');
        if (parent) {
          return <InheritedText text={name} from={taskReference(project, parent)} />;
        }
        return (
          <MenuCell
            label={`Sprint of ${reference}`}
            choices={sprintChoices(sprints ?? [], task)}
            value={task.sprintId ?? BACKLOG}
            focusIndex={focusIndex}
            isEmpty={task.sprintId === null}
            onChoose={(value) => {
              void sprintCommands.plan(task.id, value === BACKLOG ? null : value);
            }}
          >
            <span className={styles.clipped}>{name}</span>
            {current?.state === 'active' && <span className={styles.activeMark} title="Active" />}
          </MenuCell>
        );
      }

      case 'area': {
        const name = areas?.find((area) => area.id === task.areaId)?.name ?? '';
        if (parent) {
          return <InheritedText text={name} from={taskReference(project, parent)} />;
        }
        return (
          <MenuCell
            label={`Area of ${reference}`}
            choices={areaChoices}
            value={task.areaId}
            focusIndex={focusIndex}
            onChoose={(areaId) => {
              void areaCommands.moveTask(task.id, areaId);
            }}
          >
            <span className={styles.clipped}>{name}</span>
          </MenuCell>
        );
      }

      case 'tags':
        return (
          <TagsCell
            project={project}
            task={task}
            tags={tagsOf(task, tags ?? [])}
            focusIndex={focusIndex}
          />
        );

      case 'waitsFor':
        return (
          <WaitsForCell
            project={project}
            task={task}
            everyTask={everyTask}
            tasksById={tasksById}
            focusIndex={focusIndex}
            onAdd={(blockerId) => {
              void commands.addDependency(task.id, blockerId);
            }}
            onRemove={(blockerId) => {
              void commands.removeDependency(task.id, blockerId);
            }}
            onOpen={navigation.openTask}
          />
        );
    }
  }

  /** Whether a cell is reached as the cell itself, having no control of its own. */
  function isPlainCell(row: TableRow, column: Column) {
    if (TYPED.has(column.id) || DATED.has(column.id)) {
      return true;
    }
    return (column.id === 'sprint' || column.id === 'area') && row.task.parentId !== null;
  }

  const hiddenCount = COLUMNS.length - columns.length;
  const fixedWidth = COLUMNS.filter((column) => column.isFixed).reduce(
    (sum, column) => sum + column.width,
    0,
  );
  const sheetWidth = columns.reduce((sum, column) => sum + column.width, 0);
  const referenceWidth = COLUMNS[0]?.width ?? 0;

  return (
    <div className={styles.table}>
      <div className={styles.bar}>
        <span className={styles.order}>
          {settings.sort === null ? (
            <span className={styles.orderNote}>In the order of the board</span>
          ) : (
            <>
              <span className={styles.orderNote}>
                Sorted by <span className={styles.orderValue}>{describeSort(settings.sort)}</span>
              </span>
              <Button
                variant="ghost"
                className={styles.barButton}
                onClick={() => {
                  changeSettings((current) => ({ ...current, sort: null }));
                }}
              >
                Order of the board
              </Button>
            </>
          )}
        </span>

        <CheckMenu
          label="Columns"
          items={COLUMNS.map((column) => ({
            value: column.id,
            label: column.label,
            isChecked: column.isFixed || settings.columns.includes(column.id),
            isDisabled: column.isFixed,
          }))}
          onToggle={(id) => {
            changeSettings((current) => withColumnToggled(current, id));
          }}
        >
          <Button variant="ghost" className={styles.barButton}>
            Columns
            {hiddenCount > 0 && (
              <span className={styles.count}>
                {columns.length.toString()}/{COLUMNS.length.toString()}
              </span>
            )}
            <ChevronDown aria-hidden />
          </Button>
        </CheckMenu>
      </div>

      <div
        ref={scroller}
        className={styles.scroller}
        style={
          {
            '--fixed': `${fixedWidth.toString()}px`,
            '--reference': `${referenceWidth.toString()}px`,
          } as CSSProperties
        }
      >
        <table
          ref={grid}
          className={styles.grid}
          style={{ '--sheet': `${sheetWidth.toString()}px` } as CSSProperties}
          aria-label={view.name}
          aria-rowcount={rows.length + 2}
        >
          <colgroup>
            {columns.map((column) => (
              <col
                key={column.id}
                // The title takes what is left of the width of the window.
                style={column.id === 'title' ? undefined : { width: column.width }}
              />
            ))}
          </colgroup>

          <thead>
            <tr>
              {columns.map((column) => {
                const sort = settings.sort?.column === column.id ? settings.sort : null;
                return (
                  <th
                    key={column.id}
                    scope="col"
                    className={classNames(
                      column.isFixed && styles.fixed,
                      column.id === 'title' && styles.lastFixed,
                      column.isFigure && styles.figureColumn,
                    )}
                    aria-sort={sort?.direction ?? 'none'}
                  >
                    <button
                      type="button"
                      className={classNames(styles.sortButton, sort && styles.sorted)}
                      title={
                        sort ? 'Press to turn the order' : `Sort by ${column.label.toLowerCase()}`
                      }
                      onClick={() => {
                        changeSettings((current) => withSortBy(current, column.id));
                      }}
                    >
                      <span className="label">{column.label}</span>
                      {sort &&
                        (sort.direction === 'ascending' ? (
                          <ArrowUp aria-hidden />
                        ) : (
                          <ArrowDown aria-hidden />
                        ))}
                    </button>
                  </th>
                );
              })}
            </tr>
          </thead>

          <tbody onKeyDownCapture={handleKeyDown} onFocus={handleFocus}>
            {rows.length === 0 ? (
              <tr>
                <td className={styles.emptyState} colSpan={columns.length}>
                  <EmptyNote
                    emptiness={
                      everyTask.length === 0
                        ? 'project'
                        : isFiltered(view.filter)
                          ? 'filter'
                          : 'area'
                    }
                  />
                </td>
              </tr>
            ) : (
              rows.map((row, rowIndex) => (
                <tr
                  key={row.task.id}
                  className={classNames(
                    styles.row,
                    navigation.taskId === row.task.id && styles.openRow,
                    row.task.completedAt !== null && styles.finishedRow,
                  )}
                >
                  {columns.map((column, columnIndex) => {
                    const isActive = rowIndex === activeRow && columnIndex === activeColumn;
                    const focusIndex: FocusIndex = isActive ? 0 : -1;
                    const isPlain = isPlainCell(row, column);
                    return (
                      <td
                        key={column.id}
                        data-row={rowIndex}
                        data-col={columnIndex}
                        tabIndex={isPlain ? focusIndex : undefined}
                        className={classNames(
                          styles.cell,
                          column.isFixed && styles.fixed,
                          column.id === 'title' && styles.lastFixed,
                          column.isFigure && styles.figureColumn,
                          editing?.taskId === row.task.id &&
                            editing.column === column.id &&
                            styles.editing,
                        )}
                        onDoubleClick={() => {
                          if (TYPED.has(column.id)) {
                            setActive({ taskId: row.task.id, column: column.id });
                            setEditing({ taskId: row.task.id, column: column.id });
                          }
                        }}
                      >
                        {cellOf(row, column, focusIndex)}
                      </td>
                    );
                  })}
                </tr>
              ))
            )}
          </tbody>

          {rows.length > 0 && (
            <tfoot>
              <tr>
                {columns.map((column) => (
                  <td
                    key={column.id}
                    className={classNames(
                      column.isFixed && styles.fixed,
                      column.id === 'title' && styles.lastFixed,
                      column.isFigure && styles.figureColumn,
                    )}
                  >
                    <FootFigure column={column.id} totals={totals} />
                  </td>
                ))}
              </tr>
            </tfoot>
          )}
        </table>
      </div>
    </div>
  );
}

/** What the foot says under a column: the count of tasks, the points and the hours. */
function FootFigure({ column, totals }: { column: ColumnId; totals: ReturnType<typeof totalsOf> }) {
  switch (column) {
    case 'title':
      return <span className={styles.footCount}>{formatWork(totals.tasks, 'tasks')}</span>;
    case 'points':
      return (
        <span
          className={styles.footFigure}
          title={
            totals.withoutPoints > 0
              ? `${formatWork(totals.withoutPoints, 'tasks')} without points`
              : undefined
          }
        >
          {totals.points.toString()}
        </span>
      );
    case 'estimate':
      return (
        <span
          className={styles.footFigure}
          title={
            totals.withoutEstimate > 0
              ? `${formatWork(totals.withoutEstimate, 'tasks')} without an estimate`
              : undefined
          }
        >
          {totals.minutes === 0 ? EMPTY : formatHours(totals.minutes)}
        </span>
      );
    default:
      return null;
  }
}

/** Why the table has no rows: the project has no task, the filter lets none through, or the area has none. */
type Emptiness = 'project' | 'filter' | 'area';

/** What the table says when it has no rows. */
function EmptyNote({ emptiness }: { emptiness: Emptiness }) {
  switch (emptiness) {
    case 'project':
      return (
        <p className={styles.note}>
          Every task becomes a row here, and each of its fields a column you change in place. Start
          with New task, above.
        </p>
      );
    case 'filter':
      return (
        <p className={styles.note}>
          No task passes the filter of this view. Change it from the menu of the view’s tab.
        </p>
      );
    case 'area':
      return (
        <p className={styles.note}>
          This area has no tasks yet. Every task you make here becomes a row of the table.
        </p>
      );
  }
}
