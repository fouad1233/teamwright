import {
  DndContext,
  DragOverlay,
  KeyboardSensor,
  PointerSensor,
  closestCorners,
  useSensor,
  useSensors,
  type DragEndEvent,
  type DragStartEvent,
} from '@dnd-kit/core';
import { sortableKeyboardCoordinates } from '@dnd-kit/sortable';
import { useMemo, useState } from 'react';

import { useNavigation } from '@/app/navigation';
import type { Assignee, GroupKey, Id, Project, Task, TaskGroup, View } from '@/shared/api/types';
import { usePrefersReducedMotion } from '@/shared/ui/motion';

import { useAreaCommands } from '../areas/areaQueries';
import { useSprintCommands } from '../sprints/sprintQueries';
import { useTagCommands } from '../tags/tagQueries';
import { useSubtaskFolds } from '../tasks/subtaskFolds';
import { indexById, indexSubtasks, statusOf, unfinishedBlockers } from '../tasks/taskModel';
import { useTaskCommands } from '../tasks/taskQueries';
import { GroupLabel } from '../views/GroupLabel';
import {
  canDropOn,
  dropEffect,
  groupId,
  groupName,
  withCardMoved,
  type DropEffect,
} from '../views/groupModel';
import { useGroupNames, useTaskGroups } from '../views/groupQueries';
import { BoardLane } from './BoardLane';
import { putDown } from './BoardView';
import styles from './BoardView.module.css';
import { TaskCard, type BoardContents } from './TaskCard';

/** How far the pointer must move before a press becomes a drag, in pixels. */
const DRAG_START_DISTANCE = 6;

/** No task has just arrived: lanes that are not statuses do not animate arrivals. */
const NO_ARRIVALS: ReadonlySet<Id> = new Set();

/** The text a card is dragged by in a lane. A task in two lanes at once has a card in each. */
function dragIdOf(laneId: string, task: Task): string {
  return `${laneId}|${task.id}`;
}

/** A card that was just dropped: the groups as the drop made them, until newer ones arrive. */
interface Drop {
  groups: TaskGroup[];
  groupsAtDrop: TaskGroup[];
}

interface GroupedBoardProps {
  project: Project;
  view: View;
  /** Every task of the project: the board knows them all, and `shows` says which get a card. */
  tasks: Task[];
  shows?: (task: Task) => boolean;
  /** Who a task opened from a lane is given to, for a board that shows one person's tasks. */
  assigneeOfNewTasks?: Assignee;
}

/**
 * The tasks of a project as cards in one lane for each group of the view:
 * its assignees, priorities, areas, tags or sprints (decision 0021). The
 * backend says which groups there are and which tasks stand in each. A
 * card dropped on another lane changes what the lane stands for, through
 * the commands that already change it; so every rule and the log apply.
 *
 * The lanes do not show the work-in-progress limits, which belong to
 * statuses (decision 0033); a card says its status itself instead. A card
 * cannot be put in place within a lane: the order is the order of the
 * statuses, and the ranks within them.
 */
