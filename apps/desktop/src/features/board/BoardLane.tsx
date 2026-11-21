import { useDroppable } from '@dnd-kit/core';
import { SortableContext, verticalListSortingStrategy } from '@dnd-kit/sortable';

import type { ReactNode } from 'react';

import type { StatusLoad, Task } from '@/shared/api/types';
import { classNames } from '@/shared/lib/classNames';

import { TaskComposer } from '../tasks/TaskComposer';
import styles from './BoardLane.module.css';
import { laneLimit, type LaneLimit } from './boardModel';
import { SortableTaskCard, type BoardContents } from './TaskCard';

interface BoardLaneProps {
  board: BoardContents;
  /** What a drop on the lane is told: the status of the lane, or the key of another group. */
  id: string;
  /** The name of the lane in words, for a reader of the screen. */
  name: string;
  /** What stands at the head of the lane. */
  label: ReactNode;
  /**
   * The tasks of the lane from top to bottom, without the ones that are
   * part of another task: those are shown under their parent.
   */
  tasks: Task[];
  /**
   * How many tasks of the whole project stand in the status of the lane,
   * against its limit. Only a lane of a status has one.
   */
  load?: StatusLoad;
  /**
   * Opens a task in this lane. Resolves to whether it was opened. A lane
   * that cannot open a task of its own has none, and no line to add one.
   */
  onCreateTask?: (title: string) => Promise<boolean>;
  /** Whether a card cannot be dropped on the lane. */
  isDropDisabled?: boolean;
  /**
   * What the card of a task has in the lane to drag it by. A card that is
   * in two lanes at once, for a task with two tags, needs one for each.
   */
  dragIdOf?: (task: Task) => string;
}

/** One lane of the board: its head, its cards, and a line to add a task. */
export function BoardLane({
  board,
  id,
  name,
  label,
  tasks,
  load,
  onCreateTask,
  isDropDisabled = false,
  dragIdOf,
}: BoardLaneProps) {
  // The whole list accepts drops, so a card can be dropped into an empty
  // lane or below the last card.
  const { setNodeRef, isOver } = useDroppable({ id, disabled: isDropDisabled });
  const limit = laneLimit(load);
  const dragIds = tasks.map((task) => dragIdOf?.(task) ?? task.id);

  return (
    <section className={styles.lane} aria-label={name}>
      <header className={styles.header} data-fill={limit?.fill}>
        <span className={styles.name}>{label}</span>
        {limit ? (
          <LimitFigure limit={limit} />
        ) : (
          <span className={styles.count}>{tasks.length}</span>
        )}
        {limit && <LimitGauge limit={limit} />}
      </header>

      <SortableContext items={dragIds} strategy={verticalListSortingStrategy}>
        <ul ref={setNodeRef} className={classNames(styles.cards, isOver && styles.dropTarget)}>
          {tasks.map((task, place) => (
            <SortableTaskCard
              key={dragIds[place]}
              board={board}
              task={task}
              dragId={dragIds[place]}
            />
          ))}
        </ul>
      </SortableContext>

      {onCreateTask && <TaskComposer onCreate={onCreateTask} />}
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
