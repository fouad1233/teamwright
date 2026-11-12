import { BackendError } from '../backend';
import type { Area, Id } from '../types';

/**
 * The rules about the areas of a project that concern the areas alone, for
 * the demo backend. The real application is to have them in its domain
 * crate.
 */

/** The name of the area every project starts with. */
export const FIRST_AREA_NAME = 'General';

/** The most characters the name of an area can have. */
const AREA_NAME_MAX_LENGTH = 40;

/** The area a new project starts with. */
export function startingArea(projectId: Id, id: Id): Area {
  return { id, projectId, name: FIRST_AREA_NAME };
}

/** Tidies the name of an area: trimmed and on one line, not empty and not too long. */
export function tidyAreaName(raw: string): string {
  const name = raw.trim().split(/\s+/).join(' ');
  if (name === '') {
    throw new BackendError('rule_broken', 'the area name must not be empty');
  }
  const length = [...name].length;
  if (length > AREA_NAME_MAX_LENGTH) {
    throw new BackendError(
      'rule_broken',
      `the area name can have at most ${AREA_NAME_MAX_LENGTH.toString()} characters, but this one has ${length.toString()}`,
    );
  }
  return name;
}

/**
 * Refuses a name that another area of the project has. Capital and small
 * letters count as the same.
 */
export function refuseAreaNameInUse(others: readonly Area[], name: string): void {
  const wanted = name.toLowerCase();
  if (others.some((area) => area.name.toLowerCase() === wanted)) {
    throw new BackendError('area_name_taken', `the project already has an area called "${name}"`);
  }
}

/** Refuses to remove the one area a project has left. */
export function refuseRemovingLastArea(area: Area, areas: readonly Area[]): void {
  if (areas.length <= 1) {
    throw new BackendError(
      'last_area',
      `"${area.name}" is the last area of the project and cannot be removed`,
    );
  }
}

/** Refuses to remove an area that holds tasks without saying where they go. */
export function refuseAreaInUse(area: Area, taskCount: number): never {
  throw new BackendError(
    'area_in_use',
    `the area "${area.name}" still has ${taskCount.toString()} ${taskCount === 1 ? 'task' : 'tasks'}, which need another area to go to`,
  );
}
