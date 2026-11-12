import { useSortable } from '@dnd-kit/sortable';
import { CSS } from '@dnd-kit/utilities';
import { Hourglass } from 'lucide-react';
import type { HTMLAttributes, KeyboardEvent, Ref } from 'react';

import type { Id, Project, Status, Task } from '@/shared/api/types';
import { classNames } from '@/shared/lib/classNames';
import { formatDate, formatEstimate, isOverdue, taskReference } from '@/shared/lib/format';

import { SubtaskFold } from '../tasks/SubtaskFold';
import { PriorityFlag, StatusMark } from '../tasks/TaskBadges';
import type { SubtaskFolds } from '../tasks/subtaskFolds';
import { statusOf, unfinishedBlockers } from '../tasks/taskModel';
import { AssigneeAvatar } from '../team/Assignee';
import { useIsBeingWorkedOn } from '../work/WorkContext';
import styles from './TaskCard.module.css';

/** What the cards of a board need to know about the project they belong to. */
export interface BoardContents {
  project: Project;
  /** Every task of the project, to look up the tasks a card waits for. */
  tasksById: ReadonlyMap<Id, Task>;
  /** The direct subtasks of every task that has some. */
  subtasksByParent: ReadonlyMap<Id, readonly Task[]>;
  /** Which tasks show their subtasks. */
  folds: SubtaskFolds;
  onOpenTask: (taskId: Id) => void;
}

/** The subtasks of a task that has none. */
const NO_SUBTASKS: readonly Task[] = [];

/**
 * What the fold of a card does when nothing was given for it: the copy of a
 * card that follows the pointer shows the fold, but cannot be folded.
 */
function leaveAsItIs(): void {
  // Nothing to do.
}

/** What makes the face of a card something that can be pressed or dragged. */
type FaceProps = HTMLAttributes<HTMLDivElement> & { ref?: Ref<HTMLDivElement> };

interface TaskCardProps {
  project: Project;
  task: Task;
  /** How many of the tasks this one waits for are not finished yet. */
  unfinishedBlockerCount: number;
  /**
   * The status of the task, for a card that does not stand in the column of
   * its status: a subtask stands under its parent, wherever that is.
   */
  status?: Status;
  /** The subtasks of the task. A task that has some gets a row that folds them. */
  subtasks?: readonly Task[];
  areSubtasksShown?: boolean;
  onToggleSubtasks?: () => void;
  /** Handlers and attributes for the face: the part that opens the task. */
  face?: FaceProps;
}

/**
 * How a task looks on the board: its title, a line of facts below it, and,
 * if it has subtasks, a row that shows or hides them.
 */
export function TaskCard({
  project,
  task,
  unfinishedBlockerCount,
  status,
  subtasks = NO_SUBTASKS,
  areSubtasksShown = false,
  onToggleSubtasks,
  face,
}: TaskCardProps) {
  const overdue = isOverdue(task);
  const hasRaisedPriority = task.priority === 'urgent' || task.priority === 'high';
  const isBeingWorkedOn = useIsBeingWorkedOn(task.id);

  return (
    <div className={classNames(styles.card, status && styles.part)}>
      <div className={styles.face} {...face}>
        <p className={styles.title}>
          {status && (
            <span className={styles.status} role="img" aria-label={status.name} title={status.name}>
              <StatusMark status={status} />
            </span>
          )}
          {task.title}
        </p>

        <div className={styles.facts}>
          <span className={styles.reference}>{taskReference(project, task)}</span>
          {hasRaisedPriority && <PriorityFlag priority={task.priority} />}
          {unfinishedBlockerCount > 0 && (
            <span className={styles.waiting} title="Waits for tasks that are not finished">
              <Hourglass />
              {unfinishedBlockerCount}
            </span>
          )}
          {task.dueOn !== null && (
            <span
              className={classNames(styles.fact, overdue && styles.overdue)}
              title={overdue ? 'Overdue' : 'Due date'}
            >
              {formatDate(task.dueOn)}
            </span>
          )}
          {task.estimateMinutes !== null && (
            <span className={styles.fact} title="Estimate">
              {formatEstimate(task.estimateMinutes)}
            </span>
          )}
          <span className={styles.assignee}>
            {isBeingWorkedOn && (
              <span
                className={styles.atWork}
                role="img"
                aria-label="A specialist is working on this task"
                title="A specialist is working on this task"
              />
            )}
            <AssigneeAvatar assignee={task.assignee} />
          </span>
        </div>
      </div>

      {subtasks.length > 0 && (
        <SubtaskFold
          className={styles.fold}
          project={project}
          subtasks={subtasks}
          isOpen={areSubtasksShown}
          onToggle={onToggleSubtasks ?? leaveAsItIs}
        />
      )}
    </div>
  );
}

