import { useDroppable } from '@dnd-kit/core';
import { SortableContext, verticalListSortingStrategy } from '@dnd-kit/sortable';

import type { Id, Project, Status, Task } from '@/shared/api/types';
import { classNames } from '@/shared/lib/classNames';

import { StatusPill } from '../tasks/TaskBadges';
import { unfinishedBlockers } from '../tasks/taskModel';
import styles from './BoardColumn.module.css';
import { SortableTaskCard } from './TaskCard';
import { TaskComposer } from '../tasks/TaskComposer';

interface BoardColumnProps {
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
export function BoardColumn({
  project,
  status,
  tasks,
  tasksById,
  onOpenTask,
  onCreateTask,
}: BoardColumnProps) {
  // The whole list accepts drops, so a card can be dropped into an empty
  // status or below the last card.
  const { setNodeRef, isOver } = useDroppable({ id: status.id });

  return (
    <section className={styles.column} aria-label={status.name}>
      <header className={styles.header}>
        <StatusPill status={status} />
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
