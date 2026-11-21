import type { Assignee, CustomField, Id, Priority, Task } from '@/shared/api/types';

import { inOrder as fieldsInOrder, sortValueOfField } from '../fields/fieldModel';
import { asArranged } from '../tasks/taskModel';

/**
 * What the table view works out without drawing anything: its columns, what
 * it keeps of its own, the order of its rows, the sums of its foot, and where
 * the arrow keys lead. Pure functions, so that they can be tested alone.
 */

/** The columns of the table, by the name they are kept under: one of every task, or a custom field. */
export type ColumnId = BuiltInColumnId | FieldColumnId;

/** The column of a custom field: `field:` and the identifier of the field. */
export type FieldColumnId = `field:${string}`;

/** The start of the name a column of a custom field is kept under. */
const FIELD_COLUMN = 'field:';

/** The column of a custom field. */
export function fieldColumnId(fieldId: Id): FieldColumnId {
  return `${FIELD_COLUMN}${fieldId}`;
}

/** The field a column shows, if it is the column of a custom field. */
export function fieldIdOf(column: ColumnId): Id | null {
  return column.startsWith(FIELD_COLUMN) ? column.slice(FIELD_COLUMN.length) : null;
}

/** The columns every task has. */
export type BuiltInColumnId =
  | 'reference'
  | 'title'
  | 'status'
  | 'assignee'
  | 'priority'
  | 'points'
  | 'estimate'
  | 'start'
  | 'due'
  | 'sprint'
  | 'area'
  | 'tags'
  | 'waitsFor';

export interface Column {
  id: ColumnId;
  /** The word in the head. */
  label: string;
  /** How wide the column is, in pixels. The title takes this at least, and the rest of the row. */
  width: number;
  /** Figures are set to the right, so that their units stand under each other. */
  isFigure: boolean;
  /** The task and its title are always shown, and stay at the left while the rest scrolls. */
  isFixed: boolean;
}

/** Every column, in the order they stand. */
export const COLUMNS: readonly Column[] = [
  { id: 'reference', label: 'Task', width: 88, isFigure: false, isFixed: true },
  { id: 'title', label: 'Title', width: 320, isFigure: false, isFixed: true },
  { id: 'status', label: 'Status', width: 148, isFigure: false, isFixed: false },
  { id: 'assignee', label: 'Assignee', width: 192, isFigure: false, isFixed: false },
  { id: 'priority', label: 'Priority', width: 112, isFigure: false, isFixed: false },
  { id: 'points', label: 'Points', width: 76, isFigure: true, isFixed: false },
  { id: 'estimate', label: 'Estimate', width: 96, isFigure: true, isFixed: false },
  { id: 'start', label: 'Start', width: 112, isFigure: false, isFixed: false },
  { id: 'due', label: 'Due', width: 112, isFigure: false, isFixed: false },
  { id: 'sprint', label: 'Sprint', width: 136, isFigure: false, isFixed: false },
  { id: 'area', label: 'Area', width: 128, isFigure: false, isFixed: false },
  { id: 'tags', label: 'Tags', width: 208, isFigure: false, isFixed: false },
  { id: 'waitsFor', label: 'Waits for', width: 172, isFigure: false, isFixed: false },
];

/** How wide the column of a field of each kind is. */
const FIELD_WIDTHS: Record<CustomField['kind'], number> = {
  text: 200,
  number: 120,
  date: 128,
  yes_no: 96,
  choice: 160,
  link: 200,
};

/** The column of a custom field: its name in the head, after the columns every task has. */
export function fieldColumn(field: CustomField): Column {
  return {
    id: fieldColumnId(field.id),
    label: field.name,
    width: FIELD_WIDTHS[field.kind],
    isFigure: field.kind === 'number',
    isFixed: false,
  };
}

/** Every column a table of a project can show: those of every task, then one for each field. */
export function columnsOf(fields: readonly CustomField[] = []): Column[] {
  return [...COLUMNS, ...fieldsInOrder(fields).map(fieldColumn)];
}

const COLUMN_IDS: ReadonlySet<string> = new Set(COLUMNS.map((column) => column.id));