export function GroupedBoard({
  project,
  view,
  tasks,
  shows,
  assigneeOfNewTasks,
}: GroupedBoardProps) {
  const navigation = useNavigation();
  const folds = useSubtaskFolds(project.id);
  const names = useGroupNames(project);
  const found = useTaskGroups(view).data;
  const taskCommands = useTaskCommands(project.id);
  const areaCommands = useAreaCommands(project.id);
  const sprintCommands = useSprintCommands(project.id);
  const tagCommands = useTagCommands(project.id);
  const [dragging, setDragging] = useState<string | null>(null);
  const [drop, setDrop] = useState<Drop | null>(null);
  const reducesMotion = usePrefersReducedMotion();
  const dropAnimation = useMemo(() => putDown(reducesMotion), [reducesMotion]);

  const tasksById = useMemo(() => indexById(tasks), [tasks]);
  const subtasksByParent = useMemo(() => indexSubtasks(tasks), [tasks]);
  const board: BoardContents = {
    project,
    tasksById,
    subtasksByParent,
    folds,
    arrivals: NO_ARRIVALS,
    showsStatus: true,
    onOpenTask: navigation.openTask,
  };

  const groups = drop && drop.groupsAtDrop === found ? drop.groups : found;
  const lanes = useMemo(
    () =>
      (groups ?? []).map((group) => ({
        group,
        id: groupId(group.key),
        tasks: group.taskIds
          .flatMap((taskId) => tasksById.get(taskId) ?? [])
          .filter((task) => shows?.(task) ?? true),
      })),
    [groups, tasksById, shows],
  );
  const sprints = names.sprints;

  /** The lane and the task a card or a lane being dragged over stands for. */
  function laneOf(dragOrLaneId: string): { key: GroupKey; taskId?: Id } | undefined {
    const direct = lanes.find((lane) => lane.id === dragOrLaneId);
    if (direct) {
      return { key: direct.group.key };
    }
    for (const lane of lanes) {
      const card = lane.tasks.find((task) => dragIdOf(lane.id, task) === dragOrLaneId);
      if (card) {
        return { key: lane.group.key, taskId: card.id };
      }
    }
    return undefined;
  }

  const sensors = useSensors(
    // A press only becomes a drag after some movement, so a click opens the task.
    useSensor(PointerSensor, { activationConstraint: { distance: DRAG_START_DISTANCE } }),
    // Space picks a card up and drops it. Enter is left free to open the task.
    useSensor(KeyboardSensor, {
      coordinateGetter: sortableKeyboardCoordinates,
      keyboardCodes: { start: ['Space'], cancel: ['Escape'], end: ['Space'] },
    }),
  );

  function handleDragStart(event: DragStartEvent) {
    setDragging(String(event.active.id));
  }

  function handleDragEnd(event: DragEndEvent) {
    setDragging(null);
    if (!event.over || !groups) {
      return;
    }
    const from = laneOf(String(event.active.id));
    const to = laneOf(String(event.over.id));
    const task = from?.taskId === undefined ? undefined : tasksById.get(from.taskId);
    if (!from || !to || !task || !canDropOn(to.key, sprints)) {
      return;
    }
    const effect = dropEffect(task, from.key, to.key);
    if (!effect) {
      return;
    }
    setDrop({
      groups: withCardMoved(groups, task.id, from.key, to.key),
      groupsAtDrop: found ?? [],
    });
    void applyDrop(task.id, effect);
  }

  /** Changes the task through the command that already changes what the lane stands for. */
  async function applyDrop(taskId: Id, effect: DropEffect): Promise<void> {
    switch (effect.kind) {
      case 'assign':
        return taskCommands.assign(taskId, effect.assignee);
      case 'prioritize':
        return taskCommands.prioritize(taskId, effect.priority);
      case 'move-to-area':
        return areaCommands.moveTask(taskId, effect.areaId);
      case 'plan':
        return sprintCommands.plan(taskId, effect.sprintId);
      case 'tags':
        if (effect.put !== null) {
          await tagCommands.setOnTask(taskId, effect.put, true);
        }
        if (effect.takeOff !== null) {
          await tagCommands.setOnTask(taskId, effect.takeOff, false);
        }
        return;
    }
  }

  /** A line to add a task to a lane, for the lanes where the new task would stay in it. */
  function creatorFor(key: GroupKey): ((title: string) => Promise<boolean>) | undefined {
    const create = async (more: Parameters<typeof taskCommands.create>[0]) => {
      const created = await taskCommands.create(more);
      return created !== undefined;
    };
    switch (key.kind) {
      case 'manager':
        return (title) => create({ title, assignee: { kind: 'manager' } });
      case 'member':
        return (title) => create({ title, assignee: { kind: 'member', memberId: key.id } });
      case 'nobody':
        return (title) => create({ title });
      case 'priority':
        return (title) => create({ title, priority: key.priority, assignee: assigneeOfNewTasks });
      case 'area':
        return (title) => create({ title, areaId: key.id, assignee: assigneeOfNewTasks });
      default:
        return undefined;
    }
  }

  const dragged = dragging === null ? undefined : laneOf(dragging);
  const draggedTask = dragged?.taskId === undefined ? undefined : tasksById.get(dragged.taskId);

  return (
    <DndContext
      sensors={sensors}
      collisionDetection={closestCorners}
      onDragStart={handleDragStart}
      onDragEnd={handleDragEnd}
      onDragCancel={() => {
        setDragging(null);
      }}
    >
      <div className={styles.board}>
        {lanes.map((lane) => (
          <BoardLane
            key={lane.id}
            board={board}
            id={lane.id}
            name={groupName(lane.group.key, names)}
            label={<GroupLabel groupKey={lane.group.key} names={names} />}
            tasks={lane.tasks}
            isDropDisabled={!canDropOn(lane.group.key, sprints)}
            dragIdOf={(task) => dragIdOf(lane.id, task)}
            onCreateTask={creatorFor(lane.group.key)}
          />
        ))}
      </div>

      <DragOverlay dropAnimation={dropAnimation}>
        {draggedTask && (
          <div className={styles.dragged}>
            <TaskCard
              project={project}
              task={draggedTask}
              statusShown={statusOf(project, draggedTask)}
              unfinishedBlockerCount={unfinishedBlockers(draggedTask, tasksById).length}
              subtasks={subtasksByParent.get(draggedTask.id)}
              areSubtasksShown={folds.isOpen(draggedTask.id)}
            />
          </div>
        )}
      </DragOverlay>
    </DndContext>
  );
}
