import {
  DndContext,
  KeyboardSensor,
  PointerSensor,
  closestCenter,
  useSensor,
  useSensors,
  type DragEndEvent,
} from '@dnd-kit/core';
import {
  SortableContext,
  sortableKeyboardCoordinates,
  useSortable,
  verticalListSortingStrategy,
} from '@dnd-kit/sortable';
import { CSS } from '@dnd-kit/utilities';
import { GripVertical, Plus, Trash2 } from 'lucide-react';
import { useEffect, useMemo, useRef, useState, type KeyboardEvent } from 'react';

import type { Color, Id, Project, Status, StatusCategory } from '@/shared/api/types';
import { classNames } from '@/shared/lib/classNames';
import { Button, IconButton, ValueButton } from '@/shared/ui/Button';
import { Dialog } from '@/shared/ui/Dialog';
import { FormActions } from '@/shared/ui/Form';
import { ChoiceMenu, type Choice } from '@/shared/ui/Menu';

import { StatusLabel, StatusMark } from '../tasks/TaskBadges';
import { useTasks } from '../tasks/taskQueries';
import styles from './StatusesDialog.module.css';
import { useProjectCommands } from './projectQueries';
import {
  CATEGORIES,
  STATUS_COLORS,
  categoryLabel,
  colorName,
  isTheOnlyOneOfItsKind,
  suggestedStatus,
} from './workflowModel';

/** How far the pointer must move before a press on a grip becomes a drag, in pixels. */
const DRAG_START_DISTANCE = 4;

/** Why the only status of a kind a workflow needs cannot be removed or given another meaning. */
const MUST_STAY =
  'A project keeps one status in which work has not started and one in which it is done. Add another one of this kind first.';

interface StatusesDialogProps {
  open: boolean;
  project: Project;
  onClose: () => void;
}

/** The dialog in which the statuses of a project are added, changed, ordered and removed. */
export function StatusesDialog({ open, project, onClose }: StatusesDialogProps) {
  return (
    <Dialog open={open} title="Statuses" size="wide" onClose={onClose}>
      <StatusesEditor project={project} onDone={onClose} />
    </Dialog>
  );
}

/**
 * The statuses of a project as a list that can be edited in place. Every
 * change is stored as it is made, and the board behind the dialog follows.
 */
function StatusesEditor({ project, onDone }: { project: Project; onDone: () => void }) {
  const commands = useProjectCommands();
  const tasks = useTasks(project.id);
  /** The status that is asked where its tasks should go before it is removed. */
  const [leavingId, setLeavingId] = useState<Id | null>(null);
  /** The status that was just added. Its name is ready to be typed over. */
  const [newestId, setNewestId] = useState<Id | null>(null);

  const taskCounts = useMemo(() => {
    const counts = new Map<Id, number>();
    for (const task of tasks.data ?? []) {
      counts.set(task.statusId, (counts.get(task.statusId) ?? 0) + 1);
    }
    return counts;
  }, [tasks.data]);

  const sensors = useSensors(
    useSensor(PointerSensor, { activationConstraint: { distance: DRAG_START_DISTANCE } }),
    useSensor(KeyboardSensor, { coordinateGetter: sortableKeyboardCoordinates }),
  );

  function handleDragEnd(event: DragEndEvent) {
    if (!event.over || event.over.id === event.active.id) {
      return;
    }
    const place = project.statuses.findIndex((status) => status.id === event.over?.id);
    if (place !== -1) {
      void commands.moveStatus(project, String(event.active.id), place);
    }
  }

  async function addStatus() {
    const added = await commands.addStatus(project, suggestedStatus(project.statuses));
    if (added) {
      setNewestId(added);
    }
  }

  return (
    <div className={styles.editor}>
      <p className={styles.lead}>
        The steps a task of this project goes through, in the order of the board. Changes are saved
        as you make them.
      </p>

      <div className={styles.headings} aria-hidden>
        <span className="label">Status</span>
        <span className="label">Means</span>
        <span className="label">Tasks</span>
      </div>

      <DndContext sensors={sensors} collisionDetection={closestCenter} onDragEnd={handleDragEnd}>
        <SortableContext
          items={project.statuses.map((status) => status.id)}
          strategy={verticalListSortingStrategy}
        >
          <ul className={styles.statuses}>
            {project.statuses.map((status) => (
              <StatusRow
                key={status.id}
                project={project}
                status={status}
                taskCount={taskCounts.get(status.id) ?? 0}
                isNewest={status.id === newestId}
                isLeaving={status.id === leavingId}
                onAskWhereTasksGo={() => {
                  setLeavingId(status.id);
                }}
                onStay={() => {
                  setLeavingId(null);
                }}
              />
            ))}
          </ul>
        </SortableContext>
      </DndContext>

      <button
        type="button"
        className={styles.add}
        onClick={() => {
          void addStatus();
        }}
      >
        <Plus />
        Add status
      </button>

      <FormActions>
        <Button variant="secondary" onClick={onDone}>
          Done
        </Button>
      </FormActions>
    </div>
  );
}

