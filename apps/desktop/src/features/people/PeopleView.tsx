import {
  DndContext,
  DragOverlay,
  KeyboardSensor,
  PointerSensor,
  pointerWithin,
  rectIntersection,
  useDraggable,
  useDroppable,
  useSensor,
  useSensors,
  type CollisionDetection,
  type DragEndEvent,
  type DragStartEvent,
  type KeyboardCoordinateGetter,
} from '@dnd-kit/core';
import { useMemo, useState, type KeyboardEvent, type ReactNode } from 'react';

import { useNavigation } from '@/app/navigation';
import type { Assignee, CalendarDate, Conversation, Id, Project, Task } from '@/shared/api/types';
import { classNames } from '@/shared/lib/classNames';
import {
  calendarDateOf,
  formatDate,
  isOverdue,
  taskReference,
  today as todayDate,
} from '@/shared/lib/format';

import { Portrait } from '../office/characters/Portrait';
import { placeOfManager, placeOfMember, type PlaceState } from '../office/officeModel';
import { StatusMark } from '../tasks/TaskBadges';
import { indexById, statusOf } from '../tasks/taskModel';
import { useTaskCommands } from '../tasks/taskQueries';
import { MANAGER_NAME, NOBODY_NAME } from '../team/Assignee';
import { useTeamMembers } from '../team/TeamContext';
import { kindLabel, profileInWords } from '../team/teamModel';
import type { KindProps } from '../views/viewKinds';
import {
  useConversationsOfProject,
  useIsBeingWorkedOn,
  useWaitsForManager,
} from '../work/WorkContext';
import styles from './PeopleView.module.css';
import {
  NEXT_SHOWN,
  dropOutcome,
  figuresInWords,
  neighbourColumn,
  peopleColumns,
  withAssignee,
  type ColumnSubject,
  type PersonColumn,
} from './peopleModel';

/** How far the pointer must move before a press on a task becomes a drag, in pixels. */
const DRAG_START_DISTANCE = 6;

/**
 * A drop is found under the pointer. With the keyboard there is no pointer,
 * and the column the lifted task overlaps most is the one.
 */
const columnUnder: CollisionDetection = (args) => {
  const underPointer = pointerWithin(args);
  return underPointer.length > 0 ? underPointer : rectIntersection(args);
};

/**
 * With the keyboard, the left and right arrows carry a lifted task to the
 * middle of the next column on that side. Other keys do nothing.
 */
const toNeighbourColumn: KeyboardCoordinateGetter = (event, { context, currentCoordinates }) => {
  const side =
    event.code === 'ArrowLeft' ? 'left' : event.code === 'ArrowRight' ? 'right' : undefined;
  if (side === undefined) {
    return undefined;
  }
  event.preventDefault();
  const { collisionRect, droppableRects, droppableContainers } = context;
  if (!collisionRect) {
    return undefined;
  }
  const columns = droppableContainers
    .getEnabled()
    .flatMap((container) => droppableRects.get(container.id) ?? []);
  const target = neighbourColumn(columns, collisionRect.left + collisionRect.width / 2, side);
  if (!target) {
    return undefined;
  }
  const left = target.left + (target.width - collisionRect.width) / 2;
  return { x: currentCoordinates.x + left - collisionRect.left, y: currentCoordinates.y };
};

/**
 * A task that was just given to somebody else. The view shows it with them
 * until the tasks it was made from are replaced by the backend's answer, so
 * that it does not go back for a moment.
 */
interface Given {
  taskId: Id;
  assignee: Assignee | null;
  tasks: Task[];
}

/**
 * Each person and their work: a column for the manager, one for every member
 * of the team and one for the tasks given to nobody. A column says who it is
 * for, what they are doing right now, three figures, and their tasks in three
 * short groups. A task is opened with a press and given to somebody else by
 * dragging it to their column.
 */