function isColumnId(value: unknown): value is ColumnId {
  return (
    typeof value === 'string' &&
    (COLUMN_IDS.has(value) ||
      (value.startsWith(FIELD_COLUMN) && value.length > FIELD_COLUMN.length))
  );
}

export type SortDirection = 'ascending' | 'descending';

/** The column the rows are sorted by, and which way. */
export interface TableSort {
  column: ColumnId;
  direction: SortDirection;
}

/** What a table view keeps of its own, in `View.content`. */
export interface TableSettings {
  /**
   * The columns that are shown, in the order of `columnsOf`. The fixed ones
   * are always among them. A column of a field that is gone is passed over.
   */
  columns: ColumnId[];
  /** Nothing keeps the rows in the order the manager arranged the tasks. */
  sort: TableSort | null;
  /**
   * Whether the manager chose which fields the table shows. Left out, it
   * shows every field, a new one too: so does a table that kept its columns
   * before there were fields.
   */
  fieldsChosen?: true;
}

/** A new table shows every column, in the order the manager arranged the tasks. */
export const DEFAULT_SETTINGS: TableSettings = {
  columns: COLUMNS.map((column) => column.id),
  sort: null,
};

/**
 * The columns, in their order, that are among the ones named. The fixed ones
 * always are. A field column keeps its place after the others, in the order
 * of the fields when they are known.
 */
function inOrder(named: Iterable<ColumnId>, fields?: readonly CustomField[]): ColumnId[] {
  const wanted = new Set(named);
  const builtIn = COLUMNS.filter((column) => column.isFixed || wanted.has(column.id)).map(
    (column) => column.id,
  );
  const ofFields = [...wanted].filter((id) => fieldIdOf(id) !== null);
  if (fields) {
    const order = new Map(fieldsInOrder(fields).map((field, index) => [field.id, index]));
    ofFields.sort(
      (first, second) =>
        (order.get(fieldIdOf(first) ?? '') ?? Number.MAX_SAFE_INTEGER) -
        (order.get(fieldIdOf(second) ?? '') ?? Number.MAX_SAFE_INTEGER),
    );
  }
  return [...builtIn, ...ofFields];
}

/**
 * Reads what a table view keeps. Content may be missing, empty, unreadable
 * or written by another version of the program: whatever cannot be
 * understood falls back to the default, and nothing is thrown.
 */
export function parseSettings(content: string | null): TableSettings {
  if (content === null || content.trim() === '') {
    return DEFAULT_SETTINGS;
  }
  let stored: unknown;
  try {
    stored = JSON.parse(content);
  } catch {
    return DEFAULT_SETTINGS;
  }
  if (typeof stored !== 'object' || stored === null) {
    return DEFAULT_SETTINGS;
  }
  const { columns, sort, fieldsChosen } = stored as {
    columns?: unknown;
    sort?: unknown;
    fieldsChosen?: unknown;
  };

  const keptColumns = Array.isArray(columns)
    ? inOrder(columns.filter(isColumnId))
    : DEFAULT_SETTINGS.columns;

  let keptSort: TableSort | null = null;
  if (typeof sort === 'object' && sort !== null) {
    const { column, direction } = sort as { column?: unknown; direction?: unknown };
    if (isColumnId(column) && (direction === 'ascending' || direction === 'descending')) {
      keptSort = { column, direction };
    }
  }
  return {
    columns: keptColumns,
    sort: keptSort,
    ...(fieldsChosen === true ? { fieldsChosen: true } : {}),
  };
}

/** Writes what a table view keeps, for `View.content`. */
export function serializeSettings(settings: TableSettings): string {
  return JSON.stringify({
    version: 1,
    columns: settings.columns,
    sort: settings.sort,
    ...(settings.fieldsChosen ? { fieldsChosen: true } : {}),
  });
}

/** The columns that are shown: those of every task that are, then the fields that are. */
export function shownColumns(
  settings: TableSettings,
  fields: readonly CustomField[] = [],
): Column[] {
  const shown = new Set(settings.columns);
  return columnsOf(fields).filter(
    (column) =>
      column.isFixed ||
      shown.has(column.id) ||
      (fieldIdOf(column.id) !== null && settings.fieldsChosen !== true),
  );
}

