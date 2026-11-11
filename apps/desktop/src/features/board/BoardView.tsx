import {
  DndContext,
  DragOverlay,
  KeyboardSensor,
  PointerSensor,
  closestCorners,
  useSensor,
  useSensors,
  type DragEndEvent,
  type DragOverEvent,
  type DragStartEvent,
} from '@dnd-kit/core';
import { sortableKeyboardCoordinates } from '@dnd-kit/sortable';
import { useMemo, useState } from 'react';

import { useNavigation } from '@/app/navigation';
import type { Id, Project, Task } from '@/shared/api/types';

import { indexById, unfinishedBlockers } from '../tasks/taskModel';
import { useTaskCommands } from '../tasks/taskQueries';
import { BoardColumn } from './BoardColumn';
import { arrangementOf, locate, moveCard, statusUnder, type Arrangement } from './boardModel';
import styles from './BoardView.module.css';
import { TaskCard } from './TaskCard';

/** How far the pointer must move before a press becomes a drag, in pixels. */
const DRAG_START_DISTANCE = 6;

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

/** The tasks of a project as cards in one column per status. */
export function BoardView({ project, tasks }: BoardViewProps) {
  const commands = useTaskCommands(project.id);
  const navigation = useNavigation();
  const [drag, setDrag] = useState<Drag | null>(null);
  const [drop, setDrop] = useState<Drop | null>(null);

  const tasksById = useMemo(() => indexById(tasks), [tasks]);
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
          <BoardColumn
            key={status.id}
            project={project}
            status={status}
            tasks={(arrangement.get(status.id) ?? []).flatMap(
              (taskId) => tasksById.get(taskId) ?? [],
            )}
            tasksById={tasksById}
            onOpenTask={navigation.openTask}
            onCreateTask={async (title) => {
              const created = await commands.create({ title, statusId: status.id });
              return created !== undefined;
            }}
          />
        ))}
      </div>

      <DragOverlay>
        {draggedTask && (
          <div className={styles.dragged}>
            <TaskCard
              project={project}
              task={draggedTask}
              unfinishedBlockerCount={unfinishedBlockers(draggedTask, tasksById).length}
            />
          </div>
        )}
      </DragOverlay>
    </DndContext>
  );
}
