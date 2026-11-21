import { ArrowDown, ArrowUp, Plus, Trash2, X } from 'lucide-react';
import { useMemo, useState, type FormEvent, type KeyboardEvent } from 'react';

import type { Color, CustomField, FieldKind, Id, Project, Task } from '@/shared/api/types';
import { Button, IconButton, ValueButton } from '@/shared/ui/Button';
import { Dialog } from '@/shared/ui/Dialog';
import { TextInput } from '@/shared/ui/Field';
import { FormActions } from '@/shared/ui/Form';
import { ChoiceMenu } from '@/shared/ui/Menu';

import { TAG_COLORS, colorName } from '../tags/tagModel';
import { useTasks } from '../tasks/taskQueries';
import { FIELD_KINDS, inOrder, kindLabel } from './fieldModel';
import { useFieldCommands, useFields, type FieldCommands } from './fieldQueries';
import styles from './FieldsDialog.module.css';

/** The fields while they are loading. */
const NO_FIELDS: readonly CustomField[] = [];
/** The tasks while they are loading. */
const NO_TASKS: readonly Task[] = [];

interface FieldsDialogProps {
  open: boolean;
  project: Project;
  onClose: () => void;
}

/**
 * The dialog in which the custom fields of a project are added, renamed,
 * reordered and removed, and the choices of a choice field changed.
 */
export function FieldsDialog({ open, project, onClose }: FieldsDialogProps) {
  return (
    <Dialog open={open} title="Fields" size="wide" onClose={onClose}>
      <FieldsEditor project={project} onDone={onClose} />
    </Dialog>
  );
}

/** How many tasks hold a value of each field, and of each choice. */
function countsOf(tasks: readonly Task[]): Map<Id, number> {
  const counts = new Map<Id, number>();
  const count = (id: Id) => counts.set(id, (counts.get(id) ?? 0) + 1);
  for (const task of tasks) {
    for (const [fieldId, value] of Object.entries(task.fieldValues)) {
      count(fieldId);
      if (value.kind === 'choice') {
        count(value.choiceId);
      }
    }
  }
  return counts;
}

/** "1 task holds", "3 tasks hold". */
function holders(count: number): string {
  return count === 1 ? '1 task holds' : `${count.toString()} tasks hold`;
}