/**
 * The settings with a column shown that was hidden, or hidden that was
 * shown. A fixed one stays. The first time a field is turned on or off, the
 * fields shown until then are written down, so that the choice is kept.
 */
export function withColumnToggled(
  settings: TableSettings,
  id: ColumnId,
  fields: readonly CustomField[] = [],
): TableSettings {
  const shown = new Set(shownColumns(settings, fields).map((column) => column.id));
  if (!shown.delete(id)) {
    shown.add(id);
  }
  const columns = inOrder(shown, fields);
  // Rows sorted by a column that is no longer shown would be in an order
  // nobody can see the reason for.
  const sort = settings.sort && columns.includes(settings.sort.column) ? settings.sort : null;
  const fieldsChosen = settings.fieldsChosen === true || fieldIdOf(id) !== null;
  return fieldsChosen ? { columns, sort, fieldsChosen: true } : { columns, sort };
}

/**
 * The settings after a press on the head of a column: sorted by it from the
 * first value on, or, if it already sorts the rows, the other way round.
 */
export function withSortBy(settings: TableSettings, id: ColumnId): TableSettings {
  const sort = settings.sort;
  if (sort?.column === id) {
    return {
      ...settings,
      sort: { column: id, direction: sort.direction === 'ascending' ? 'descending' : 'ascending' },
    };
  }
  return { ...settings, sort: { column: id, direction: 'ascending' } };
}

/** The words for the two ways a column can be sorted, by what its values are. */
const SORT_WORDS: Record<BuiltInColumnId, readonly [ascending: string, descending: string]> = {
  reference: ['oldest first', 'newest first'],
  title: ['A to Z', 'Z to A'],
  status: ['in the order of the workflow', 'the workflow reversed'],
  assignee: ['A to Z', 'Z to A'],
  priority: ['most urgent first', 'least urgent first'],
  points: ['fewest first', 'most first'],
  estimate: ['shortest first', 'longest first'],
  start: ['earliest first', 'latest first'],
  due: ['earliest first', 'latest first'],
  sprint: ['in the order of the sprints', 'the sprints reversed'],
  area: ['in the order of the areas', 'the areas reversed'],
  tags: ['A to Z', 'Z to A'],
  waitsFor: ['fewest first', 'most first'],
};

/** The words for the two ways the column of a field of each kind can be sorted. */
const FIELD_SORT_WORDS: Record<CustomField['kind'], readonly [string, string]> = {
  text: ['A to Z', 'Z to A'],
  number: ['smallest first', 'largest first'],
  date: ['earliest first', 'latest first'],
  yes_no: ['no first', 'yes first'],
  choice: ['in the order of the choices', 'the choices reversed'],
  link: ['A to Z', 'Z to A'],
};

/** Says how the rows are sorted, in a few words: "Due, earliest first". */
export function describeSort(sort: TableSort, fields: readonly CustomField[] = []): string {
  const fieldId = fieldIdOf(sort.column);
  let label: string;
  let words: readonly [string, string];
  if (fieldId === null) {
    label = COLUMNS.find((candidate) => candidate.id === sort.column)?.label ?? sort.column;
    words = SORT_WORDS[sort.column as BuiltInColumnId];
  } else {
    const field = fields.find((candidate) => candidate.id === fieldId);
    label = field?.name ?? 'A field that was removed';
    words = field ? FIELD_SORT_WORDS[field.kind] : ['first to last', 'last to first'];
  }
  const [ascending, descending] = words;
  return `${label}, ${sort.direction === 'ascending' ? ascending : descending}`;
}

