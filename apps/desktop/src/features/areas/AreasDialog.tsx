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

import type { Area, Id, Project } from '@/shared/api/types';
import { classNames } from '@/shared/lib/classNames';
import { Button, IconButton, ValueButton } from '@/shared/ui/Button';
import { Dialog } from '@/shared/ui/Dialog';
import { FormActions } from '@/shared/ui/Form';
import { ChoiceMenu } from '@/shared/ui/Menu';

import { useTasks } from '../tasks/taskQueries';
import styles from './AreasDialog.module.css';
import { likelyAreaForTasksOf, nameForNewArea, taskCountsByArea } from './areaModel';
import { useAreaCommands, useAreas } from './areaQueries';

/** How far the pointer must move before a press on a grip becomes a drag, in pixels. */
const DRAG_START_DISTANCE = 4;

/** Why the last area cannot be removed. */
const MUST_STAY = 'A project keeps at least one area. Add another one first.';

/** The areas while they are loading. */
const NO_AREAS: readonly Area[] = [];

interface AreasDialogProps {
  open: boolean;
  project: Project;
  onClose: () => void;
}

/** The dialog in which the areas of a project are added, renamed, ordered and removed. */
export function AreasDialog({ open, project, onClose }: AreasDialogProps) {
  return (
    <Dialog open={open} title="Areas" size="wide" onClose={onClose}>
      <AreasEditor project={project} onDone={onClose} />
    </Dialog>
  );
}

/**
 * The areas of a project as a list that can be edited in place. Every
 * change is stored as it is made, and the navigation at the side follows.
 */