export function PeopleView({ project, tasks, everyTask }: KindProps) {
  const navigation = useNavigation();
  const commands = useTaskCommands(project.id);
  const members = useTeamMembers();
  const conversations = useConversationsOfProject();
  const [given, setGiven] = useState<Given | null>(null);
  const [liftedId, setLiftedId] = useState<Id | null>(null);
  const today = todayDate();

  const shown = useMemo(
    () => (given?.tasks === tasks ? withAssignee(tasks, given.taskId, given.assignee) : tasks),
    [given, tasks],
  );
  const columns = useMemo(() => peopleColumns(project, shown, members), [project, shown, members]);
  const tasksById = useMemo(() => indexById(shown), [shown]);
  const lifted = liftedId === null ? undefined : tasksById.get(liftedId);

  const sensors = useSensors(
    // A press only becomes a drag after some movement, so a click opens the task.
    useSensor(PointerSensor, { activationConstraint: { distance: DRAG_START_DISTANCE } }),
    // Space picks a task up and puts it down, the arrows go from column to
    // column. Enter is left free to open the task.
    useSensor(KeyboardSensor, {
      coordinateGetter: toNeighbourColumn,
      keyboardCodes: { start: ['Space'], cancel: ['Escape'], end: ['Space'] },
    }),
  );

  function handleDragStart(event: DragStartEvent) {
    setLiftedId(String(event.active.id));
  }

  function handleDragEnd(event: DragEndEvent) {
    setLiftedId(null);
    const task = tasksById.get(String(event.active.id));
    const column = columns.find((candidate) => candidate.key === event.over?.id);
    const outcome = task ? dropOutcome(task, column) : undefined;
    if (!task || !outcome) {
      return;
    }
    setGiven({ taskId: task.id, assignee: outcome.assignee, tasks });
    void commands.assign(task.id, outcome.assignee);
  }

  if (tasks.length === 0) {
    return (
      <div className={classNames(styles.empty, 'pattern-band')}>
        <p className={styles.emptyLead}>Each person and their work.</p>
        <p className={styles.emptyNote}>
          You and every member of the team get a column here: what they are doing now, what comes
          next and what they finished lately. Give a task to someone and it appears under their
          name.
        </p>
      </div>
    );
  }

  return (
    <DndContext
      sensors={sensors}
      collisionDetection={columnUnder}
      onDragStart={handleDragStart}
      onDragEnd={handleDragEnd}
      onDragCancel={() => {
        setLiftedId(null);
      }}
    >
      <div className={styles.people}>
        {columns.map((column) => (
          <Column
            key={column.key}
            project={project}
            column={column}
            conversations={conversations}
            everyTask={everyTask}
            lifted={lifted}
            today={today}
            openTaskId={navigation.taskId}
            onOpen={navigation.openTask}
          />
        ))}
      </div>

      <DragOverlay dropAnimation={null}>
        {lifted && (
          <div className={styles.inHand}>
            <TaskLineFace project={project} task={lifted} today={today} finished={false} />
          </div>
        )}
      </DragOverlay>
    </DndContext>
  );
}

interface ColumnProps {
  project: Project;
  column: PersonColumn;
  conversations: readonly Conversation[];
  /** Every task of the project, to name the one a specialist works on. */
  everyTask: readonly Task[];
  /** The task in the hand, if one is being dragged. */
  lifted: Task | undefined;
  today: CalendarDate;
  openTaskId: Id | null;
  onOpen: (taskId: Id) => void;
}

/** One person: who they are, what they do now, their figures and their tasks. */
function Column({
  project,
  column,
  conversations,
  everyTask,
  lifted,
  today,
  openTaskId,
  onOpen,
}: ColumnProps) {
  const { setNodeRef, isOver } = useDroppable({
    id: column.key,
    disabled: column.receives === undefined,
  });
  const [isUnfolded, setIsUnfolded] = useState(false);
  const doing = doingOf(project, column.subject, conversations, everyTask);
  const takesTheLifted =
    isOver && lifted !== undefined && dropOutcome(lifted, column) !== undefined;
  // A list one longer than the short one is shown whole: "1 more" would take
  // the same line as the task it hides.
  const canFold = column.next.length > NEXT_SHOWN + 1;
  const nextShown = canFold && !isUnfolded ? column.next.slice(0, NEXT_SHOWN) : column.next;
  const name = nameOf(column.subject);

  const line = (task: Task, finished = false) => (
    <li key={task.id}>
      <TaskLine
        project={project}
        task={task}
        today={today}
        finished={finished}
        isOpenInPanel={openTaskId === task.id}
        onOpen={() => {
          onOpen(task.id);
        }}
      />
    </li>
  );

  return (
    <section
      ref={setNodeRef}
      className={classNames(styles.column, takesTheLifted && styles.dropTarget)}
      aria-label={name}
    >
      <header className={classNames(styles.head, doing.state === 'working' && styles.lit)}>
        <div className={styles.who}>
          <Picture subject={column.subject} name={name} />
          <div className={styles.identity}>
            <span
              className={classNames(
                styles.name,
                (column.subject.kind === 'nobody' || column.subject.kind === 'other') &&
                  styles.quiet,
              )}
            >
              {name}
            </span>
            <span className={styles.role}>{roleOf(column.subject)}</span>
          </div>
        </div>

        <p className={styles.responsibility}>{responsibilityOf(column.subject)}</p>

        <p className={classNames('label', styles.state)} data-state={doing.state}>
          {doing.state === 'working' && (
            <span className={styles.atWork} role="img" aria-label="Working now" />
          )}
          {doing.state === 'waiting' && <span className={styles.waits} aria-hidden />}
          {doing.words}
        </p>

        <p className={styles.figures} aria-label={figuresInWords(column.figures)}>
          <Figure value={column.figures.open} word="open" />
          <Figure value={column.figures.inWork} word="in work" />
          <Figure value={column.figures.finished} word="finished" />
        </p>
      </header>

      <div className={styles.groups}>
        <Group title="In work" count={column.inWork.length} emptyNote="Nothing begun">
          {column.inWork.map((task) => line(task))}
        </Group>

        <Group title="Next" count={column.next.length} emptyNote="Nothing waits to start">
          {nextShown.map((task) => line(task))}
          {canFold && (
            <li>
              <button
                type="button"
                className={styles.more}
                aria-expanded={isUnfolded}
                onClick={() => {
                  setIsUnfolded(!isUnfolded);
                }}
              >
                {isUnfolded ? 'Fewer' : `${(column.next.length - NEXT_SHOWN).toString()} more`}
              </button>
            </li>
          )}
        </Group>

        <Group
          title="Finished lately"
          count={column.figures.finished}
          emptyNote="Nothing finished yet"
        >
          {column.finishedLately.map((task) => line(task, true))}
        </Group>
      </div>
    </section>
  );
}

