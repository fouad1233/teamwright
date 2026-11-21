import { Plus, Trash2 } from 'lucide-react';
import { useMemo, useState, type FormEvent, type KeyboardEvent } from 'react';

import type { Color, Id, Project, Tag } from '@/shared/api/types';
import { Button, IconButton } from '@/shared/ui/Button';
import { Dialog } from '@/shared/ui/Dialog';
import { TextInput } from '@/shared/ui/Field';
import { FormActions } from '@/shared/ui/Form';
import { ChoiceMenu } from '@/shared/ui/Menu';

import { useTasks } from '../tasks/taskQueries';
import { TAG_COLORS, colorName, taskCountsByTag } from './tagModel';
import { useTagCommands, useTags } from './tagQueries';
import styles from './TagsDialog.module.css';

/** The tags while they are loading. */
const NO_TAGS: readonly Tag[] = [];

interface TagsDialogProps {
  open: boolean;
  project: Project;
  onClose: () => void;
}

/** The dialog in which the tags of a project are added, renamed, recolored and removed. */
export function TagsDialog({ open, project, onClose }: TagsDialogProps) {
  return (
    <Dialog open={open} title="Tags" size="wide" onClose={onClose}>
      <TagsEditor project={project} onDone={onClose} />
    </Dialog>
  );
}

/**
 * The tags of a project as a list that can be edited in place. Every change
 * is stored as it is made, and the cards follow.
 */
function TagsEditor({ project, onDone }: { project: Project; onDone: () => void }) {
  const tags = useTags(project.id).data ?? NO_TAGS;
  const commands = useTagCommands(project.id);
  const tasks = useTasks(project.id);
  const counts = useMemo(() => taskCountsByTag(tasks.data ?? []), [tasks.data]);
  /** The tag that asks whether it should really go, with the tasks it is on. */
  const [leavingId, setLeavingId] = useState<Id | null>(null);
  const [newName, setNewName] = useState('');

  async function add(event: FormEvent) {
    event.preventDefault();
    if (newName.trim() === '') {
      return;
    }
    const added = await commands.add(newName);
    if (added) {
      setNewName('');
    }
  }

  return (
    <div className={styles.editor}>
      <p className={styles.lead}>
        Short words to put on tasks across statuses and areas, such as a supplier, a risk or a test.
        A view can show only the tasks with one tag. Changes are saved as you make them.
      </p>

      <div className={styles.headings} aria-hidden>
        <span className="label">Tag</span>
        <span className="label">Tasks</span>
      </div>

      {tags.length === 0 ? (
        <p className={styles.none}>The project has no tags yet.</p>
      ) : (
        <ul className={styles.tags}>
          {tags.map((tag) => {
            const count = counts.get(tag.id) ?? 0;
            return (
              <li key={tag.id} className={styles.tag}>
                <div className={styles.row}>
                  <ColorChoice
                    tag={tag}
                    onChoose={(color) => {
                      void commands.recolor(tag.id, color);
                    }}
                  />
                  <NameEditor
                    key={`${tag.id}:${tag.name}`}
                    name={tag.name}
                    onRename={(name) => {
                      void commands.rename(tag.id, name);
                    }}
                  />
                  <span className={styles.count}>{count === 0 ? '' : count}</span>
                  <IconButton
                    label={`Remove ${tag.name}`}
                    onClick={() => {
                      if (count === 0) {
                        void commands.remove(tag.id);
                      } else {
                        setLeavingId(tag.id);
                      }
                    }}
                  >
                    <Trash2 />
                  </IconButton>
                </div>
                {leavingId === tag.id && (
                  <div className={styles.question} role="group" aria-label={`Remove ${tag.name}`}>
                    <span className={styles.questionText}>
                      {count === 1 ? '1 task carries' : `${count.toString()} tasks carry`} this tag.
                      It comes off {count === 1 ? 'it' : 'them'}.
                    </span>
                    <span className={styles.space} />
                    <Button
                      variant="ghost"
                      onClick={() => {
                        setLeavingId(null);
                      }}
                    >
                      Keep
                    </Button>
                    <Button
                      variant="danger"
                      onClick={() => {
                        setLeavingId(null);
                        void commands.remove(tag.id);
                      }}
                    >
                      Remove tag
                    </Button>
                  </div>
                )}
              </li>
            );
          })}
        </ul>
      )}

      <form className={styles.add} onSubmit={(event) => void add(event)}>
        <Plus aria-hidden className={styles.addMark} />
        <TextInput
          value={newName}
          placeholder="New tag"
          aria-label="Name of a new tag"
          maxLength={30}
          data-escape-undoes
          onChange={(event) => {
            setNewName(event.target.value);
          }}
          onKeyDown={(event) => {
            if (event.key === 'Escape') {
              setNewName('');
            }
          }}
        />
        <Button variant="secondary" type="submit" disabled={newName.trim() === ''}>
          Add tag
        </Button>
      </form>

      <FormActions>
        <Button variant="secondary" onClick={onDone}>
          Done
        </Button>
      </FormActions>
    </div>
  );
}

interface ColorChoiceProps {
  tag: Tag;
  onChoose: (color: Color) => void;
}

/** The color of a tag, as a square of its pigment that opens the colors to choose from. */
function ColorChoice({ tag, onChoose }: ColorChoiceProps) {
  return (
    <ChoiceMenu
      label={`The color of ${tag.name}`}
      choices={TAG_COLORS.map((color) => ({
        value: color,
        label: colorName(color),
        icon: <span className={styles.swatch} data-tone={color} />,
      }))}
      value={tag.color}
      onChoose={onChoose}
    >
      <button
        type="button"
        className={styles.color}
        aria-label={`Color of ${tag.name}: ${colorName(tag.color)}`}
        title={colorName(tag.color)}
      >
        <span className={styles.swatch} data-tone={tag.color} />
      </button>
    </ChoiceMenu>
  );
}

interface NameEditorProps {
  name: string;
  onRename: (name: string) => void;
}

/** The name of a tag, edited in place. Enter or leaving the field saves; Escape undoes. */
function NameEditor({ name, onRename }: NameEditorProps) {
  const [draft, setDraft] = useState(name);

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
      className={styles.name}
      value={draft}
      maxLength={30}
      aria-label="Name of the tag"
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