/** The fields of a project as a list that is edited in place; every change is saved as it is made. */
function FieldsEditor({ project, onDone }: { project: Project; onDone: () => void }) {
  const fields = inOrder(useFields(project.id).data ?? NO_FIELDS);
  const commands = useFieldCommands(project.id);
  const tasks = useTasks(project.id).data ?? NO_TASKS;
  const counts = useMemo(() => countsOf(tasks), [tasks]);
  /** The field that asks whether it should really go. */
  const [leavingId, setLeavingId] = useState<Id | null>(null);

  return (
    <div className={styles.editor}>
      <p className={styles.lead}>
        Properties every task of the project can hold, such as a supplier, a cost or a datasheet.
        Each is a column of the table, and a view can show only the tasks with one choice or one
        answer. The kind of a field stays as it was made. Changes are saved as you make them.
      </p>

      {fields.length === 0 ? (
        <p className={styles.none}>The project has no fields yet.</p>
      ) : (
        <ul className={styles.fields}>
          {fields.map((field, index) => {
            const count = counts.get(field.id) ?? 0;
            return (
              <li key={field.id} className={styles.field}>
                <div className={styles.row}>
                  <span className={styles.kind}>{kindLabel(field.kind)}</span>
                  <InlineText
                    key={`${field.id}:${field.name}`}
                    text={field.name}
                    label={`Name of the field ${field.name}`}
                    maxLength={40}
                    className={styles.name}
                    onSave={(name) => {
                      void commands.rename(field.id, name);
                    }}
                  />
                  {field.kind === 'number' ? (
                    <InlineText
                      key={`${field.id}:${field.unit ?? ''}`}
                      text={field.unit ?? ''}
                      placeholder="Unit"
                      label={`Unit of ${field.name}`}
                      maxLength={8}
                      allowsEmpty
                      className={styles.unit}
                      onSave={(unit) => {
                        void commands.setUnit(field.id, unit === '' ? null : unit);
                      }}
                    />
                  ) : (
                    <span />
                  )}
                  <span className={styles.count}>{count === 0 ? '' : count}</span>
                  <IconButton
                    label={`Move ${field.name} up`}
                    disabled={index === 0}
                    onClick={() => void commands.move(field.id, index - 1)}
                  >
                    <ArrowUp />
                  </IconButton>
                  <IconButton
                    label={`Move ${field.name} down`}
                    disabled={index === fields.length - 1}
                    onClick={() => void commands.move(field.id, index + 1)}
                  >
                    <ArrowDown />
                  </IconButton>
                  <IconButton
                    label={`Remove ${field.name}`}
                    onClick={() => {
                      if (count === 0) {
                        void commands.remove(field.id);
                      } else {
                        setLeavingId(field.id);
                      }
                    }}
                  >
                    <Trash2 />
                  </IconButton>
                </div>
                {leavingId === field.id && (
                  <div className={styles.question} role="group" aria-label={`Remove ${field.name}`}>
                    <span className={styles.questionText}>
                      {holders(count)} a value of this field. It is cleared from{' '}
                      {count === 1 ? 'it' : 'them'}.
                    </span>
                    <span className={styles.space} />
                    <Button variant="ghost" onClick={() => setLeavingId(null)}>
                      Keep
                    </Button>
                    <Button
                      variant="danger"
                      onClick={() => {
                        setLeavingId(null);
                        void commands.remove(field.id);
                      }}
                    >
                      Remove field
                    </Button>
                  </div>
                )}
                {field.kind === 'choice' && (
                  <ChoicesEditor field={field} counts={counts} commands={commands} />
                )}
              </li>
            );
          })}
        </ul>
      )}

      <NewField commands={commands} />

      <FormActions>
        <Button variant="secondary" onClick={onDone}>
          Done
        </Button>
      </FormActions>
    </div>
  );
}

/** The line at the end of the list that adds a field: its name, its kind and, for a number, its unit. */
function NewField({ commands }: { commands: FieldCommands }) {
  const [name, setName] = useState('');
  const [kind, setKind] = useState<FieldKind>('text');
  const [unit, setUnit] = useState('');

  async function add(event: FormEvent) {
    event.preventDefault();
    if (name.trim() === '') {
      return;
    }
    const added = await commands.add(
      name,
      kind,
      kind === 'number' && unit.trim() !== '' ? unit : null,
    );
    if (added) {
      setName('');
      setUnit('');
    }
  }

  return (
    <form className={styles.add} onSubmit={(event) => void add(event)}>
      <Plus aria-hidden className={styles.addMark} />
      <TextInput
        value={name}
        placeholder="New field"
        aria-label="Name of a new field"
        maxLength={40}
        data-escape-undoes
        onChange={(event) => setName(event.target.value)}
        onKeyDown={(event) => {
          if (event.key === 'Escape') {
            setName('');
          }
        }}
      />
      <ChoiceMenu
        label="Kind of the new field"
        choices={FIELD_KINDS.map((entry) => ({
          value: entry.kind,
          label: `${entry.label}: ${entry.hint}`,
        }))}
        value={kind}
        onChoose={setKind}
      >
        <ValueButton className={styles.kindButton} aria-label={`Kind: ${kindLabel(kind)}`}>
          {kindLabel(kind)}
        </ValueButton>
      </ChoiceMenu>
      {kind === 'number' && (
        <TextInput
          className={styles.newUnit}
          value={unit}
          placeholder="Unit"
          aria-label="Unit of the new field"
          maxLength={8}
          onChange={(event) => setUnit(event.target.value)}
        />
      )}
      <Button variant="secondary" type="submit" disabled={name.trim() === ''}>
        Add field
      </Button>
    </form>
  );
}

interface ChoicesEditorProps {
  field: CustomField;
  counts: ReadonlyMap<Id, number>;
  commands: FieldCommands;
}