/** Everything a card shows that follows from the board and the task. */
function cardOf(board: BoardContents, task: Task) {
  const subtasks = board.subtasksByParent.get(task.id) ?? NO_SUBTASKS;
  return {
    project: board.project,
    task,
    unfinishedBlockerCount: unfinishedBlockers(task, board.tasksById).length,
    subtasks,
    areSubtasksShown: subtasks.length > 0 && board.folds.isOpen(task.id),
    onToggleSubtasks: () => {
      board.folds.toggle(task.id);
    },
  };
}

interface BoardCardProps {
  board: BoardContents;
  task: Task;
}

/**
 * A card that can be dragged to another position or status. With the
 * keyboard, Space picks it up and drops it, the arrow keys move it, and
 * Enter opens the task. Its subtasks travel with it.
 */
export function SortableTaskCard({ board, task }: BoardCardProps) {
  const {
    attributes,
    listeners,
    setNodeRef,
    setActivatorNodeRef,
    transform,
    transition,
    isDragging,
  } = useSortable({ id: task.id });
  const card = cardOf(board, task);

  function handleKeyDown(event: KeyboardEvent<HTMLDivElement>) {
    if (event.key === 'Enter') {
      board.onOpenTask(task.id);
      return;
    }
    listeners?.onKeyDown?.(event);
  }

  return (
    <li
      ref={setNodeRef}
      className={classNames(styles.item, isDragging && styles.lifted)}
      style={{ transform: CSS.Translate.toString(transform), transition }}
    >
      <TaskCard
        {...card}
        face={{
          ref: setActivatorNodeRef,
          ...attributes,
          ...listeners,
          onKeyDown: handleKeyDown,
          onClick: () => {
            board.onOpenTask(task.id);
          },
        }}
      />
      {card.areSubtasksShown && <SubtaskCards board={board} subtasks={card.subtasks} />}
    </li>
  );
}

interface SubtaskCardsProps {
  board: BoardContents;
  subtasks: readonly Task[];
}

/**
 * The subtasks of a card, set in under it. Each one shows the mark of its
 * own status, because it stays with its parent whatever its status is.
 */
function SubtaskCards({ board, subtasks }: SubtaskCardsProps) {
  return (
    <ul className={styles.subtasks}>
      {subtasks.map((subtask) => (
        <SubtaskCard key={subtask.id} board={board} task={subtask} />
      ))}
    </ul>
  );
}

/** The card of a subtask. It opens the task, and it can have subtasks itself. */
function SubtaskCard({ board, task }: BoardCardProps) {
  const card = cardOf(board, task);

  function open() {
    board.onOpenTask(task.id);
  }

  return (
    <li>
      <TaskCard
        {...card}
        status={statusOf(board.project, task)}
        face={{
          role: 'button',
          tabIndex: 0,
          onClick: open,
          onKeyDown: (event) => {
            if (event.key === 'Enter' || event.key === ' ') {
              event.preventDefault();
              open();
            }
          },
        }}
      />
      {card.areSubtasksShown && <SubtaskCards board={board} subtasks={card.subtasks} />}
    </li>
  );
}
