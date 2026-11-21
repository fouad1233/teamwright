import type { ProjectChange, Task, TaskChange } from '../types';

/**
 * Tells what changed between two versions of a task, in the order the
 * history lists changes that happened at the same moment.
 *
 * The real application reads the versions of a task from its log of
 * operations and compares them the same way (`history.rs` in the application
 * crate). A change of the rank alone, which only moves a card up or down
 * within its status, is not part of the history.
 */
export function changesBetween(before: Task, after: Task): TaskChange[] {
  const at = after.updatedAt;
  const changes: TaskChange[] = [];

  if (before.title !== after.title) {
    changes.push({ at, kind: 'renamed', from: before.title, to: after.title });
  }
  if (before.description !== after.description) {
    changes.push({ at, kind: 'described' });
  }
  if (before.statusId !== after.statusId) {
    changes.push({ at, kind: 'moved', fromStatusId: before.statusId, toStatusId: after.statusId });
  }
  if (JSON.stringify(before.assignee) !== JSON.stringify(after.assignee)) {
    changes.push({ at, kind: 'assigned', from: before.assignee, to: after.assignee });
  }
  if (before.priority !== after.priority) {
    changes.push({ at, kind: 'prioritized', from: before.priority, to: after.priority });
  }
  if (before.estimateMinutes !== after.estimateMinutes) {
    changes.push({
      at,
      kind: 'estimated',
      from: before.estimateMinutes,
      to: after.estimateMinutes,
    });
  }
  if (before.sprintId !== after.sprintId) {
    changes.push({
      at,
      kind: 'planned',
      fromSprintId: before.sprintId,
      toSprintId: after.sprintId,
    });
  }
  if (before.points !== after.points) {
    changes.push({ at, kind: 'pointed', from: before.points, to: after.points });
  }
  if (before.startOn !== after.startOn || before.dueOn !== after.dueOn) {
    changes.push({
      at,
      kind: 'rescheduled',
      from: { startOn: before.startOn, dueOn: before.dueOn },
      to: { startOn: after.startOn, dueOn: after.dueOn },
    });
  }
  for (const blockerId of after.blockedBy) {
    if (!before.blockedBy.includes(blockerId)) {
      changes.push({ at, kind: 'now_waits_for', taskId: blockerId });
    }
  }
  for (const blockerId of before.blockedBy) {
    if (!after.blockedBy.includes(blockerId)) {
      changes.push({ at, kind: 'no_longer_waits_for', taskId: blockerId });
    }
  }
  for (const tagId of after.tagIds) {
    if (!before.tagIds.includes(tagId)) {
      changes.push({ at, kind: 'tagged', tagId });
    }
  }
  for (const tagId of before.tagIds) {
    if (!after.tagIds.includes(tagId)) {
      changes.push({ at, kind: 'untagged', tagId });
    }
  }
  // Adding, renaming, moving and removing an item are not part of the
  // history; ticking it is, because that is work done.
  for (const item of after.checklist) {
    const earlier = before.checklist.find((candidate) => candidate.id === item.id);
    if (earlier && earlier.done !== item.done) {
      changes.push({ at, kind: item.done ? 'ticked' : 'unticked', item: item.text });
    }
  }
  return changes;
}

/**
 * The history of a task from its versions, the oldest first: that it was
 * opened, and then every change from one version to the next.
 */
export function historyOf(versions: readonly Task[]): TaskChange[] {
  const [first, ...later] = versions;
  if (!first) {
    return [];
  }
  const history: TaskChange[] = [{ at: first.createdAt, kind: 'created' }];
  let previous = first;
  for (const version of later) {
    history.push(...changesBetween(previous, version));
    previous = version;
  }
  return history;
}

/** The most changes one page of the history of a project has. */
export const MOST_PROJECT_CHANGES = 200;

/** Every version a task of a project has had, and whether it is gone since. */
export interface TaskVersions {
  versions: readonly Task[];
  gone: boolean;
}

/**
 * One page of the history of a project, the newest change first, as the
 * application crate tells it (`TaskService::project_history`): at most
 * `limit` changes (never more than 200) older than `before`, compared to
 * the millisecond. A page does not split the changes of one millisecond: if
 * the last ones do not all fit, they are left for the next page, unless the
 * page would then be empty.
 */
export function projectHistoryOf(
  tasks: readonly TaskVersions[],
  limit: number,
  before: string | undefined,
): ProjectChange[] {
  const most = Math.min(Math.max(Math.trunc(limit), 0), MOST_PROJECT_CHANGES);
  if (most === 0) {
    return [];
  }
  const beforeMillis = before === undefined ? undefined : Date.parse(before);
  const placed: { place: number; change: ProjectChange }[] = [];
  for (const { versions, gone } of tasks) {
    const latest = versions.at(-1);
    if (!latest) {
      continue;
    }
    const task = { id: latest.id, number: latest.number, title: latest.title, gone };
    historyOf(versions).forEach((change, place) => {
      if (beforeMillis === undefined || Date.parse(change.at) < beforeMillis) {
        placed.push({ place, change: { ...change, task } });
      }
    });
  }
  const byId = (one: string, other: string) => (one < other ? -1 : one > other ? 1 : 0);
  placed.sort(
    (one, other) =>
      Date.parse(other.change.at) - Date.parse(one.change.at) ||
      byId(one.change.task.id, other.change.task.id) ||
      other.place - one.place,
  );
  const changes = placed.map(({ change }) => change);
  const firstLeftOut = changes[most];
  if (firstLeftOut === undefined) {
    return changes;
  }
  const cut = Date.parse(firstLeftOut.at);
  const inCut = (change: ProjectChange) => Date.parse(change.at) === cut;
  let end = most;
  for (let previous = changes[end - 1]; previous && inCut(previous); previous = changes[end - 1]) {
    end -= 1;
  }
  if (end === 0) {
    end = changes.findIndex((change) => !inCut(change));
    if (end === -1) {
      end = changes.length;
    }
  }
  return changes.slice(0, end);
}

/**
 * A believable past for a task of the sample workspace, which has only the
 * version it has now: it was opened in the first status of its workflow and
 * given to nobody, then given to whoever has it now, then moved to the
 * status it is in now. The last version is the task as it is.
 */
export function pretendedPastOf(task: Task, firstStatusId: string, now: Date): Task[] {
  const created = Date.parse(task.createdAt);
  const latest = now.getTime();
  const at = (millis: number) => new Date(Math.min(millis, latest)).toISOString();
  const hour = 60 * 60 * 1000;
  const opened: Task = {
    ...structuredClone(task),
    statusId: firstStatusId,
    assignee: null,
    completedAt: null,
    updatedAt: task.createdAt,
  };
  const versions: Task[] = [opened];
  let last = opened;
  if (task.assignee !== null) {
    last = { ...last, assignee: structuredClone(task.assignee), updatedAt: at(created + 2 * hour) };
    versions.push(last);
  }
  if (task.statusId !== firstStatusId) {
    const moved = Math.max(
      Date.parse(task.completedAt ?? at(created + 26 * hour)),
      Date.parse(last.updatedAt) + 60 * 1000,
    );
    last = {
      ...structuredClone(task),
      updatedAt: at(moved),
    };
    versions.push(last);
  }
  if (JSON.stringify(last) !== JSON.stringify(task)) {
    versions.push(structuredClone(task));
  }
  return versions;
}
