import type { Color, Id, Tag, Task } from '@/shared/api/types';

/**
 * What the tags of a project look like on screen, worked out without
 * drawing anything: which tags a task carries, which of them fit on a card,
 * and which match what the manager types.
 */

/** The colors a tag can have, in the order the dialog offers them. */
export const TAG_COLORS: readonly Color[] = [
  'blue',
  'green',
  'amber',
  'purple',
  'teal',
  'pink',
  'red',
  'gray',
];

/** The names of the colors, for people. */
const COLOR_NAMES: Record<Color, string> = {
  gray: 'Gray',
  blue: 'Blue',
  purple: 'Purple',
  amber: 'Amber',
  green: 'Green',
  red: 'Red',
  pink: 'Pink',
  teal: 'Teal',
};

/** The name of a color, for people. */
export function colorName(color: Color): string {
  return COLOR_NAMES[color];
}

/** Two tag names are the same whatever the capitals, as the backend compares them. */
function sameName(first: string, second: string): boolean {
  return first.trim().toLowerCase() === second.trim().toLowerCase();
}

/**
 * The tags a task carries, in the order the tags of the project are listed:
 * by name. A tag the project no longer has is left out.
 */
export function tagsOf(task: Pick<Task, 'tagIds'>, tags: readonly Tag[]): Tag[] {
  return tags.filter((tag) => task.tagIds.includes(tag.id));
}

/**
 * The tags that fit in a row of so many, and how many are left over for a
 * "+N". All of them fit if there are no more than `most`; otherwise one
 * place goes to the count.
 */
export function fittingTags(
  tags: readonly Tag[],
  most: number,
): { shown: readonly Tag[]; more: number } {
  if (tags.length <= most) {
    return { shown: tags, more: 0 };
  }
  const shown = tags.slice(0, Math.max(most - 1, 0));
  return { shown, more: tags.length - shown.length };
}

/**
 * The tags that match what the manager typed, by a part of their name,
 * whatever the capitals. Nothing typed matches every tag.
 */
export function matchingTags(tags: readonly Tag[], typed: string): Tag[] {
  const wanted = typed.trim().toLowerCase();
  return tags.filter((tag) => tag.name.toLowerCase().includes(wanted));
}

/**
 * Whether what the manager typed would make a new tag: it says something,
 * and no tag of the project has that name already.
 */
export function makesNewTag(tags: readonly Tag[], typed: string): boolean {
  return typed.trim() !== '' && !tags.some((tag) => sameName(tag.name, typed));
}

/** How many tasks carry each tag, by its identifier. */
export function taskCountsByTag(tasks: readonly Pick<Task, 'tagIds'>[]): ReadonlyMap<Id, number> {
  const counts = new Map<Id, number>();
  for (const task of tasks) {
    for (const tagId of task.tagIds) {
      counts.set(tagId, (counts.get(tagId) ?? 0) + 1);
    }
  }
  return counts;
}

/** A task as it is once it carries a tag, or no longer does. */
export function withTagSet<Tagged extends Pick<Task, 'tagIds'>>(
  task: Tagged,
  tagId: Id,
  isOn: boolean,
): Tagged {
  const others = task.tagIds.filter((id) => id !== tagId);
  return { ...task, tagIds: isOn ? [...others, tagId] : others };
}
