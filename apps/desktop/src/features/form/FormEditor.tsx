import {
  DndContext,
  KeyboardSensor,
  PointerSensor,
  closestCenter,
  useSensor,
  useSensors,
  type Announcements,
  type DragEndEvent,
  type UniqueIdentifier,
} from '@dnd-kit/core';
import {
  SortableContext,
  sortableKeyboardCoordinates,
  useSortable,
  verticalListSortingStrategy,
} from '@dnd-kit/sortable';
import { CSS } from '@dnd-kit/utilities';
import { GripVertical, Plus, X } from 'lucide-react';
import { useState, type KeyboardEvent } from 'react';

import type { Project } from '@/shared/api/types';
import { classNames } from '@/shared/lib/classNames';
import { Button, IconButton } from '@/shared/ui/Button';
import { ActionMenu } from '@/shared/ui/Menu';

import {
  INTRODUCTION_MAX_LENGTH,
  LABEL_MAX_LENGTH,
  TITLE_MAX_LENGTH,
  addField,
  introduceForm,
  missingProperties,
  moveField,
  relabelField,
  removeField,
  requireField,
  retitleForm,
  usualLabel,
  type FieldProperty,
  type FormField,
  type TaskForm,
} from './formModel';
import styles from './FormView.module.css';

/** How far the pointer must move before a press on a grip becomes a drag, in pixels. */
const DRAG_START_DISTANCE = 4;

/** What goes into a field of each property, shown where the field will be. */
const WHAT_GOES_IN: Record<FieldProperty, string> = {
  title: 'A line of text',
  description: 'A few lines of text',
  assignee: 'Nobody, you, or a member of the team',
  priority: 'Urgent, high, normal or low',
  startOn: 'A day from the calendar',
  dueOn: 'A day from the calendar',
  points: 'A whole number',
  estimate: 'A length of time, like 2h',
};

interface FormEditorProps {
  project: Project;
  form: TaskForm;
  onChange: (how: (form: TaskForm) => TaskForm) => void;
}

/**
 * The form with handles: its title and introduction written in place, and
 * each field relabelled in place, marked required, removed, or moved by
 * its grip, with the pointer or with the keyboard.
 */
export function FormEditor({ project, form, onChange }: FormEditorProps) {
  const sensors = useSensors(
    useSensor(PointerSensor, { activationConstraint: { distance: DRAG_START_DISTANCE } }),
    useSensor(KeyboardSensor, { coordinateGetter: sortableKeyboardCoordinates }),
  );
  const missing = missingProperties(form);

  function labelOf(id: UniqueIdentifier | undefined): string {
    return form.fields.find((field) => field.property === id)?.label ?? 'The field';
  }

  function placeOf(id: UniqueIdentifier | undefined): string {
    const index = form.fields.findIndex((field) => field.property === id);
    return `${(index + 1).toString()} of ${form.fields.length.toString()}`;
  }

  const announcements: Announcements = {
    onDragStart: ({ active }) => `${labelOf(active.id)} is lifted, at place ${placeOf(active.id)}.`,
    onDragOver: ({ active, over }) =>
      over ? `${labelOf(active.id)} is over place ${placeOf(over.id)}.` : undefined,
    onDragEnd: ({ active, over }) =>
      over
        ? `${labelOf(active.id)} is put down at place ${placeOf(over.id)}.`
        : `${labelOf(active.id)} is put down where it was.`,
    onDragCancel: ({ active }) => `${labelOf(active.id)} is back where it was.`,
  };

  function handleDragEnd(event: DragEndEvent) {
    const { active, over } = event;
    if (!over || over.id === active.id) {
      return;
    }
    const place = form.fields.findIndex((field) => field.property === over.id);
    onChange((before) => moveField(before, active.id as FieldProperty, place));
  }

  return (
    <div className={classNames(styles.sheet, styles.editing)}>
      <header className={styles.head}>
        <InPlaceText
          key={`title:${form.title}`}
          className={styles.titleInput}
          label="Title of the form"
          stored={form.title}
          maxLength={TITLE_MAX_LENGTH}
          onKeep={(title) => {
            onChange((before) => retitleForm(before, title));
          }}
        />
        <p className={styles.purpose}>What is sent here becomes a task in {project.name}.</p>
        <InPlaceText
          key={`introduction:${form.introduction}`}
          className={styles.introductionInput}
          label="Introduction of the form"
          stored={form.introduction}
          maxLength={INTRODUCTION_MAX_LENGTH}
          placeholder="A few words of introduction, if the form needs them."
          isLong
          onKeep={(introduction) => {
            onChange((before) => introduceForm(before, introduction));
          }}
        />
      </header>

      <DndContext
        sensors={sensors}
        collisionDetection={closestCenter}
        onDragEnd={handleDragEnd}
        accessibility={{
          announcements,
          screenReaderInstructions: {
            draggable:
              'Press Space to lift the field. Move it with the arrow keys, press Space again to put it down, or Escape to put it back.',
          },
        }}
      >
        <SortableContext
          items={form.fields.map((field) => field.property)}
          strategy={verticalListSortingStrategy}
        >
          <ol className={styles.fields} aria-label="Fields of the form">
            {form.fields.map((field) => (
              <EditedField key={field.property} field={field} onChange={onChange} />
            ))}
          </ol>
        </SortableContext>
      </DndContext>

      {missing.length > 0 && (
        <div className={styles.adder}>
          <ActionMenu
            label="Add a field"
            align="start"
            actions={missing.map((property) => ({
              label: usualLabel(property),
              detail: WHAT_GOES_IN[property],
              onSelect: () => {
                onChange((before) => addField(before, property));
              },
            }))}
          >
            <Button variant="ghost" icon={<Plus />}>
              Add a field
            </Button>
          </ActionMenu>
        </div>
      )}

      <footer className={styles.foot}>
        <p className={styles.outcome}>
          <span className={styles.outcomeWord}>The form is sent from "Fill in".</span>
        </p>
        <Button variant="primary" disabled>
          Send
        </Button>
      </footer>
    </div>
  );
}