interface StatusRowProps {
  project: Project;
  status: Status;
  /** How many tasks are in the status. */
  taskCount: number;
  /** Whether the status was just added. Its name then takes the focus. */
  isNewest: boolean;
  /** Whether the row asks where the tasks of the status should go. */
  isLeaving: boolean;
  onAskWhereTasksGo: () => void;
  onStay: () => void;
}

/** One status: a grip to move it, its color, its name, what it means, and a way to remove it. */
function StatusRow({
  project,
  status,
  taskCount,
  isNewest,
  isLeaving,
  onAskWhereTasksGo,
  onStay,
}: StatusRowProps) {
  const commands = useProjectCommands();
  const {
    attributes,
    listeners,
    setNodeRef,
    setActivatorNodeRef,
    transform,
    transition,
    isDragging,
  } = useSortable({ id: status.id });
  const mustStay = isTheOnlyOneOfItsKind(project.statuses, status);

  function change(changes: Partial<Pick<Status, 'name' | 'category' | 'color'>>) {
    return commands.changeStatus(project.id, status.id, {
      name: status.name,
      category: status.category,
      color: status.color,
      ...changes,
    });
  }

  const colorChoices: Choice<Color>[] = STATUS_COLORS.map((color) => ({
    value: color,
    label: colorName(color),
    icon: <StatusMark status={{ category: status.category, color }} />,
  }));
  const categoryChoices: Choice<StatusCategory>[] = CATEGORIES.map(({ category, label }) => ({
    value: category,
    label,
    icon: <StatusMark status={{ category, color: status.color }} />,
  }));

  return (
    <li
      ref={setNodeRef}
      className={classNames(styles.status, isDragging && styles.lifted)}
      style={{ transform: CSS.Translate.toString(transform), transition }}
    >
      <div className={styles.row}>
        <button
          ref={setActivatorNodeRef}
          type="button"
          className={styles.grip}
          aria-label={`Move ${status.name}`}
          {...attributes}
          {...listeners}
        >
          <GripVertical />
        </button>

        <ChoiceMenu
          label={`Color of ${status.name}`}
          choices={colorChoices}
          value={status.color}
          onChoose={(color) => {
            void change({ color });
          }}
        >
          <IconButton label={`Color: ${colorName(status.color)}`}>
            <StatusMark status={status} />
          </IconButton>
        </ChoiceMenu>

        <NameEditor
          key={`${status.id}:${status.name}`}
          name={status.name}
          takesFocus={isNewest}
          onRename={(name) => {
            void change({ name });
          }}
        />

        <ChoiceMenu
          label={`What ${status.name} means`}
          choices={categoryChoices}
          value={status.category}
          onChoose={(category) => {
            void change({ category });
          }}
        >
          <ValueButton
            className={styles.category}
            aria-label={`What ${status.name} means`}
            disabled={mustStay}
            title={mustStay ? MUST_STAY : undefined}
          >
            {categoryLabel(status.category)}
          </ValueButton>
        </ChoiceMenu>

        <span className={styles.count}>{taskCount === 0 ? '' : taskCount}</span>

        <IconButton
          label={mustStay ? MUST_STAY : `Remove ${status.name}`}
          disabled={mustStay}
          onClick={() => {
            if (taskCount === 0) {
              void commands.removeStatus(project.id, status.id);
            } else {
              onAskWhereTasksGo();
            }
          }}
        >
          <Trash2 />
        </IconButton>
      </div>

      {isLeaving && (
        <WhereTasksGo project={project} status={status} taskCount={taskCount} onStay={onStay} />
      )}
    </li>
  );
}