/** One figure at the head of a column: the number, and the word for what it counts. */
function Figure({ value, word }: { value: number; word: string }) {
  return (
    <span className={classNames(styles.figure, value === 0 && styles.nil)} aria-hidden>
      <span className={styles.value}>{value}</span> {word}
    </span>
  );
}

interface GroupProps {
  title: string;
  count: number;
  emptyNote: string;
  children: ReactNode;
}

/** A short group of the tasks of a column, under a small label. */
function Group({ title, count, emptyNote, children }: GroupProps) {
  return (
    <div className={styles.group}>
      <h3 className={classNames('label', styles.groupTitle)}>
        {title}
        <span className={styles.groupCount}>{count}</span>
      </h3>
      {count === 0 ? (
        <p className={styles.nothing}>{emptyNote}</p>
      ) : (
        <ul className={styles.lines}>{children}</ul>
      )}
    </div>
  );
}

/** What somebody is doing at this moment, as the office says it. */
interface Doing {
  state: PlaceState;
  /** "At work on SSN-4", "Free", "Nothing waits for you". Empty where there is nothing to say. */
  words: string;
}

/**
 * What the one a column is for is doing right now, worked out by the office
 * and said in its words: a specialist at work or waiting for the manager,
 * with the task it is about; the manager, with what waits for them.
 */
function doingOf(
  project: Project,
  subject: ColumnSubject,
  conversations: readonly Conversation[],
  everyTask: readonly Task[],
): Doing {
  switch (subject.kind) {
    case 'manager':
      return {
        state: 'manager',
        words: placeOfManager({ project, tasks: everyTask, conversations }).stateInWords,
      };
    case 'member': {
      const place = placeOfMember(subject.member, {
        project,
        tasks: everyTask,
        conversations,
        now: new Date().toISOString(),
      });
      if (place.state === 'person') {
        return { state: 'person', words: '' };
      }
      const about = place.subject?.reference ?? (place.subject ? 'a question of yours' : null);
      return {
        state: place.state,
        words: about === null ? place.stateInWords : `${place.stateInWords} on ${about}`,
      };
    }
    case 'nobody':
    case 'other':
      return { state: 'free', words: '' };
  }
}

function nameOf(subject: ColumnSubject): string {
  switch (subject.kind) {
    case 'manager':
      return MANAGER_NAME;
    case 'member':
      return subject.member.name;
    case 'nobody':
      return NOBODY_NAME;
    case 'other':
      return 'Not on the team';
  }
}

/** What somebody is, in the words of the team: "AI specialist, Sonnet". */
function roleOf(subject: ColumnSubject): string {
  switch (subject.kind) {
    case 'manager':
      return 'Manager';
    case 'member':
      return [kindLabel(subject.member.kind), profileInWords(subject.member)]
        .filter(Boolean)
        .join(', ');
    case 'nobody':
      return 'Given to nobody';
    case 'other':
      return 'Given to members who left';
  }
}

