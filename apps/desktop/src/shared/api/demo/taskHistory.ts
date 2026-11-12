import type { Task, TaskChange } from '../types';

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
