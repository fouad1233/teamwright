import type { Color, Status, StatusCategory, StatusProperties } from '@/shared/api/types';

/**
 * Pure functions about the workflow of a project, for the dialog in which
 * its statuses are edited. The rules of a workflow are the backend's; what
 * is here only helps the dialog offer sensible things and show a change
 * before the backend has answered.
 */

/** What a category of status means, in the words a manager reads. */
interface CategoryDescription {
  category: StatusCategory;
  /** The name of the category in a menu. */
  label: string;
  /** What it says about a task that is in such a status. */
  meaning: string;
}

/** The categories a status can have, in the order of the course of the work. */
export const CATEGORIES: readonly CategoryDescription[] = [
  { category: 'todo', label: 'Not started', meaning: 'Work on the task has not started.' },
  { category: 'active', label: 'In progress', meaning: 'Work on the task is under way.' },
  { category: 'review', label: 'In review', meaning: 'The work waits to be checked.' },
  { category: 'done', label: 'Done', meaning: 'The task is finished.' },
];

/** The name of a category in the words a manager reads. */
export function categoryLabel(category: StatusCategory): string {
  return CATEGORIES.find((known) => known.category === category)?.label ?? category;
}

/** The pigments a status can have, in the order they are offered. */
export const STATUS_COLORS: readonly Color[] = [
  'gray',
  'blue',
  'purple',
  'teal',
  'green',
  'amber',
  'pink',
  'red',
];

/** The names of the pigments, for people who cannot see them. */
const COLOR_NAMES: Record<Color, string> = {
  gray: 'Gray',
  blue: 'Blue',
  purple: 'Purple',
  teal: 'Teal',
  green: 'Green',
  amber: 'Amber',
  pink: 'Pink',
  red: 'Red',
};

/** The name of a pigment. */
export function colorName(color: Color): string {
  return COLOR_NAMES[color];
}

/**
 * A copy of the statuses with one moved to another place, the way the
 * backend will move it. `index` counts the other statuses from the first.
 */
export function withMovedStatus(
  statuses: readonly Status[],
  statusId: string,
  index: number,
): Status[] {
  const moving = statuses.find((status) => status.id === statusId);
  if (!moving) {
    return [...statuses];
  }
  const others = statuses.filter((status) => status.id !== statusId);
  others.splice(Math.min(Math.max(index, 0), others.length), 0, moving);
  return others;
}

const NEW_STATUS_NAME = 'New status';

/**
 * What a new status starts as, until the manager says otherwise: a name no
 * status of the workflow has, a step of the work under way, and a pigment
 * that no status uses yet if there is one.
 */
export function suggestedStatus(statuses: readonly Status[]): StatusProperties {
  const taken = new Set(statuses.map((status) => status.name.toLowerCase()));
  let name = NEW_STATUS_NAME;
  for (let number = 2; taken.has(name.toLowerCase()); number += 1) {
    name = `${NEW_STATUS_NAME} ${number.toString()}`;
  }

  const used = new Set(statuses.map((status) => status.color));
  // Gray stands for work that has not started and red for trouble, so a new
  // step of the work gets neither unless nothing else is free.
  const preferred = STATUS_COLORS.filter((color) => color !== 'gray' && color !== 'red');
  const color = preferred.find((candidate) => !used.has(candidate)) ?? 'blue';

  return { name, category: 'active', color };
}

/**
 * Tells whether a status is the only one of a category that a workflow
 * cannot do without: the only one in which work has not started, or the
 * only one in which it is done. The backend refuses to remove such a
 * status or to give it another category; the dialog uses this to say so
 * beforehand instead of letting the manager run into the refusal.
 */
export function isTheOnlyOneOfItsKind(statuses: readonly Status[], status: Status): boolean {
  const isRequired = status.category === 'todo' || status.category === 'done';
  return isRequired && statuses.filter((other) => other.category === status.category).length === 1;
}
