import type { Assignee, Id, Member, Project, Task, TaskChange } from '@/shared/api/types';
import { formatDate, formatEstimate, taskReference } from '@/shared/lib/format';

import { priorityLabel } from './TaskBadges';

/** What is needed to put a change into words: the names behind the identifiers. */
export interface HistoryNames {
  project: Project;
  /** Every task of the project, to name the tasks a task waited for. */
  tasks: readonly Task[];
  /** The team of the project, to name who a task was given to. */
  members: readonly Member[];
}

/**
 * Says one change of a task in a few words, the way the history lists it:
 * "Moved from To do to In progress".
 *
 * A change can name a status, a member or a task that is gone by now. The
 * history then says so instead of showing an identifier.
 */
export function changeInWords(change: TaskChange, names: HistoryNames): string {
  switch (change.kind) {
    case 'created':
      return 'Opened';
    case 'renamed':
      return `Renamed from "${change.from}"`;
    case 'described':
      return 'Description changed';
    case 'moved':
      return `Moved from ${statusName(names, change.fromStatusId)} to ${statusName(names, change.toStatusId)}`;
    case 'assigned':
      return `Given to ${assigneeName(names, change.to)}`;
    case 'prioritized':
      return `Priority changed from ${priorityLabel(change.from)} to ${priorityLabel(change.to)}`;
    case 'estimated':
      if (change.to === null) {
        return 'Estimate cleared';
      }
      return change.from === null
        ? `Estimate set to ${formatEstimate(change.to)}`
        : `Estimate changed from ${formatEstimate(change.from)} to ${formatEstimate(change.to)}`;
    case 'rescheduled': {
      const { startOn, dueOn } = change.to;
      if (startOn !== null && dueOn !== null) {
        return `Planned from ${formatDate(startOn)} to ${formatDate(dueOn)}`;
      }
      if (dueOn !== null) {
        return `Due date set to ${formatDate(dueOn)}`;
      }
      return startOn === null ? 'Dates cleared' : `Start set to ${formatDate(startOn)}`;
    }
    case 'now_waits_for':
      return `Now waits for ${taskName(names, change.taskId)}`;
    case 'no_longer_waits_for':
      return `No longer waits for ${taskName(names, change.taskId)}`;
  }
}

function statusName({ project }: HistoryNames, statusId: Id): string {
  return (
    project.statuses.find((status) => status.id === statusId)?.name ?? 'a status that was removed'
  );
}

function assigneeName({ members }: HistoryNames, assignee: Assignee | null): string {
  if (assignee === null) {
    return 'nobody';
  }
  if (assignee.kind === 'manager') {
    return 'you';
  }
  return (
    members.find((member) => member.id === assignee.memberId)?.name ??
    'a member who has left the team'
  );
}

function taskName({ project, tasks }: HistoryNames, taskId: Id): string {
  const task = tasks.find((other) => other.id === taskId);
  return task ? taskReference(project, task) : 'a task that was removed';
}