interface NameEditorProps {
  name: string;
  /** Whether the field takes the focus when it appears, with its text selected. */
  takesFocus: boolean;
  onRename: (name: string) => void;
}

/** The name of a status, edited in place. Enter or leaving the field saves; Escape undoes. */
function NameEditor({ name, takesFocus, onRename }: NameEditorProps) {
  const [draft, setDraft] = useState(name);
  const field = useRef<HTMLInputElement>(null);

  useEffect(() => {
    if (takesFocus) {
      field.current?.focus();
      field.current?.select();
    }
  }, [takesFocus]);

  function save() {
    const tidyDraft = draft.trim();
    if (tidyDraft === '') {
      setDraft(name);
    } else if (tidyDraft !== name) {
      onRename(tidyDraft);
    }
  }

  function handleKeyDown(event: KeyboardEvent<HTMLInputElement>) {
    if (event.key === 'Enter') {
      event.currentTarget.blur();
    } else if (event.key === 'Escape') {
      setDraft(name);
    }
  }

  return (
    <input
      ref={field}
      className={styles.name}
      value={draft}
      maxLength={40}
      aria-label="Name of the status"
      // Escape undoes the edit and leaves the dialog open.
      data-escape-undoes
      onChange={(event) => {
        setDraft(event.target.value);
      }}
      onBlur={save}
      onKeyDown={handleKeyDown}
    />
  );
}

interface WhereTasksGoProps {
  project: Project;
  /** The status that is to be removed. */
  status: Status;
  taskCount: number;
  onStay: () => void;
}

/**
 * The question a status with tasks asks before it is removed: which status
 * its tasks should go to.
 */
function WhereTasksGo({ project, status, taskCount, onStay }: WhereTasksGoProps) {
  const commands = useProjectCommands();
  const others = project.statuses.filter((other) => other.id !== status.id);
  // The status in front of it is the likely place: the work goes one step back.
  const place = project.statuses.findIndex((other) => other.id === status.id);
  const likely = project.statuses[place - 1] ?? project.statuses[place + 1];
  const [targetId, setTargetId] = useState<Id | undefined>(likely?.id);
  const target = others.find((other) => other.id === targetId);

  async function remove() {
    const wasRemoved = await commands.removeStatus(project.id, status.id, targetId);
    if (wasRemoved) {
      onStay();
    }
  }

  return (
    <div className={styles.question} role="group" aria-label={`Remove ${status.name}`}>
      <span className={styles.questionText}>
        {taskCount === 1 ? '1 task is' : `${taskCount.toString()} tasks are`} in this status. Move{' '}
        {taskCount === 1 ? 'it' : 'them'} to
      </span>
      <ChoiceMenu
        label="The status the tasks go to"
        choices={others.map((other) => ({
          value: other.id,
          label: other.name,
          icon: <StatusMark status={other} />,
        }))}
        value={targetId ?? ''}
        onChoose={setTargetId}
      >
        <ValueButton className={styles.target} aria-label="The status the tasks go to">
          {target ? <StatusLabel status={target} /> : 'Choose a status'}
        </ValueButton>
      </ChoiceMenu>
      <span className={styles.space} />
      <Button variant="ghost" onClick={onStay}>
        Keep
      </Button>
      <Button
        variant="danger"
        disabled={target === undefined}
        onClick={() => {
          void remove();
        }}
      >
        Remove status
      </Button>
    </div>
  );
}
