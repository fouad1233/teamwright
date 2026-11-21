import { BackendError } from '../backend';
import type { ChecklistItem, Color, Tag } from '../types';

/**
 * The rules about tags and checklists that concern one tag, one name or
 * one checklist, for the demo backend. The real application has them in
 * its domain crate (`tag.rs`, `checklist.rs`).
 */

/** The most characters the name of a tag can have. */
const TAG_NAME_MAX_LENGTH = 30;

/** The most tags a project may have. */
export const MOST_TAGS_IN_A_PROJECT = 100;

/** The most characters an item of a checklist can have. */
const CHECKLIST_TEXT_MAX_LENGTH = 200;

/** The most items a checklist may have. */
export const MOST_CHECKLIST_ITEMS = 100;

/** The colors a new tag gets in turn, when the manager names none. */
export const TAG_COLORS_IN_TURN: readonly Color[] = [
  'blue',
  'green',
  'amber',
  'purple',
  'teal',
  'pink',
  'red',
  'gray',
];

/** The color of a new tag, given how many tags the project has already. */
export function tagColorInTurn(tagsSoFar: number): Color {
  return TAG_COLORS_IN_TURN[tagsSoFar % TAG_COLORS_IN_TURN.length] ?? 'blue';
}

/** Tidies a text: trimmed and on one line, not empty and not too long. */
function tidy(raw: string, field: string, maxLength: number): string {
  const text = raw.trim().split(/\s+/).join(' ');
  if (text === '') {
    throw new BackendError('rule_broken', `the ${field} must not be empty`);
  }
  const length = [...text].length;
  if (length > maxLength) {
    throw new BackendError(
      'rule_broken',
      `the ${field} can have at most ${maxLength.toString()} characters, but this one has ${length.toString()}`,
    );
  }
  return text;
}

/** Tidies the name of a tag. */
export function tidyTagName(raw: string): string {
  return tidy(raw, 'tag name', TAG_NAME_MAX_LENGTH);
}

/** Tidies the text of an item of a checklist. */
export function tidyChecklistText(raw: string): string {
  return tidy(raw, 'checklist item', CHECKLIST_TEXT_MAX_LENGTH);
}

/**
 * Refuses a name that another tag of the project has. Capital and small
 * letters count as the same.
 */
export function refuseTagNameInUse(others: readonly Tag[], name: string): void {
  const wanted = name.toLowerCase();
  if (others.some((tag) => tag.name.toLowerCase() === wanted)) {
    throw new BackendError('tag_name_taken', `the project already has a tag called "${name}"`);
  }
}

/** Refuses a tag that would be one too many. */
export function refuseTooManyTags(count: number): void {
  if (count >= MOST_TAGS_IN_A_PROJECT) {
    throw new BackendError(
      'rule_broken',
      `a project has at most ${MOST_TAGS_IN_A_PROJECT.toString()} tags`,
    );
  }
}

/** Refuses an item that would be one too many. */
export function refuseFullChecklist(items: readonly ChecklistItem[]): void {
  if (items.length >= MOST_CHECKLIST_ITEMS) {
    throw new BackendError(
      'rule_broken',
      `a checklist has at most ${MOST_CHECKLIST_ITEMS.toString()} items`,
    );
  }
}

/** The tags of a project by name, whatever the capitals. */
export function byName(tags: readonly Tag[]): Tag[] {
  return [...tags].sort(
    (first, second) =>
      first.name.toLowerCase().localeCompare(second.name.toLowerCase()) ||
      first.id.localeCompare(second.id),
  );
}

/** The item of a checklist with an identifier, or the refusal the backend gives. */
export function requireItem(checklist: readonly ChecklistItem[], itemId: string): ChecklistItem {
  const item = checklist.find((candidate) => candidate.id === itemId);
  if (!item) {
    throw new BackendError('not_found', 'the checklist of the task has no such item');
  }
  return item;
}
