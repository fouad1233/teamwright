import type {
  Assignee,
  CustomField,
  Id,
  Member,
  Project,
  Sprint,
  Tag,
  Task,
  TaskChange,
} from '@/shared/api/types';
import { formatDate, formatEstimate, taskReference } from '@/shared/lib/format';

import { valueInWords } from '../fields/fieldModel';
import { priorityLabel } from './TaskBadges';

/** What is needed to put a change into words: the names behind the identifiers. */
export interface HistoryNames {
  project: Project;
  /** Every task of the project, to name the tasks a task waited for. */
  tasks: readonly Task[];
  /** The team of the project, to name who a task was given to. */
  members: readonly Member[];
  /** The tags of the project, to name a tag put on or taken off. */
  tags?: readonly Tag[];
  /** The sprints of the project, to name the sprints a task went into. Left out while they load. */
  sprints?: readonly Sprint[];
  /** The custom fields of the project, to name a field and its value. */
  fields?: readonly CustomField[];
}

/**
 * Says one change of a task in a few words, the way the history of the task
 * lists it: "Moved from To do to In progress".
 *
 * A change can name a status, a member or a task that is gone by now. The
 * history then says so instead of showing an identifier.
 */
export function changeInWords(change: TaskChange, names: HistoryNames): string {
  const { words } = wordingOf(change, names);
  return words.charAt(0).toUpperCase() + words.slice(1);
}

/**
 * Says one change as the rest of a sentence that begins with the name of
 * the task, the way the history of a project tells it: "was given to you",
 * to follow "SSN-4 Draw the schematic". The words are those of
 * `changeInWords`.
 */
export function changeAfterTaskName(change: TaskChange, names: HistoryNames): string {
  const { joinedBy, words } = wordingOf(change, names);
  return joinedBy === null ? words : `${joinedBy} ${words}`;
}

/** One change in words, written once for both ways of saying it. */
interface Wording {
  /**
   * What joins the words to the name of the task in a sentence: "was" for
   * something done to the task ("was given to you"), "had its" for
   * something done to a part of it ("had its estimate cleared"), and
   * nothing where the task is what acts ("now waits for SSN-3").
   */
  joinedBy: 'was' | 'had its' | null;
  /** What happened, without the task and without a capital: "given to you". */
  words: string;
}

function wordingOf(change: TaskChange, names: HistoryNames): Wording {
  switch (change.kind) {
    case 'created':
      return { joinedBy: 'was', words: 'opened' };
    case 'renamed':
      return { joinedBy: 'was', words: `renamed from "${change.from}"` };
    case 'described':
      return { joinedBy: 'had its', words: 'description changed' };
    case 'moved':
      return {
        joinedBy: null,
        words: `moved from ${statusName(names, change.fromStatusId)} to ${statusName(names, change.toStatusId)}`,
      };
    case 'made_part_of':
      if (change.toTaskId === null) {
        return {
          joinedBy: 'was',
          words:
            change.fromTaskId === null
              ? 'made a task of its own'
              : `made a task of its own, out of ${taskName(names, change.fromTaskId)}`,
        };
      }
      return {
        joinedBy: 'was',
        words:
          change.fromTaskId === null
            ? `made a part of ${taskName(names, change.toTaskId)}`
            : `moved to ${taskName(names, change.toTaskId)}, out of ${taskName(names, change.fromTaskId)}`,
      };
    case 'assigned':
      return { joinedBy: 'was', words: `given to ${assigneeName(names, change.to)}` };
    case 'prioritized':
      return {
        joinedBy: 'had its',
        words: `priority changed from ${priorityLabel(change.from)} to ${priorityLabel(change.to)}`,
      };
    case 'estimated':
      if (change.to === null) {
        return { joinedBy: 'had its', words: 'estimate cleared' };
      }
      return {
        joinedBy: 'had its',
        words:
          change.from === null
            ? `estimate set to ${formatEstimate(change.to)}`
            : `estimate changed from ${formatEstimate(change.from)} to ${formatEstimate(change.to)}`,
      };
    case 'planned': {
      if (change.toSprintId === null) {
        return {
          joinedBy: 'was',
          words: `taken back to the backlog from ${sprintName(names, change.fromSprintId)}`,
        };
      }
      const into = `planned into ${sprintName(names, change.toSprintId)}`;
      return {
        joinedBy: 'was',
        words:
          change.fromSprintId === null
            ? into
            : `${into}, from ${sprintName(names, change.fromSprintId)}`,
      };
    }
    case 'pointed':
      if (change.to === null) {
        return { joinedBy: 'had its', words: 'points taken away' };
      }
      return change.from === null
        ? { joinedBy: 'was', words: `given ${pointsInWords(change.to)}` }
        : {
            joinedBy: 'had its',
            words: `points changed from ${change.from.toString()} to ${change.to.toString()}`,
          };
    case 'rescheduled': {
      const { startOn, dueOn } = change.to;
      if (startOn !== null && dueOn !== null) {
        return {
          joinedBy: 'was',
          words: `planned from ${formatDate(startOn)} to ${formatDate(dueOn)}`,
        };
      }
      if (dueOn !== null) {
        return { joinedBy: 'had its', words: `due date set to ${formatDate(dueOn)}` };
      }
      return {
        joinedBy: 'had its',
        words: startOn === null ? 'dates cleared' : `start set to ${formatDate(startOn)}`,
      };
    }
    case 'now_waits_for':
      return { joinedBy: null, words: `now waits for ${taskName(names, change.taskId)}` };
    case 'no_longer_waits_for':
      return { joinedBy: null, words: `no longer waits for ${taskName(names, change.taskId)}` };
    case 'tagged':
      return { joinedBy: 'was', words: `tagged ${tagName(names, change.tagId)}` };
    case 'untagged':
      return { joinedBy: 'had its', words: `tag ${tagName(names, change.tagId)} taken off` };
    case 'ticked':
      return { joinedBy: 'had its', words: `checklist item "${change.item}" ticked` };
    case 'unticked':
      return { joinedBy: 'had its', words: `checklist item "${change.item}" unticked` };
    case 'field_value_changed': {
      const field = names.fields?.find((other) => other.id === change.fieldId);
      const name = field ? `"${field.name}"` : 'a field that was removed';
      const value = (shown: NonNullable<typeof change.to>) => `"${valueInWords(field, shown)}"`;
      if (change.to === null) {
        return { joinedBy: 'had its', words: `field ${name} cleared` };
      }
      return {
        joinedBy: 'had its',
        words:
          change.from === null
            ? `field ${name} set to ${value(change.to)}`
            : `field ${name} changed from ${value(change.from)} to ${value(change.to)}`,
      };
    }
  }
}

function tagName({ tags }: HistoryNames, tagId: Id): string {
  const tag = tags?.find((other) => other.id === tagId);
  return tag ? `"${tag.name}"` : 'a tag that was removed';
}

function statusName({ project }: HistoryNames, statusId: Id): string {
  return (
    project.statuses.find((status) => status.id === statusId)?.name ?? 'a status that was removed'
  );
}

function sprintName({ sprints }: HistoryNames, sprintId: Id | null): string {
  if (sprintId === null) {
    return 'the backlog';
  }
  if (sprints === undefined) {
    return 'a sprint';
  }
  return sprints.find((sprint) => sprint.id === sprintId)?.name ?? 'a sprint that was deleted';
}

function pointsInWords(points: number): string {
  return `${points.toString()} ${points === 1 ? 'point' : 'points'}`;
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
