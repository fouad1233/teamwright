import { useDroppable } from '@dnd-kit/core';
import { SortableContext, verticalListSortingStrategy } from '@dnd-kit/sortable';

import type { Id, Project, Status, Task } from '@/shared/api/types';
import { classNames } from '@/shared/lib/classNames';

import { StatusLabel } from '../tasks/TaskBadges';
import { TaskComposer } from '../tasks/TaskComposer';
import { unfinishedBlockers } from '../tasks/taskModel';
import styles from './BoardLane.module.css';
import { SortableTaskCard } from './TaskCard';

interface BoardLaneProps {
  project: Project;
  status: Status;
  /** The tasks of the status, from top to bottom. */
  tasks: Task[];
  /** Every task of the project, to look up the tasks a card waits for. */
  tasksById: ReadonlyMap<Id, Task>;
  onOpenTask: (taskId: Id) => void;
  /** Opens a task in this status. Resolves to whether it was opened. */
  onCreateTask: (title: string) => Promise<boolean>;
}

/** One status of the board: its name, its cards, and a line to add a task. */
export function BoardLane({
  project,
  status,
  tasks,
  tasksById,
  onOpenTask,
  onCreateTask,
}: BoardLaneProps) {
  // The whole list accepts drops, so a card can be dropped into an empty
  // status or below the last card.
  const { setNodeRef, isOver } = useDroppable({ id: status.id });

  return (
    <section className={styles.lane} aria-label={status.name}>
      <header className={styles.header}>
        <span className={styles.name}>
          <StatusLabel status={status} />
        </span>
        <span className={styles.count}>{tasks.length}</span>
      </header>

      <SortableContext items={tasks.map((task) => task.id)} strategy={verticalListSortingStrategy}>
        <ul ref={setNodeRef} className={classNames(styles.cards, isOver && styles.dropTarget)}>
          {tasks.map((task) => (
            <SortableTaskCard
              key={task.id}
              project={project}
              task={task}
              unfinishedBlockerCount={unfinishedBlockers(task, tasksById).length}
              onOpen={() => {
                onOpenTask(task.id);
              }}
            />
          ))}
        </ul>
      </SortableContext>

      <TaskComposer onCreate={onCreateTask} />
    </section>
  );
}
