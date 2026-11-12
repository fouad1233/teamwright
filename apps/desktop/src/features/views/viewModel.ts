import type {
  Assignee,
  Id,
  Member,
  Status,
  Task,
  View,
  ViewFilter,
  ViewKind,
} from '@/shared/api/types';

/**
 * What the row of views works out without drawing anything: which tasks a
 * filter shows, how a filter reads in words, which kinds can still be added,
 * and how many tabs fit in the row.
 */

/** The kinds of view that can be filtered. The backend refuses a filter on the others. */
const FILTERABLE_KINDS: readonly ViewKind[] = ['board', 'list', 'gantt', 'calendar'];

/** The kinds of which a project has at most one view. The backend refuses a second. */
const SINGLE_KINDS: readonly ViewKind[] = ['backlog', 'dashboard'];

/** The filter of a view that shows every task. */
export const EVERY_TASK: ViewFilter = { assignee: { kind: 'anyone' }, hidesFinished: false };

/** Whether a view of a kind can be filtered. */
export function isFilterable(kind: ViewKind): boolean {
  return FILTERABLE_KINDS.includes(kind);
}

/** The kinds the manager can add: every kind, except one the project may have once and has. */
export function kindsToOffer(kinds: readonly ViewKind[], views: readonly View[]): ViewKind[] {
  return kinds.filter(
    (kind) => !SINGLE_KINDS.includes(kind) || !views.some((view) => view.kind === kind),
  );
}

/** Whether a filter leaves any task out. */
export function isFiltered(filter: ViewFilter): boolean {
  return filter.assignee.kind !== 'anyone' || filter.hidesFinished;
}

/**
 * The tasks a filter shows, by their identifiers, or nothing when it shows
 * them all. A task is shown when it passes the filter or one of its subtasks
 * does, and its subtasks are shown with it: a task is never taken apart.
 * Finished means a status of the category done, whatever it is called.
 */
export function shownTaskIds(
  tasks: readonly Task[],
  statuses: readonly Status[],
  filter: ViewFilter,
): ReadonlySet<Id> | null {
  if (!isFiltered(filter)) {
    return null;
  }
  const doneStatuses = new Set(
    statuses.filter((status) => status.category === 'done').map((status) => status.id),
  );
  const passes = (task: Task) =>
    isGivenAsFiltered(task, filter) && !(filter.hidesFinished && doneStatuses.has(task.statusId));

  const byId = new Map(tasks.map((task) => [task.id, task]));
  const rootOf = (task: Task): Id => {
    let current = task;
    const seen = new Set<Id>();
    while (current.parentId !== null && !seen.has(current.id)) {
      seen.add(current.id);
      const parent = byId.get(current.parentId);
      if (!parent) {
        break;
      }
      current = parent;
    }
    return current.id;
  };

  const shownRoots = new Set(tasks.filter(passes).map(rootOf));
  return new Set(tasks.filter((task) => shownRoots.has(rootOf(task))).map((task) => task.id));
}

/**
 * Who a task opened in a filtered view is given to, so that it does not
 * vanish the moment it is made: the member or the manager the filter shows.
 * Nothing for a view that shows everybody's tasks or nobody's, where a new
 * task given to nobody stays in view.
 */
export function assigneeForNewTasks(filter: ViewFilter): Assignee | undefined {
  const wanted = filter.assignee;
  switch (wanted.kind) {
    case 'manager':
      return { kind: 'manager' };
    case 'member':
      return { kind: 'member', memberId: wanted.memberId };
    case 'anyone':
    case 'nobody':
      return undefined;
  }
}

/** Whether a task is given to the one the filter asks for. */
function isGivenAsFiltered(task: Task, filter: ViewFilter): boolean {
  const wanted = filter.assignee;
  switch (wanted.kind) {
    case 'anyone':
      return true;
    case 'manager':
      return task.assignee?.kind === 'manager';
    case 'member':
      return task.assignee?.kind === 'member' && task.assignee.memberId === wanted.memberId;
    case 'nobody':
      return task.assignee === null;
  }
}

/**
 * A filter in one sentence, for the top of the view: "Shows the tasks of
 * Firmware engineer". Nothing for a filter that shows every task.
 */
export function filterInWords(filter: ViewFilter, members: readonly Member[]): string | null {
  if (!isFiltered(filter)) {
    return null;
  }
  const tasks = filter.hidesFinished ? 'unfinished tasks' : 'tasks';
  const wanted = filter.assignee;
  switch (wanted.kind) {
    case 'anyone':
      return 'Hides the finished tasks';
    case 'manager':
      return `Shows your ${tasks}. New ones are yours`;
    case 'member': {
      const name = members.find((member) => member.id === wanted.memberId)?.name;
      return name === undefined
        ? `Shows the ${tasks} of a member who has left the team`
        : `Shows the ${tasks} of ${name}. New ones go to them`;
    }
    case 'nobody':
      return `Shows the ${tasks} given to nobody`;
  }
}

/**
 * The view to show: the one asked for if the project still has it, and
 * otherwise the first view of the project.
 */
export function viewToShow(views: readonly View[], viewId: Id | null): View | undefined {
  return views.find((view) => view.id === viewId) ?? views[0];
}

/** The views as they are after one of them has moved to a place. Index 0 is the first. */
export function withMovedView(views: readonly View[], viewId: Id, index: number): View[] {
  const moving = views.find((view) => view.id === viewId);
  if (!moving) {
    return [...views];
  }
  const others = views.filter((view) => view.id !== viewId);
  others.splice(Math.min(Math.max(index, 0), others.length), 0, moving);
  return others;
}

/** What is known about the row of view tabs when it is laid out. */
export interface RowMeasures {
  /** The width of the tab of every view, in the order of the views. */
  widths: readonly number[];
  /** The room there is for the tabs of the views and the button for the rest. */
  available: number;
  /** The space between two tabs. */
  gap: number;
  /** The width of the button that opens the views that do not fit: "3 more". */
  moreWidth: number;
  /** The place of the view that is open, which is always shown. Nothing if none is. */
  openIndex: number | null;
}

/**
 * Which views get a tab in the row and which are behind "N more", by their
 * places. The views keep their order. If the open view would be behind the
 * button, it takes the place of the last tab that fits, so the manager always
 * sees where they are.
 */
export function splitRow({ widths, available, gap, moreWidth, openIndex }: RowMeasures): {
  shown: number[];
  hidden: number[];
} {
  const all = widths.map((_, index) => index);
  const widthOf = (indices: readonly number[]) =>
    indices.reduce((sum, index) => sum + (widths[index] ?? 0) + gap, 0);
  if (widthOf(all) - gap <= available) {
    return { shown: all, hidden: [] };
  }

  const room = available - moreWidth;
  let shown: number[] = [];
  for (const index of all) {
    if (widthOf([...shown, index]) > room) {
      break;
    }
    shown.push(index);
  }
  if (openIndex !== null && !shown.includes(openIndex)) {
    shown = [...shown, openIndex];
    while (shown.length > 1 && widthOf(shown) > room) {
      shown.splice(shown.length - 2, 1);
    }
    shown.sort((first, second) => first - second);
  }
  return { shown, hidden: all.filter((index) => !shown.includes(index)) };
}