interface EditedFieldProps {
  field: FormField;
  onChange: (how: (form: TaskForm) => TaskForm) => void;
}

/** One field with its handles: a grip, its label in place, required or not, and removal. */
function EditedField({ field, onChange }: EditedFieldProps) {
  const {
    attributes,
    listeners,
    setNodeRef,
    setActivatorNodeRef,
    transform,
    transition,
    isDragging,
  } = useSortable({ id: field.property });
  const isTitle = field.property === 'title';

  return (
    <li
      ref={setNodeRef}
      className={classNames(styles.row, styles.editedRow, isDragging && styles.lifted)}
      style={{ transform: CSS.Translate.toString(transform), transition }}
    >
      <button
        ref={setActivatorNodeRef}
        type="button"
        className={styles.grip}
        aria-label={`Move ${field.label}`}
        {...attributes}
        {...listeners}
      >
        <GripVertical />
      </button>
      <div className={styles.labelLine}>
        <InPlaceText
          key={`label:${field.label}`}
          className={styles.labelInput}
          label={`Label of the field ${usualLabel(field.property)}`}
          stored={field.label}
          maxLength={LABEL_MAX_LENGTH}
          onKeep={(label) => {
            onChange((before) => relabelField(before, field.property, label));
          }}
        />
        {field.label !== usualLabel(field.property) && (
          <span className={styles.property}>
            Fills the {usualLabel(field.property).toLowerCase()}
          </span>
        )}
        <button
          type="button"
          className={classNames(styles.requiredToggle, field.required && styles.isRequired)}
          aria-pressed={field.required}
          aria-label={`${field.label} is required`}
          disabled={isTitle}
          title={isTitle ? 'A task always has a title.' : undefined}
          onClick={() => {
            onChange((before) => requireField(before, field.property, !field.required));
          }}
        >
          {field.required ? 'Required' : 'Optional'}
        </button>
        {isTitle ? (
          <span className={styles.removeSpace} />
        ) : (
          <IconButton
            label={`Remove ${field.label}`}
            className={styles.remove}
            onClick={() => {
              onChange((before) => removeField(before, field.property));
            }}
          >
            <X />
          </IconButton>
        )}
      </div>
      <div
        className={classNames(styles.preview, field.property === 'description' && styles.tall)}
        aria-hidden
      >
        {WHAT_GOES_IN[field.property]}
      </div>
    </li>
  );
}

interface InPlaceTextProps {
  className: string | undefined;
  /** What the text is, for screen readers. */
  label: string;
  stored: string;
  maxLength: number;
  placeholder?: string;
  /** Several lines: Enter then starts a new line, and Ctrl+Enter keeps it. */
  isLong?: boolean;
  onKeep: (text: string) => void;
}

/** Words written in place. Enter or leaving keeps them; Escape gives them up. */
function InPlaceText({
  className,
  label,
  stored,
  maxLength,
  placeholder,
  isLong = false,
  onKeep,
}: InPlaceTextProps) {
  const [draft, setDraft] = useState(stored);

  function keep() {
    if (draft !== stored) {
      onKeep(draft);
      // What is kept comes back as a new stored text, which starts the field
      // again; text that is not kept, such as an empty title, goes back.
      setDraft(stored);
    }
  }

  function handleKeyDown(event: KeyboardEvent<HTMLInputElement | HTMLTextAreaElement>) {
    if (event.key === 'Enter' && (!isLong || event.ctrlKey || event.metaKey)) {
      event.preventDefault();
      event.currentTarget.blur();
    } else if (event.key === 'Escape') {
      event.preventDefault();
      setDraft(stored);
    }
  }

  const common = {
    className,
    value: draft,
    maxLength,
    placeholder,
    'aria-label': label,
    spellCheck: true,
    onBlur: keep,
    onKeyDown: handleKeyDown,
  };

  return isLong ? (
    <textarea
      {...common}
      rows={1}
      onChange={(event) => {
        setDraft(event.target.value);
      }}
    />
  ) : (
    <input
      {...common}
      autoComplete="off"
      onChange={(event) => {
        setDraft(event.target.value);
      }}
    />
  );
}