/** What a sort needs to know beyond the tasks: the orders and names the project gives. */
export interface SortContext {
  /** The place of each status in the workflow. */
  statusOrder: ReadonlyMap<Id, number>;
  /** The place of each sprint in the list of sprints. */
  sprintOrder: ReadonlyMap<Id, number>;
  /** The place of each area in the list of areas. */
  areaOrder: ReadonlyMap<Id, number>;
  /** The name of whoever a task is given to. */
  assigneeName: (assignee: Assignee) => string;
  /**
   * The names of the tags a task carries, as they read in its cell, or
   * nothing for a task without tags. Left out, no task has any.
   */
  tagNames?: (task: Task) => string | null;
  /** The custom fields of the project, to sort by the column of one. Left out, there are none. */
  fields?: readonly CustomField[];
}

/** The priorities from the most urgent on, as the menus of the program list them. */
const BY_URGENCY: readonly Priority[] = ['urgent', 'high', 'normal', 'low'];

/** The place of a priority from the most urgent on: the first sort puts urgent work first. */
function urgencyOf(priority: Priority): number {
  return BY_URGENCY.indexOf(priority);
}

/** What a task is sorted by in a column. Nothing for an empty cell, which goes last either way. */
export function sortValueOf(
  column: ColumnId,
  task: Task,
  context: SortContext,
): string | number | null {
  switch (column) {
    case 'reference':
      return task.number;
    case 'title':
      return task.title;
    case 'status':
      return context.statusOrder.get(task.statusId) ?? null;
    case 'assignee':
      return task.assignee === null ? null : context.assigneeName(task.assignee);
    case 'priority':
      return urgencyOf(task.priority);
    case 'points':
      return task.points;
    case 'estimate':
      return task.estimateMinutes;
    case 'start':
      return task.startOn;
    case 'due':
      return task.dueOn;
    case 'sprint':
      return task.sprintId === null ? null : (context.sprintOrder.get(task.sprintId) ?? null);
    case 'area':
      return context.areaOrder.get(task.areaId) ?? null;
    case 'tags':
      return context.tagNames?.(task) ?? null;
    case 'waitsFor':
      return task.blockedBy.length === 0 ? null : task.blockedBy.length;
    default: {
      const fieldId = fieldIdOf(column);
      const field = context.fields?.find((candidate) => candidate.id === fieldId);
      return field ? sortValueOfField(field, task) : null;
    }
  }
}

const collator = new Intl.Collator('en', { numeric: true, sensitivity: 'base' });

function compareValues(first: string | number, second: string | number): number {
  if (typeof first === 'number' && typeof second === 'number') {
    return first - second;
  }
  return collator.compare(String(first), String(second));
}

/** The order the manager arranged: by status in the workflow, then as on the board. */
function compareArranged(first: Task, second: Task, context: SortContext): number {
  const firstStatus = context.statusOrder.get(first.statusId) ?? Number.MAX_SAFE_INTEGER;
  const secondStatus = context.statusOrder.get(second.statusId) ?? Number.MAX_SAFE_INTEGER;
  return firstStatus - secondStatus || asArranged(first, second);
}

/**
 * Compares two tasks for a sort. Empty cells go last whichever way the
 * column is sorted, and equal ones keep the arranged order.
 */
export function compareTasks(
  first: Task,
  second: Task,
  sort: TableSort | null,
  context: SortContext,
): number {
  if (sort !== null) {
    const firstValue = sortValueOf(sort.column, first, context);
    const secondValue = sortValueOf(sort.column, second, context);
    if (firstValue === null || secondValue === null) {
      if (firstValue !== secondValue) {
        return firstValue === null ? 1 : -1;
      }
    } else {
      const order = compareValues(firstValue, secondValue);
      if (order !== 0) {
        return sort.direction === 'ascending' ? order : -order;
      }
    }
  }
  return compareArranged(first, second, context);
}

/** One row of the table: a task, and how far it is set in under the tasks it is part of. */
export interface TableRow {
  task: Task;
  depth: number;
}

/**
 * The rows of the table: the tasks that are not part of another, in order,
 * each followed by its subtasks in the same order, to any depth. A subtask
 * whose task is not among the ones given stands on its own.
 */
