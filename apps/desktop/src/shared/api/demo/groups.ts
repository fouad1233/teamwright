import type {
  Area,
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
} from '../types';

/**
 * The rule that puts the tasks of a view into groups, for the demo backend.
 * The real application has it in its domain crate (`grouping.rs`); the cases
 * of `demoGroups.test.ts` follow the tests there.
 */

/** What the groups are made from besides the tasks. */
export interface GroupingContext {
  statuses: readonly Status[];
  /** The members of the team, in the order of the team. */
  members: readonly Member[];
  /** The areas, in the order the manager gave them. */
  areas: readonly Area[];
  /** The sprints, in the order they are listed in: active, planned, completed. */
  sprints: readonly Sprint[];
  /** The tags, by name. */
  tags: readonly Tag[];
  tasks: readonly Task[];
}

/** The priorities from the highest. */
const PRIORITIES_FROM_HIGHEST: readonly Priority[] = ['urgent', 'high', 'normal', 'low'];

/**
 * Puts the tasks of a project into groups, in a fixed order.
 *
 * - Only tasks that are not subtasks are grouped.
 * - Within a group the tasks are in the order of their statuses, then by
 *   rank, then by number.
 * - Every group a card can be dropped on is returned, even empty. A
 *   completed sprint and the group of a removed area are returned only with
 *   tasks. A task with several tags is in the group of each of them.
 * - A task of a member, tag or sprint that is not known counts as given to
 *   nobody, as without a tag, or as in the backlog.
 */
export function groupTasks(grouping: Grouping, context: GroupingContext): TaskGroup[] {
  const statusPlace = (task: Task) => {
    const place = context.statuses.findIndex((status) => status.id === task.statusId);
    return place === -1 ? Number.MAX_SAFE_INTEGER : place;
  };
  const roots = context.tasks
    .filter((task) => task.parentId === null)
    .sort(
      (first, second) =>
        statusPlace(first) - statusPlace(second) ||
        first.rank - second.rank ||
        first.number - second.number ||
        first.id.localeCompare(second.id),
    );
  const group = (key: GroupKey, belongs: (task: Task) => boolean): TaskGroup => ({
    key,
    taskIds: roots.filter(belongs).map((task) => task.id),
  });

  switch (grouping) {
    case 'status':
      return context.statuses.map((status) =>
        group({ kind: 'status', id: status.id }, (task) => task.statusId === status.id),
      );
    case 'assignee': {
      const known = new Set(context.members.map((member) => member.id));
      return [
        group({ kind: 'manager' }, (task) => task.assignee?.kind === 'manager'),
        ...context.members.map((member) =>
          group(
            { kind: 'member', id: member.id },
            (task) => task.assignee?.kind === 'member' && task.assignee.memberId === member.id,
          ),
        ),
        group(
          { kind: 'nobody' },
          (task) =>
            task.assignee === null ||
            (task.assignee.kind === 'member' && !known.has(task.assignee.memberId)),
        ),
      ];
    }
    case 'priority':
      return PRIORITIES_FROM_HIGHEST.map((priority) =>
        group({ kind: 'priority', priority }, (task) => task.priority === priority),
      );
    case 'area': {
      const known = new Set<Id>(context.areas.map((area) => area.id));
      const groups = context.areas.map((area) =>
        group({ kind: 'area', id: area.id }, (task) => task.areaId === area.id),
      );
      const strays = group({ kind: 'noArea' }, (task) => !known.has(task.areaId));
      return strays.taskIds.length > 0 ? [...groups, strays] : groups;
    }
    case 'tag': {
      const known = new Set<Id>(context.tags.map((tag) => tag.id));
      return [
        ...context.tags.map((tag) =>
          group({ kind: 'tag', id: tag.id }, (task) => task.tagIds.includes(tag.id)),
        ),
        group({ kind: 'noTag' }, (task) => task.tagIds.every((id) => !known.has(id))),
      ];
    }
    case 'sprint': {
      const known = new Set<Id>(context.sprints.map((sprint) => sprint.id));
      return [
        ...context.sprints
          .map((sprint) => ({
            sprint,
            group: group({ kind: 'sprint', id: sprint.id }, (task) => task.sprintId === sprint.id),
          }))
          .filter(
            ({ sprint, group: found }) => sprint.state !== 'completed' || found.taskIds.length > 0,
          )
          .map(({ group: found }) => found),
        group({ kind: 'backlog' }, (task) => task.sprintId === null || !known.has(task.sprintId)),
      ];
    }
  }
}
