import {
  DndContext,
  DragOverlay,
  KeyboardSensor,
  PointerSensor,
  closestCorners,
  defaultDropAnimationSideEffects,
  useSensor,
  useSensors,
  type DragEndEvent,
  type DragOverEvent,
  type DragStartEvent,
  type DropAnimation,
} from '@dnd-kit/core';
import { sortableKeyboardCoordinates } from '@dnd-kit/sortable';
import { CSS } from '@dnd-kit/utilities';
import { useMemo, useState } from 'react';

import { useNavigation } from '@/app/navigation';
import type { Id, Project, Task } from '@/shared/api/types';
import { durationToken, easingToken, usePrefersReducedMotion } from '@/shared/ui/motion';

import { useSubtaskFolds } from '../tasks/subtaskFolds';
import { indexById, indexSubtasks, unfinishedBlockers } from '../tasks/taskModel';
import { useTaskCommands } from '../tasks/taskQueries';
import { BoardLane } from './BoardLane';
import {
  arrangementOf,
  locate,
  moveCard,
  newArrivals,
  statusUnder,
  type Arrangement,
} from './boardModel';
import styles from './BoardView.module.css';
import { TaskCard, type BoardContents } from './TaskCard';

/** How far the pointer must move before a press becomes a drag, in pixels. */
const DRAG_START_DISTANCE = 6;

/** No task has just arrived. */
const NO_ARRIVALS: ReadonlySet<Id> = new Set();

/**
 * How a card that is let go is put down in its place (decision 0015). It
 * travels there and settles, while the copy in the hand loses its lift.
 * With motion turned down nothing travels: the copy fades where it was let
 * go, and the card is in its place.
 */
function putDown(reducesMotion: boolean): DropAnimation {
  const sideEffects = defaultDropAnimationSideEffects({
    // The card in the lane stays hidden until the copy has landed on it.
    styles: { active: { opacity: '0' } },
    className: { dragOverlay: styles.puttingDown },
  });
  if (reducesMotion) {
    return {
      duration: durationToken('--duration-quick'),
      easing: 'linear',
      sideEffects,
      keyframes: ({ transform }) => [
        { opacity: 1, transform: CSS.Transform.toString(transform.initial) },
        { opacity: 0, transform: CSS.Transform.toString(transform.initial) },
      ],
    };
  }
  return {
    duration: durationToken('--duration-settle'),
    easing: easingToken('--ease-arrive'),
    sideEffects,
  };
}

/** What the board knows about the tasks it has shown, to tell which are new. */
interface Seen {
  projectId: Id;
  tasks: readonly Task[];
  arrivals: ReadonlySet<Id>;
}

/**
 * The tasks that have just come onto the board, so that their cards can
 * arrive in their lane. The tasks the board opens with are not new. The
 * newest arrivals are kept until others come or a card is picked up, so a
 * second answer from the backend does not cut their arrival short.
 */
function useArrivals(projectId: Id, tasks: readonly Task[]) {
  const [seen, setSeen] = useState<Seen>({ projectId, tasks, arrivals: NO_ARRIVALS });
  if (seen.projectId !== projectId) {
    setSeen({ projectId, tasks, arrivals: NO_ARRIVALS });
  } else if (seen.tasks !== tasks) {
    const fresh = newArrivals(new Set(seen.tasks.map((task) => task.id)), tasks);
    setSeen({ projectId, tasks, arrivals: fresh.size > 0 ? fresh : seen.arrivals });
  }
  return {
    arrivals: seen.arrivals,
    forget: () => {
      setSeen((current) => ({ ...current, arrivals: NO_ARRIVALS }));
    },
  };
}

/** A card that is being dragged, and how the board looks meanwhile. */
interface Drag {
  taskId: Id;
  arrangement: Arrangement;
}

/**
 * A card that was just dropped. The board keeps showing the arrangement of
 * the drop until the tasks it was made from have been replaced by newer
 * ones, so the card never jumps back for a moment.
 */