export function tableRows(
  tasks: readonly Task[],
  sort: TableSort | null,
  context: SortContext,
): TableRow[] {
  const given = new Set(tasks.map((task) => task.id));
  const byParent = new Map<Id | null, Task[]>();
  for (const task of tasks) {
    const parent = task.parentId !== null && given.has(task.parentId) ? task.parentId : null;
    const siblings = byParent.get(parent);
    if (siblings) {
      siblings.push(task);
    } else {
      byParent.set(parent, [task]);
    }
  }

  const rows: TableRow[] = [];
  const visit = (parent: Id | null, depth: number) => {
    const children = [...(byParent.get(parent) ?? [])].sort((first, second) =>
      compareTasks(first, second, sort, context),
    );
    for (const task of children) {
      rows.push({ task, depth });
      visit(task.id, depth + 1);
    }
  };
  visit(null, 0);
  return rows;
}

/** What the foot of the table says. Every row counts, subtasks included. */
export interface TableTotals {
  tasks: number;
  points: number;
  /** The estimates together, in minutes. */
  minutes: number;
  /** How many rows have no points, and how many no estimate. */
  withoutPoints: number;
  withoutEstimate: number;
}

export function totalsOf(tasks: readonly Task[]): TableTotals {
  return {
    tasks: tasks.length,
    points: tasks.reduce((sum, task) => sum + (task.points ?? 0), 0),
    minutes: tasks.reduce((sum, task) => sum + (task.estimateMinutes ?? 0), 0),
    withoutPoints: tasks.filter((task) => task.points === null).length,
    withoutEstimate: tasks.filter((task) => task.estimateMinutes === null).length,
  };
}

/** Writes a sum of working time in hours, the unit a manager plans in: "28h", "3.5h". */
export function formatHours(minutes: number): string {
  const hours = Math.round((minutes / 60) * 10) / 10;
  return `${hours.toString()}h`;
}

/** Where a cell of the body stands: its row and its column, both counted from 0. */
export interface CellPosition {
  row: number;
  column: number;
}

/** How far Page Up and Page Down move. */
const PAGE = 10;

/**
 * The cell a key leads to, kept within the table, or nothing if the key
 * does not move. The arrows move by one; Home and End go to the first and
 * last column of the row, and with Ctrl to the first and last cell of the
 * table; Page Up and Page Down move by ten rows.
 */
export function nextCell(
  from: CellPosition,
  key: string,
  withCtrl: boolean,
  size: { rows: number; columns: number },
): CellPosition | null {
  if (size.rows === 0 || size.columns === 0) {
    return null;
  }
  const lastRow = size.rows - 1;
  const lastColumn = size.columns - 1;
  const clamp = (row: number, column: number): CellPosition => ({
    row: Math.min(Math.max(row, 0), lastRow),
    column: Math.min(Math.max(column, 0), lastColumn),
  });
  switch (key) {
    case 'ArrowUp':
      return clamp(from.row - 1, from.column);
    case 'ArrowDown':
      return clamp(from.row + 1, from.column);
    case 'ArrowLeft':
      return clamp(from.row, from.column - 1);
    case 'ArrowRight':
      return clamp(from.row, from.column + 1);
    case 'Home':
      return clamp(withCtrl ? 0 : from.row, 0);
    case 'End':
      return clamp(withCtrl ? lastRow : from.row, lastColumn);
    case 'PageUp':
      return clamp(from.row - PAGE, from.column);
    case 'PageDown':
      return clamp(from.row + PAGE, from.column);
    default:
      return null;
  }
}

/** The value of the assignee menu that stands for "nobody". */
export const UNASSIGNED = 'unassigned';
/** The value of the assignee menu that stands for the manager. */
export const MANAGER = 'manager';

/** The value of the assignee menu that stands for an assignee. */
export function assigneeValue(assignee: Assignee | null): string {
  if (assignee === null) {
    return UNASSIGNED;
  }
  return assignee.kind === 'manager' ? MANAGER : assignee.memberId;
}

/** The assignee a value of the assignee menu stands for. */
export function assigneeOf(value: string): Assignee | null {
  if (value === UNASSIGNED) {
    return null;
  }
  return value === MANAGER ? { kind: 'manager' } : { kind: 'member', memberId: value };
}
