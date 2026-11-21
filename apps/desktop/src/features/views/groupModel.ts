import type {
  Area,
  Assignee,
  GroupKey,
  Grouping,
  Id,
  Member,
  Priority,
  Sprint,
  Status,
  Tag,
  Task,
  TaskGroup,
  ViewKind,
} from '@/shared/api/types';

/**
 * What a board and a list work out about groups without drawing anything:
 * the names of the groupings, what a group is called, which groups take a
 * card, and which existing command a card dropped on a group stands for.
 * The groups themselves, and their order, come from the backend; nothing
 * here decides which task belongs where.
 */

/** The groupings in the order of the menu, with their words. */
export const GROUPINGS: readonly { grouping: Grouping; label: string }[] = [
  { grouping: 'status', label: 'Status' },
  { grouping: 'assignee', label: 'Assignee' },
  { grouping: 'priority', label: 'Priority' },
  { grouping: 'area', label: 'Area' },
  { grouping: 'tag', label: 'Tag' },
  { grouping: 'sprint', label: 'Sprint' },
];

/** The kinds of view that can be grouped by something other than status. The backend refuses the rest. */
const GROUPED_KINDS: readonly ViewKind[] = ['board', 'list'];

/** Whether a view of a kind can be grouped. */
export function isGroupable(kind: ViewKind): boolean {
  return GROUPED_KINDS.includes(kind);
}

/** The name of a grouping for people. */
export function groupingLabel(grouping: Grouping): string {
  return GROUPINGS.find((choice) => choice.grouping === grouping)?.label ?? grouping;
}

/** What group names are made from. */
export interface GroupNames {
  statuses: readonly Status[];
  members: readonly Member[];
  areas: readonly Area[];
  sprints: readonly Sprint[];
  tags: readonly Tag[];
}

/** One text for a group, stable and unique among the groups of a view, for keys and drop targets. */
export function groupId(key: GroupKey): string {
  switch (key.kind) {
    case 'status':
    case 'member':
    case 'area':
    case 'tag':
    case 'sprint':
      return `${key.kind}:${key.id}`;
    case 'priority':
      return `priority:${key.priority}`;
    case 'manager':
    case 'nobody':
    case 'noArea':
    case 'noTag':
    case 'backlog':
      return key.kind;
  }
}

const PRIORITY_NAMES: Record<Priority, string> = {
  urgent: 'Urgent',
  high: 'High',
  normal: 'Normal',
  low: 'Low',
};

/** What a group is called, in words: for a label that has no picture, and for a reader of the screen. */
export function groupName(key: GroupKey, names: GroupNames): string {
  switch (key.kind) {
    case 'status':
      return names.statuses.find((status) => status.id === key.id)?.name ?? 'A status';
    case 'manager':
      return 'You';
    case 'member':
      return names.members.find((member) => member.id === key.id)?.name ?? 'Team member';
    case 'nobody':
      return 'Unassigned';
    case 'priority':
      return PRIORITY_NAMES[key.priority];
    case 'area':
      return names.areas.find((area) => area.id === key.id)?.name ?? 'An area';
    case 'noArea':
      return 'No area';
    case 'tag':
      return names.tags.find((tag) => tag.id === key.id)?.name ?? 'A tag';
    case 'noTag':
      return 'No tag';
    case 'sprint':
      return names.sprints.find((sprint) => sprint.id === key.id)?.name ?? 'A sprint';
    case 'backlog':
      return 'Backlog';
  }
}

/**
 * Whether a card can be dropped on a group. A group of tasks of an area
 * that is gone and a sprint that is over take no card: nothing can be moved
 * into either.
 */
export function canDropOn(key: GroupKey, sprints: readonly Sprint[]): boolean {
  switch (key.kind) {
    case 'noArea':
      return false;
    case 'sprint':
      return sprints.find((sprint) => sprint.id === key.id)?.state !== 'completed';
    default:
      return true;
  }
}

/** What dropping a card on another group changes, in the terms of the commands that already exist. */
export type DropEffect =
  | { kind: 'assign'; assignee: Assignee | null }
  | { kind: 'prioritize'; priority: Priority }
  | { kind: 'move-to-area'; areaId: Id }
  | { kind: 'plan'; sprintId: Id | null }
  /**
   * Puts one tag on the task and takes another off: dropped on the lane of a
   * tag, the task gets that tag and loses the tag of the lane it came from.
   */
  | { kind: 'tags'; put: Id | null; takeOff: Id | null };

/**
 * What a card dropped from one group onto another changes, or nothing when
 * the drop changes nothing or is not offered.
 *
 * For tags, a card is in a lane for each of its tags, so a drop swaps the
 * tag of the lane it came from for the tag of the lane it went to; from or
 * to "No tag" it only puts one on or takes one off.
 */
export function dropEffect(task: Task, from: GroupKey, to: GroupKey): DropEffect | undefined {
  if (groupId(from) === groupId(to)) {
    return undefined;
  }
  switch (to.kind) {
    case 'manager':
      return { kind: 'assign', assignee: { kind: 'manager' } };
    case 'member':
      return { kind: 'assign', assignee: { kind: 'member', memberId: to.id } };
    case 'nobody':
      return { kind: 'assign', assignee: null };
    case 'priority':
      return { kind: 'prioritize', priority: to.priority };
    case 'area':
      return { kind: 'move-to-area', areaId: to.id };
    case 'sprint':
      return { kind: 'plan', sprintId: to.id };
    case 'backlog':
      return { kind: 'plan', sprintId: null };
    case 'tag': {
      const put = task.tagIds.includes(to.id) ? null : to.id;
      return { kind: 'tags', put, takeOff: from.kind === 'tag' ? from.id : null };
    }
    case 'noTag':
      return from.kind === 'tag' ? { kind: 'tags', put: null, takeOff: from.id } : undefined;
    case 'status':
    case 'noArea':
      return undefined;
  }
}

/**
 * The groups as they are once a card has been dropped from one group on
 * another, so that the card does not jump back while the backend answers.
 * A card dropped from a tag lane leaves that lane; a card is never put in
 * a lane it is in already.
 */
export function withCardMoved(
  groups: readonly TaskGroup[],
  taskId: Id,
  from: GroupKey,
  to: GroupKey,
): TaskGroup[] {
  const fromId = groupId(from);
  const toId = groupId(to);
  return groups.map((group) => {
    const id = groupId(group.key);
    if (id === fromId) {
      return { ...group, taskIds: group.taskIds.filter((candidate) => candidate !== taskId) };
    }
    if (id === toId && !group.taskIds.includes(taskId)) {
      return { ...group, taskIds: [...group.taskIds, taskId] };
    }
    return group;
  });
}