/** The choices of a choice field, set in under it: color, name and removal, and a line to add one. */
function ChoicesEditor({ field, counts, commands }: ChoicesEditorProps) {
  const [newName, setNewName] = useState('');

  async function add(event: FormEvent) {
    event.preventDefault();
    if (newName.trim() === '') {
      return;
    }
    if (await commands.addChoice(field.id, newName)) {
      setNewName('');
    }
  }

  return (
    <div className={styles.choices}>
      {field.choices.length > 0 && (
        <ul className={styles.choiceList}>
          {field.choices.map((choice) => {
            const count = counts.get(choice.id) ?? 0;
            return (
              <li key={choice.id} className={styles.choice}>
                <ChoiceMenu
                  label={`The color of ${choice.name}`}
                  choices={TAG_COLORS.map((color) => ({
                    value: color,
                    label: colorName(color),
                    icon: <span className={styles.swatch} data-tone={color} />,
                  }))}
                  value={choice.color}
                  onChoose={(color: Color) =>
                    void commands.recolorChoice(field.id, choice.id, color)
                  }
                >
                  <button
                    type="button"
                    className={styles.color}
                    aria-label={`Color of ${choice.name}: ${colorName(choice.color)}`}
                    title={colorName(choice.color)}
                  >
                    <span className={styles.swatch} data-tone={choice.color} />
                  </button>
                </ChoiceMenu>
                <InlineText
                  key={`${choice.id}:${choice.name}`}
                  text={choice.name}
                  label={`Name of the choice ${choice.name}`}
                  maxLength={30}
                  className={styles.name}
                  onSave={(name) => void commands.renameChoice(field.id, choice.id, name)}
                />
                <span
                  className={styles.count}
                  title={count === 0 ? undefined : `${holders(count)} it`}
                >
                  {count === 0 ? '' : count}
                </span>
                <IconButton
                  label={
                    count === 0
                      ? `Remove ${choice.name}`
                      : `Remove ${choice.name}: ${holders(count)} it, and ${count === 1 ? 'it is' : 'they are'} cleared`
                  }
                  onClick={() => void commands.removeChoice(field.id, choice.id)}
                >
                  <X />
                </IconButton>
              </li>
            );
          })}
        </ul>
      )}
      <form className={styles.addChoice} onSubmit={(event) => void add(event)}>
        <Plus aria-hidden className={styles.addMark} />
        <TextInput
          value={newName}
          placeholder="New choice"
          aria-label={`Name of a new choice of ${field.name}`}
          maxLength={30}
          data-escape-undoes
          onChange={(event) => setNewName(event.target.value)}
          onKeyDown={(event) => {
            if (event.key === 'Escape') {
              setNewName('');
            }
          }}
        />
        <Button variant="ghost" type="submit" disabled={newName.trim() === ''}>
          Add choice
        </Button>
      </form>
    </div>
  );
}

interface InlineTextProps {
  text: string;
  label: string;
  maxLength: number;
  className?: string;
  placeholder?: string;
  /** Whether an empty text is saved, as for a unit taken away. */
  allowsEmpty?: boolean;
  onSave: (text: string) => void;
}

/** A name edited in place. Enter or leaving the line saves; Escape undoes. */
function InlineText({
  text,
  label,
  maxLength,
  className,
  placeholder,
  allowsEmpty = false,
  onSave,
}: InlineTextProps) {
  const [draft, setDraft] = useState(text);

  function save() {
    const tidyDraft = draft.trim();
    if (tidyDraft === '' && !allowsEmpty) {
      setDraft(text);
    } else if (tidyDraft !== text) {
      onSave(tidyDraft);
    }
  }

  function handleKeyDown(event: KeyboardEvent<HTMLInputElement>) {
    if (event.key === 'Enter') {
      event.currentTarget.blur();
    } else if (event.key === 'Escape') {
      setDraft(text);
    }
  }

  return (
    <input
      className={className}
      value={draft}
      placeholder={placeholder}
      maxLength={maxLength}
      aria-label={label}
      data-escape-undoes
      onChange={(event) => setDraft(event.target.value)}
      onBlur={save}
      onKeyDown={handleKeyDown}
    />
  );
}
