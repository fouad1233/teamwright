import { BackendError } from '../backend';
import copy from '../projectTemplates.json';
import type {
  Color,
  Fill,
  ProjectTemplate,
  Status,
  StatusCategory,
  StatusLoad,
  Task,
  ViewKind,
  WorkflowTemplate,
} from '../types';

/*
 * The templates of a new project and the rules about work-in-progress
 * limits, for the demo backend, as the program has them (decision 0033).
 * The templates are the program's own: `projectTemplates.json` is a copy
 * that a test of the desktop shell keeps equal to the set in
 * `crates/domain`.
 */

/** The templates, in the order the manager is shown them. */
export const PROJECT_TEMPLATES: readonly ProjectTemplate[] = copy.templates.map((template) => ({
  ...template,
  key: template.key as WorkflowTemplate,
  views: template.views as ViewKind[],
  statuses: template.statuses.map((status) => ({
    ...status,
    category: status.category as StatusCategory,
    color: status.color as Color,
  })),
}));

/** The smallest and the largest work-in-progress limit. */
export const LIMIT_RANGE: { min: number; max: number } = copy.limitRange;

/** Finds a template by its key, or refuses with `rule_broken`. */
export function requireTemplate(key: WorkflowTemplate): ProjectTemplate {
  const found = PROJECT_TEMPLATES.find((template) => template.key === key);
  if (found === undefined) {
    throw new BackendError('rule_broken', `there is no project template ${String(key)}`);
  }
  return found;
}

/**
 * Checks a limit a status is to get: a whole number in its range, or `null`
 * for none. A status of the category done takes none.
 */
export function checkedLimit(status: Status, limit: number | null): number | null {
  if (limit === null) {
    return null;
  }
  if (!Number.isInteger(limit) || limit < LIMIT_RANGE.min || limit > LIMIT_RANGE.max) {
    throw new BackendError(
      'rule_broken',
      `a limit is a whole number of tasks from ${LIMIT_RANGE.min.toString()} to ${LIMIT_RANGE.max.toString()}`,
    );
  }
  if (status.category === 'done') {
    throw new BackendError(
      'rule_broken',
      `"${status.name}" is a status in which tasks are done, and such a status has no limit`,
    );
  }
  return limit;
}

/** How full a status with a limit is. */
function fillOf(count: number, limit: number): Fill {
  if (count < limit) {
    return 'under';
  }
  return count === limit ? 'at' : 'over';
}

/**
 * How many tasks stand in each status, against its limit, in board order.
 * Every task counts, wherever it is shown; a subtask does not count apart
 * from its task.
 */
export function statusLoads(statuses: readonly Status[], tasks: readonly Task[]): StatusLoad[] {
  return statuses.map((status) => {
    const count = tasks.filter(
      (task) => task.statusId === status.id && task.parentId === null,
    ).length;
    const limit = status.limit ?? null;
    return {
      statusId: status.id,
      count,
      limit,
      fill: limit === null ? null : fillOf(count, limit),
    };
  });
}