interface Drop {
  arrangement: Arrangement;
  tasksAtDrop: Task[];
}

interface BoardViewProps {
  project: Project;
  tasks: Task[];
}

/**
 * The tasks of a project as cards in one column per status. A subtask has no
 * card of its own in a column: it is shown under its parent, which has a row
 * that folds its subtasks away.
 */
export function BoardView({ project, tasks }: BoardViewProps) {
  const commands = useTaskCommands(project.id);
  const navigation = useNavigation();
  const folds = useSubtaskFolds(project.id);
  const [drag, setDrag] = useState<Drag | null>(null);
  const [drop, setDrop] = useState<Drop | null>(null);
  const reducesMotion = usePrefersReducedMotion();
  const dropAnimation = useMemo(() => putDown(reducesMotion), [reducesMotion]);
  const { arrivals, forget: forgetArrivals } = useArrivals(project.id, tasks);

  const tasksById = useMemo(() => indexById(tasks), [tasks]);
  const subtasksByParent = useMemo(() => indexSubtasks(tasks), [tasks]);
  const board: BoardContents = {
    project,
    tasksById,
    subtasksByParent,
    folds,
    arrivals,
    onOpenTask: navigation.openTask,
  };
  const stored = useMemo(() => arrangementOf(project.statuses, tasks), [project.statuses, tasks]);
  const settling = drop?.tasksAtDrop === tasks ? drop.arrangement : null;
  const arrangement = drag?.arrangement ?? settling ?? stored;

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
    // A card that moves to another lane is put there anew; it must not
    // arrive there as if it were new.
    forgetArrivals();
    setDrag({ taskId: String(event.active.id), arrangement: stored });
  }

  function handleDragOver(event: DragOverEvent) {
    if (!drag || !event.over) {
      return;
    }
    const targetId = String(event.over.id);
    const from = locate(drag.arrangement, drag.taskId);
    const toStatusId = statusUnder(drag.arrangement, targetId);
    // Within one status the list itself shows where the card will land.
    // Only a change of status has to be applied while the card is dragged.
    if (!from || toStatusId === undefined || from.statusId === toStatusId) {
      return;
    }
    setDrag({
      taskId: drag.taskId,
      arrangement: moveCard(drag.arrangement, drag.taskId, targetId),
    });
  }

  function handleDragEnd(event: DragEndEvent) {
    const finished = drag;
    setDrag(null);
    if (!finished || !event.over) {
      return;
    }

    const dropped = moveCard(finished.arrangement, finished.taskId, String(event.over.id));
    const destination = locate(dropped, finished.taskId);
    const origin = locate(stored, finished.taskId);
    const hasMoved =
      origin?.statusId !== destination?.statusId || origin?.index !== destination?.index;
    if (!destination || !hasMoved) {
      return;
    }

    setDrop({ arrangement: dropped, tasksAtDrop: tasks });
    void commands.move(finished.taskId, destination.statusId, destination.index);
  }

  const draggedTask = drag ? tasksById.get(drag.taskId) : undefined;

  return (
    <DndContext
      sensors={sensors}
      collisionDetection={closestCorners}
      onDragStart={handleDragStart}
      onDragOver={handleDragOver}
      onDragEnd={handleDragEnd}
      onDragCancel={() => {
        setDrag(null);
      }}
    >
      <div className={styles.board}>
        {project.statuses.map((status) => (
          <BoardLane
            key={status.id}
            board={board}
            status={status}
            tasks={(arrangement.get(status.id) ?? []).flatMap(
              (taskId) => tasksById.get(taskId) ?? [],
            )}
            onCreateTask={async (title) => {
              const created = await commands.create({ title, statusId: status.id });
              return created !== undefined;
            }}
          />
        ))}
      </div>

      <DragOverlay dropAnimation={dropAnimation}>
        {draggedTask && (
          <div className={styles.dragged}>
            <TaskCard
              project={project}
              task={draggedTask}
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
