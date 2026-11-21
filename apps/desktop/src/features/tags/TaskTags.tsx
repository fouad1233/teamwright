import { Check, Plus, Settings2, X } from 'lucide-react';
import { Popover } from 'radix-ui';
import { useState, type KeyboardEvent } from 'react';

import type { Project, Tag, Task } from '@/shared/api/types';
import { classNames } from '@/shared/lib/classNames';
import { IconButton } from '@/shared/ui/Button';
import { EDGE_GAP } from '@/shared/ui/edgeGap';
import { TextInput } from '@/shared/ui/Field';

import { TagChip } from './TagChip';
import { makesNewTag, matchingTags, tagsOf } from './tagModel';
import { useTagCommands, useTags } from './tagQueries';
import { TagsDialog } from './TagsDialog';
import styles from './TaskTags.module.css';

/** The tags while they are loading. */
const NO_TAGS: readonly Tag[] = [];

interface TaskTagsProps {
  project: Project;
  task: Task;
}

/**
 * The tags of a task in its panel: a chip for each, with a way to take it
 * off, and a button that opens the list of the tags of the project to put
 * on, where a new one is made by typing its name.
 */
export function TaskTags({ project, task }: TaskTagsProps) {
  const tags = useTags(project.id).data ?? NO_TAGS;
  const commands = useTagCommands(project.id);
  const [isEditing, setIsEditing] = useState(false);
  const carried = tagsOf(task, tags);

  return (
    <div className={styles.tags}>
      {carried.map((tag) => (
        <span key={tag.id} className={styles.carried} data-tone={tag.color}>
          <TagChip tag={tag} size="regular" />
          <button
            type="button"
            className={styles.takeOff}
            aria-label={`Take ${tag.name} off`}
            title={`Take ${tag.name} off`}
            onClick={() => {
              void commands.setOnTask(task.id, tag.id, false);
            }}
          >
            <X />
          </button>
        </span>
      ))}
      <TagPicker
        task={task}
        tags={tags}
        isEmpty={carried.length === 0}
        onEditTags={() => {
          setIsEditing(true);
        }}
      />
      <TagsDialog
        open={isEditing}
        project={project}
        onClose={() => {
          setIsEditing(false);
        }}
      />
    </div>
  );
}

interface TagPickerProps {
  task: Task;
  tags: readonly Tag[];
  /** Whether the task carries no tag yet: the button then says what it does. */
  isEmpty: boolean;
  onEditTags: () => void;
}

/** One line of the picker: a tag to put on or take off, or a new tag to make. */
type PickerLine = { kind: 'tag'; tag: Tag } | { kind: 'new'; name: string };

/**
 * The list of the tags of the project, to put on a task or take off. It
 * stays open, so that several can be chosen at once. Typing narrows the
 * list; a name no tag has offers to make that tag. The arrow keys and Enter
 * choose with the keyboard.
 */
function TagPicker({ task, tags, isEmpty, onEditTags }: TagPickerProps) {
  const commands = useTagCommands(task.projectId);
  const [isOpen, setIsOpen] = useState(false);
  const [typed, setTyped] = useState('');
  const [highlighted, setHighlighted] = useState(0);

  const lines: PickerLine[] = [
    ...matchingTags(tags, typed).map((tag): PickerLine => ({ kind: 'tag', tag })),
    ...(makesNewTag(tags, typed) ? [{ kind: 'new', name: typed.trim() } as const] : []),
  ];

  function changeOpen(open: boolean) {
    setIsOpen(open);
    setTyped('');
    setHighlighted(0);
  }

  function choose(line: PickerLine) {
    if (line.kind === 'tag') {
      void commands.setOnTask(task.id, line.tag.id, !task.tagIds.includes(line.tag.id));
    } else {
      void commands.putNamedOnTask(task.id, line.name);
      setTyped('');
      setHighlighted(0);
    }
  }

  function handleKeyDown(event: KeyboardEvent) {
    if (event.key === 'ArrowDown') {
      event.preventDefault();
      setHighlighted((current) => Math.min(current + 1, lines.length - 1));
    } else if (event.key === 'ArrowUp') {
      event.preventDefault();
      setHighlighted((current) => Math.max(current - 1, 0));
    } else if (event.key === 'Enter') {
      event.preventDefault();
      const line = lines[highlighted];
      if (line) {
        choose(line);
      }
    }
  }

  return (
    <Popover.Root open={isOpen} onOpenChange={changeOpen}>
      <Popover.Trigger asChild>
        {isEmpty ? (
          <button type="button" className={styles.add}>
            <Plus />
            Add a tag
          </button>
        ) : (
          <IconButton label="Add a tag">
            <Plus />
          </IconButton>
        )}
      </Popover.Trigger>
      <Popover.Portal>
        <Popover.Content
          className={styles.picker}
          align="start"
          sideOffset={4}
          collisionPadding={EDGE_GAP}
        >
          <TextInput
            value={typed}
            placeholder="Find or make a tag…"
            aria-label="Find or make a tag"
            maxLength={30}
            autoFocus
            onChange={(event) => {
              setTyped(event.target.value);
              setHighlighted(0);
            }}
            onKeyDown={handleKeyDown}
          />
          {lines.length === 0 ? (
            <p className={styles.none}>Type a name to make the first tag.</p>
          ) : (
            <ul className={styles.lines} role="listbox" aria-label="Tags" aria-multiselectable>
              {lines.map((line, position) => {
                const isOn = line.kind === 'tag' && task.tagIds.includes(line.tag.id);
                return (
                  <li
                    key={line.kind === 'tag' ? line.tag.id : 'new'}
                    role="option"
                    aria-selected={isOn}
                  >
                    <button
                      type="button"
                      className={classNames(
                        styles.line,
                        position === highlighted && styles.highlighted,
                      )}
                      tabIndex={-1}
                      onPointerMove={() => {
                        setHighlighted(position);
                      }}
                      onClick={() => {
                        choose(line);
                      }}
                    >
                      <span className={styles.check}>{isOn && <Check aria-hidden />}</span>
                      {line.kind === 'tag' ? (
                        <TagChip tag={line.tag} size="regular" />
                      ) : (
                        <span className={styles.make}>
                          Make the tag <strong>{line.name}</strong>
                        </span>
                      )}
                    </button>
                  </li>
                );
              })}
            </ul>
          )}
          <button
            type="button"
            className={styles.edit}
            onClick={() => {
              changeOpen(false);
              onEditTags();
            }}
          >
            <Settings2 />
            Edit the tags of the project
          </button>
        </Popover.Content>
      </Popover.Portal>
    </Popover.Root>
  );
}
