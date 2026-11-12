import { BackendError } from '../backend';
import type { Id, View, ViewFilter, ViewKind } from '../types';

/**
 * The rules about the views of a project that concern the views alone, for
 * the demo backend. The real application is to have them in its domain
 * crate.
 */

/** The kinds of view, in the order a new project gets them. */
export const VIEW_KINDS: readonly ViewKind[] = [
  'board',
  'backlog',
  'list',
  'gantt',
  'calendar',
  'dashboard',
];

/** The name a view gets from its kind. */
const NAME_OF_KIND: Record<ViewKind, string> = {
  board: 'Board',
  backlog: 'Backlog',
  list: 'List',
  gantt: 'Gantt',
  calendar: 'Calendar',
  dashboard: 'Dashboard',
};

/** The kinds of which a project has at most one view. */
const SINGLE_KINDS: readonly ViewKind[] = ['backlog', 'dashboard'];

/** The kinds of view that can be filtered. */
const FILTERED_KINDS: readonly ViewKind[] = ['board', 'list', 'gantt', 'calendar'];

/** The most characters the name of a view can have. */
const VIEW_NAME_MAX_LENGTH = 40;

/** The filter of a view that shows every task. */
export const EVERY_TASK: ViewFilter = { assignee: { kind: 'anyone' }, hidesFinished: false };

/** The views a new project starts with: one of each kind, named after it. */
export function startingViews(projectId: Id, newId: () => Id): View[] {
  return VIEW_KINDS.map((kind) => ({
    id: newId(),
    projectId,
    kind,
    name: NAME_OF_KIND[kind],
    filter: structuredClone(EVERY_TASK),
  }));
}

/** Tidies the name of a view: trimmed and on one line, not empty and not too long. */
export function tidyViewName(raw: string): string {
  const name = raw.trim().split(/\s+/).join(' ');
  if (name === '') {
    throw new BackendError('rule_broken', 'the view name must not be empty');
  }
  const length = [...name].length;
  if (length > VIEW_NAME_MAX_LENGTH) {
    throw new BackendError(
      'rule_broken',
      `the view name can have at most ${VIEW_NAME_MAX_LENGTH.toString()} characters, but this one has ${length.toString()}`,
    );
  }
  return name;
}

/**
 * Refuses a name that another view of the project has. Capital and small
 * letters count as the same.
 */
export function refuseViewNameInUse(others: readonly View[], name: string): void {
  const wanted = name.toLowerCase();
  if (others.some((view) => view.name.toLowerCase() === wanted)) {
    throw new BackendError('view_name_taken', `the project already has a view called "${name}"`);
  }
}

/**
 * The name a new view gets when none is given: the name of its kind, or
 * that name with the first number from 2 on that no view has.
 */
export function nameForNewView(kind: ViewKind, views: readonly View[]): string {
  const taken = new Set(views.map((view) => view.name.toLowerCase()));
  const base = NAME_OF_KIND[kind];
  if (!taken.has(base.toLowerCase())) {
    return base;
  }
  for (let number = 2; ; number += 1) {
    const candidate = `${base} ${number.toString()}`;
    if (!taken.has(candidate.toLowerCase())) {
      return candidate;
    }
  }
}

/** Refuses a second backlog or a second dashboard. */
export function refuseSecondOfKind(views: readonly View[], kind: ViewKind): void {
  if (!SINGLE_KINDS.includes(kind)) {
    return;
  }
  const existing = views.find((view) => view.kind === kind);
  if (existing) {
    throw new BackendError(
      'view_kind_taken',
      `the project already has a ${kind}, "${existing.name}", and can have only one`,
    );
  }
}

/** Refuses to filter a backlog or a dashboard: they show every task. */
export function refuseFilterOf(view: View): void {
  if (!FILTERED_KINDS.includes(view.kind)) {
    throw new BackendError(
      'rule_broken',
      `"${view.name}" is a ${view.kind}, which shows every task and cannot be filtered`,
    );
  }
}

/** Refuses to remove the one view a project has left. */
export function refuseRemovingLast(view: View, views: readonly View[]): void {
  if (views.length <= 1) {
    throw new BackendError(
      'last_view',
      `"${view.name}" is the last view of the project and cannot be removed`,
    );
  }
}

/** A filter as it is kept: only the fields it has, so that nothing else travels with it. */
export function tidyFilter(filter: ViewFilter): ViewFilter {
  const assignee = filter.assignee;
  return {
    assignee:
      assignee.kind === 'member'
        ? { kind: 'member', memberId: assignee.memberId }
        : { kind: assignee.kind },
    hidesFinished: filter.hidesFinished,
  };
}
