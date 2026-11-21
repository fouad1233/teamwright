import type {
  NamedTask,
  PlannedLoad,
  Project,
  SprintForecast,
  SprintUnit,
  Velocity,
} from '@/shared/api/types';
import { taskReference } from '@/shared/lib/format';

import { formatWork } from './sprintModel';

/**
 * Puts the forecast of a project into words. The numbers and the rules are
 * the backend's (`SprintService::forecast`); this only says them.
 */

/** How many tasks without points are named before the rest are counted. */
const MOST_NAMED = 4;

/** The range of the pace: "5–11 points". */
function rangeOf(velocity: Velocity & { state: 'known' }, unit: SprintUnit): string {
  return velocity.low === velocity.high
    ? formatWork(velocity.low, unit)
    : `${velocity.low.toString()}–${formatWork(velocity.high, unit)}`;
}

/**
 * The pace of the team in a sentence, or nothing before any sprint was
 * completed: "The team finished 5–11 points a sprint over its last 3
 * sprints." With fewer than three it says that it is too early.
 */
export function paceInWords(forecast: SprintForecast): string | null {
  const { velocity, unit } = forecast;
  switch (velocity.state) {
    case 'no_sprints':
      return null;
    case 'too_early': {
      const count = velocity.finished.length;
      return `Too early to tell a pace: ${count.toString()} ${count === 1 ? 'sprint' : 'sprints'} completed, 3 are needed.`;
    }
    case 'known':
      return `The team finished ${rangeOf(velocity, unit)} a sprint over its last ${velocity.sprints.toString()} sprints.`;
  }
}

/**
 * How a planned sprint compares with the pace, as a fact next to its dates:
 * "more than the team has finished (5–11 points)". Nothing while no pace
 * can be told.
 */
export function loadInWords(load: PlannedLoad, forecast: SprintForecast): string | null {
  const { velocity, unit } = forecast;
  if (velocity.state !== 'known' || load.load === null) {
    return null;
  }
  const range = rangeOf(velocity, unit);
  switch (load.load) {
    case 'above':
      return `more than the team has finished in a sprint (${range})`;
    case 'within':
      return `within what the team finishes (${range})`;
    case 'below':
      return `less than the team finishes (${range})`;
  }
}

/**
 * The rest of the work and how long it would take at the pace: "The work
 * not finished, 34 points in 12 tasks, would take 4 to 7 sprints at this
 * pace." Without a pace it only says what is left.
 */
export function aheadInWords(forecast: SprintForecast): string {
  const { ahead, unit, velocity } = forecast;
  const tasks = formatWork(ahead.tasks, 'tasks');
  if (ahead.tasks === 0) {
    return 'Every task is finished.';
  }
  const what =
    unit === 'points'
      ? `The work not finished, ${formatWork(ahead.work, 'points')} in ${tasks},`
      : `The work not finished, ${tasks},`;
  if (velocity.state !== 'known' || ahead.fewestSprints === null) {
    return `${what} cannot be forecast until the pace is known.`;
  }
  const sprints = (count: number) => `${count.toString()} ${count === 1 ? 'sprint' : 'sprints'}`;
  if (ahead.mostSprints === null) {
    return `${what} would take at least ${sprints(ahead.fewestSprints)} at this pace; a sprint that finished nothing leaves no upper end.`;
  }
  if (ahead.mostSprints === ahead.fewestSprints) {
    return `${what} would take about ${sprints(ahead.fewestSprints)} at this pace.`;
  }
  return `${what} would take ${ahead.fewestSprints.toString()} to ${sprints(ahead.mostSprints)} at this pace.`;
}

/**
 * The tasks a forecast cannot count, named: "2 tasks have no points and are
 * not counted: SSN-12, SSN-21." Nothing when there are none.
 */
export function withoutPointsInWords(
  project: Pick<Project, 'key'>,
  tasks: readonly NamedTask[],
): string | null {
  if (tasks.length === 0) {
    return null;
  }
  const named = tasks.slice(0, MOST_NAMED).map((task) => taskReference(project, task));
  const rest = tasks.length - named.length;
  const list = rest > 0 ? `${named.join(', ')} and ${rest.toString()} more` : named.join(', ');
  return tasks.length === 1
    ? `1 task has no points and is not counted: ${list}.`
    : `${tasks.length.toString()} tasks have no points and are not counted: ${list}.`;
}