function responsibilityOf(subject: ColumnSubject): string {
  switch (subject.kind) {
    case 'manager':
      return 'You own the project and make the decisions.';
    case 'member':
      return subject.member.responsibility;
    case 'nobody':
      return 'Drag a task here to take it from whoever has it.';
    case 'other':
      return 'Drag a task to somebody on the team to give it to them.';
  }
}

/** The portrait of whoever a column is for, or an empty frame for nobody. */
function Picture({ subject, name }: { subject: ColumnSubject; name: string }) {
  switch (subject.kind) {
    case 'manager':
      return <Portrait sitter={{ kind: 'manager' }} name={name} />;
    case 'member':
      return (
        <Portrait
          sitter={{ kind: 'member', member: subject.member }}
          name={name}
          isPerson={subject.member.kind === 'human'}
        />
      );
    case 'nobody':
    case 'other':
      return <span className={styles.nobody} aria-hidden />;
  }
}

interface TaskLineProps {
  project: Project;
  task: Task;
  today: CalendarDate;
  /** Whether the line is in the group of finished tasks: it then says when. */
  finished: boolean;
  isOpenInPanel: boolean;
  onOpen: () => void;
}

/**
 * A task in a column. A press or Enter opens it; dragging it, or Space and
 * the arrow keys, gives it to somebody else.
 */
function TaskLine({ project, task, today, finished, isOpenInPanel, onOpen }: TaskLineProps) {
  const { attributes, listeners, setNodeRef, isDragging } = useDraggable({ id: task.id });

  function handleKeyDown(event: KeyboardEvent<HTMLDivElement>) {
    if (event.key === 'Enter') {
      onOpen();
      return;
    }
    listeners?.onKeyDown?.(event);
  }

  return (
    <div
      ref={setNodeRef}
      {...attributes}
      {...listeners}
      className={classNames(
        styles.task,
        isDragging && styles.lifted,
        isOpenInPanel && styles.isOpen,
      )}
      aria-roledescription="task, can be given to somebody else"
      onKeyDown={handleKeyDown}
      onClick={onOpen}
    >
      <TaskLineFace project={project} task={task} today={today} finished={finished} />
    </div>
  );
}

interface TaskLineFaceProps {
  project: Project;
  task: Task;
  today: CalendarDate;
  finished: boolean;
}

/**
 * What a task line shows: the mark of its status and its title, then its
 * key and its date. A task a specialist works on right now carries the light
 * of a card on the board that is being worked on.
 */
function TaskLineFace({ project, task, today, finished }: TaskLineFaceProps) {
  const status = statusOf(project, task);
  const isWorkedOn = useIsBeingWorkedOn(task.id);
  const waitsForManager = useWaitsForManager(task.id);
  const isLate = !finished && isOverdue(task, today);

  return (
    <span
      className={classNames(styles.face, isWorkedOn && styles.working)}
      data-progress={status?.category}
    >
      {isWorkedOn && <span className={styles.edge} aria-hidden />}
      <span className={styles.title}>
        {status && (
          <span className={styles.mark} role="img" aria-label={status.name} title={status.name}>
            <StatusMark status={status} />
          </span>
        )}
        <span className={styles.titleText}>{task.title}</span>
      </span>
      <span className={styles.facts}>
        <span className={styles.reference}>{taskReference(project, task)}</span>
        <TaskDate task={task} today={today} finished={finished} isLate={isLate} />
        {isWorkedOn && (
          <span
            className={styles.atWork}
            role="img"
            aria-label="A specialist is working on this task"
            title="A specialist is working on this task"
          />
        )}
        {waitsForManager && (
          <span className={styles.waitsForYou} title="A specialist waits for your decision">
            <span className={styles.waits} aria-hidden />
            Waits for you
          </span>
        )}
      </span>
    </span>
  );
}

interface TaskDateProps {
  task: Task;
  today: CalendarDate;
  finished: boolean;
  isLate: boolean;
}

/** When a finished task was finished, or when an open one is due. */
function TaskDate({ task, today, finished, isLate }: TaskDateProps) {
  if (finished) {
    return task.completedAt === null ? null : (
      <span className={styles.date}>{formatDate(calendarDateOf(new Date(task.completedAt)))}</span>
    );
  }
  if (task.dueOn === null) {
    return null;
  }
  return (
    <span className={classNames(styles.date, isLate && styles.late)} title="Due date">
      {task.dueOn === today ? 'Due today' : formatDate(task.dueOn)}
    </span>
  );
}