function AreasEditor({ project, onDone }: { project: Project; onDone: () => void }) {
  const areas = useAreas(project.id).data ?? NO_AREAS;
  const commands = useAreaCommands(project.id);
  const tasks = useTasks(project.id);
  /** The area that is asked where its tasks should go before it is removed. */
  const [leavingId, setLeavingId] = useState<Id | null>(null);
  /** The area that was just added. Its name is ready to be typed over. */
  const [newestId, setNewestId] = useState<Id | null>(null);

  const taskCounts = useMemo(() => taskCountsByArea(tasks.data ?? []), [tasks.data]);

  const sensors = useSensors(
    useSensor(PointerSensor, { activationConstraint: { distance: DRAG_START_DISTANCE } }),
    useSensor(KeyboardSensor, { coordinateGetter: sortableKeyboardCoordinates }),
  );

  function handleDragEnd(event: DragEndEvent) {
    if (!event.over || event.over.id === event.active.id) {
      return;
    }
    const place = areas.findIndex((area) => area.id === event.over?.id);
    if (place !== -1) {
      void commands.move(String(event.active.id), place);
    }
  }

  async function addArea() {
    const added = await commands.add(nameForNewArea(areas));
    if (added) {
      setNewestId(added.id);
    }
  }

  return (
    <div className={styles.editor}>
      <p className={styles.lead}>
        The jobs this project is cut into, such as the electronics, the firmware and the enclosure
        of a device. Every task is in one area. With two or more, they appear under the project at
        the side, and each shows only its own tasks. Changes are saved as you make them.
      </p>

      <div className={styles.headings} aria-hidden>
        <span className="label">Area</span>
        <span className="label">Tasks</span>
      </div>

      <DndContext sensors={sensors} collisionDetection={closestCenter} onDragEnd={handleDragEnd}>
        <SortableContext
          items={areas.map((area) => area.id)}
          strategy={verticalListSortingStrategy}
        >
          <ul className={styles.areas}>
            {areas.map((area) => (
              <AreaRow
                key={area.id}
                project={project}
                areas={areas}
                area={area}
                taskCount={taskCounts.get(area.id) ?? 0}
                isNewest={area.id === newestId}
                isLeaving={area.id === leavingId}
                onAskWhereTasksGo={() => {
                  setLeavingId(area.id);
                }}
                onStay={() => {
                  setLeavingId(null);
                }}
                onNamed={() => {
                  // Once named, the new area no longer takes the focus back.
                  setNewestId(null);
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
          void addArea();
        }}
      >
        <Plus />
        Add area
      </button>

      <FormActions>
        <Button variant="secondary" onClick={onDone}>
          Done
        </Button>
      </FormActions>
    </div>
  );
}

interface AreaRowProps {
  project: Project;
  /** Every area of the project, in its order. */
  areas: readonly Area[];
  area: Area;
  /** How many tasks are in the area, subtasks included. */
  taskCount: number;
  /** Whether the area was just added. Its name then takes the focus. */
  isNewest: boolean;
  /** Whether the row asks where the tasks of the area should go. */
  isLeaving: boolean;
  onAskWhereTasksGo: () => void;
  onStay: () => void;
  /** Called when the name of the area has been edited. */
  onNamed: () => void;
}

/** One area: a grip to move it, its name, how many tasks it holds, and a way to remove it. */
function AreaRow({
  project,
  areas,
  area,
  taskCount,
  isNewest,
  isLeaving,
  onAskWhereTasksGo,
  onStay,
  onNamed,
}: AreaRowProps) {
  const commands = useAreaCommands(project.id);
  const {
    attributes,
    listeners,
    setNodeRef,
    setActivatorNodeRef,
    transform,
    transition,
    isDragging,
  } = useSortable({ id: area.id });
  const mustStay = areas.length <= 1;

  return (
    <li
      ref={setNodeRef}
      className={classNames(styles.area, isDragging && styles.lifted)}
      style={{ transform: CSS.Translate.toString(transform), transition }}
    >
      <div className={styles.row}>
        <button
          ref={setActivatorNodeRef}
          type="button"
          className={styles.grip}
          aria-label={`Move ${area.name}`}
          {...attributes}
          {...listeners}
        >
          <GripVertical />
        </button>

        <NameEditor
          key={`${area.id}:${area.name}`}
          name={area.name}
          takesFocus={isNewest}
          onRename={(name) => {
            onNamed();
            void commands.rename(area.id, name);
          }}
        />

        <span className={styles.count}>{taskCount === 0 ? '' : taskCount}</span>

        <IconButton
          label={mustStay ? MUST_STAY : `Remove ${area.name}`}
          disabled={mustStay}
          onClick={() => {
            if (taskCount === 0) {
              void commands.remove(area.id);
            } else {
              onAskWhereTasksGo();
            }
          }}
        >
          <Trash2 />
        </IconButton>
      </div>

      {isLeaving && (
        <WhereTasksGo
          project={project}
          areas={areas}
          area={area}
          taskCount={taskCount}
          onStay={onStay}
        />
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

/** The name of an area, edited in place. Enter or leaving the field saves; Escape undoes. */
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
      aria-label="Name of the area"
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
  areas: readonly Area[];
  /** The area that is to be removed. */
  area: Area;
  taskCount: number;
  onStay: () => void;
}

/**
 * The question an area with tasks asks before it is removed: which area its
 * tasks should go to.
 */
function WhereTasksGo({ project, areas, area, taskCount, onStay }: WhereTasksGoProps) {
  const commands = useAreaCommands(project.id);
  const others = areas.filter((other) => other.id !== area.id);
  const [targetId, setTargetId] = useState<Id | undefined>(likelyAreaForTasksOf(areas, area)?.id);
  const target = others.find((other) => other.id === targetId);

  async function remove() {
    const wasRemoved = await commands.remove(area.id, targetId);
    if (wasRemoved) {
      onStay();
    }
  }

  return (
    <div className={styles.question} role="group" aria-label={`Remove ${area.name}`}>
      <span className={styles.questionText}>
        {taskCount === 1 ? '1 task is' : `${taskCount.toString()} tasks are`} in this area. Move{' '}
        {taskCount === 1 ? 'it' : 'them'} to
      </span>
      <ChoiceMenu
        label="The area the tasks go to"
        choices={others.map((other) => ({ value: other.id, label: other.name }))}
        value={targetId ?? ''}
        onChoose={setTargetId}
      >
        <ValueButton className={styles.target} aria-label="The area the tasks go to">
          {target ? target.name : 'Choose an area'}
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
        Remove area
      </Button>
    </div>
  );
}
