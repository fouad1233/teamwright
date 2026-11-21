import { useDroppable } from '@dnd-kit/core';
import { SortableContext, verticalListSortingStrategy } from '@dnd-kit/sortable';

import type { Status, StatusLoad, Task } from '@/shared/api/types';
import { classNames } from '@/shared/lib/classNames';

import { StatusLabel } from '../tasks/TaskBadges';
import { TaskComposer } from '../tasks/TaskComposer';
import styles from './BoardLane.module.css';
import { laneLimit, type LaneLimit } from './boardModel';
import { SortableTaskCard, type BoardContents } from './TaskCard';

interface BoardLaneProps {
  board: BoardContents;
  status: Status;
  /**
   * The tasks of the status from top to bottom, without the ones that are
   * part of another task: those are shown under their parent.
   */
  tasks: Task[];
  /** How many tasks of the whole project stand in the status, against its limit. */
  load?: StatusLoad;
  /** Opens a task in this status. Resolves to whether it was opened. */
  onCreateTask: (title: string) => Promise<boolean>;
}

/** One status of the board: its name, its cards, and a line to add a task. */
export function BoardLane({ board, status, tasks, load, onCreateTask }: BoardLaneProps) {
  // The whole list accepts drops, so a card can be dropped into an empty
  // status or below the last card.
  const { setNodeRef, isOver } = useDroppable({ id: status.id });
  const limit = laneLimit(load);

  return (
    <section className={styles.lane} aria-label={status.name}>
      <header className={styles.header} data-fill={limit?.fill}>
        <span className={styles.name}>
          <StatusLabel status={status} />
        </span>
        {limit ? (
          <LimitFigure limit={limit} />
        ) : (
          <span className={styles.count}>{tasks.length}</span>
        )}
        {limit && <LimitGauge limit={limit} />}
      </header>

      <SortableContext items={tasks.map((task) => task.id)} strategy={verticalListSortingStrategy}>
        <ul ref={setNodeRef} className={classNames(styles.cards, isOver && styles.dropTarget)}>
          {tasks.map((task) => (
            <SortableTaskCard key={task.id} board={board} task={task} />
          ))}
        </ul>
      </SortableContext>

      <TaskComposer onCreate={onCreateTask} />
    </section>
  );
}

/**
 * The tasks of a status against its limit, as in `5/4`. A status over its
 * limit says so in a word as well as in its color, so that it still reads
 * without color. Nothing is lit: an exceeded limit is a signal about the
 * flow of the work, not something that waits for the manager (decision
 * 0033).
 */
function LimitFigure({ limit }: { limit: LaneLimit }) {
  return (
    <span className={styles.limit} title={limit.words}>
      <span className={styles.figure} aria-label={limit.words}>
        {limit.figure}
      </span>
      {limit.fill === 'over' && (
        <span className={styles.overWord} aria-hidden>
          over
        </span>
      )}
    </span>
  );
}

/**
 * A gauge under the head of a lane: one mark for each task the limit
 * allows, inked where a task stands, or one bar for a large limit.
 */
function LimitGauge({ limit }: { limit: LaneLimit }) {
  return (
    <div className={styles.gauge} data-fill={limit.fill} aria-hidden>
      {limit.marks ? (
        limit.marks.map((isTaken, place) => (
          <span key={place} className={styles.mark} data-taken={isTaken || undefined} />
        ))
      ) : (
        <span className={styles.bar}>
          <span
            className={styles.barFill}
            style={{ width: `${(limit.share * 100).toString()}%` }}
          />
        </span>
      )}
    </div>
  );
}
