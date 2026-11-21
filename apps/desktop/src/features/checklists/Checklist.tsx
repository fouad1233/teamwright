import { ArrowDown, ArrowUp, Check, Ellipsis, Plus, Trash2 } from 'lucide-react';
import { useState, type FormEvent, type KeyboardEvent } from 'react';

import type { ChecklistItem, Task } from '@/shared/api/types';
import { classNames } from '@/shared/lib/classNames';
import { IconButton } from '@/shared/ui/Button';
import { ActionMenu } from '@/shared/ui/Menu';

import styles from './Checklist.module.css';
import { progressOf } from './checklistModel';
import { useChecklistCommands } from './checklistQueries';

interface ChecklistProps {
  task: Task;
}

/**
 * The checklist of a task in its panel: its items in their order, each
 * ticked with its box and renamed in place, with a menu to move it or remove
 * it, and a line at the end to add the next one. After an item is added the
 * line stays ready for another, so a list can be typed in one go.
 */
export function Checklist({ task }: ChecklistProps) {
  const commands = useChecklistCommands(task.projectId);
  const progress = progressOf(task);
  const last = task.checklist.length - 1;

  return (
    <div className={styles.checklist}>
      {progress !== null && (
        <div
          className={styles.meter}
          role="progressbar"
          aria-label="Items done"
          aria-valuemin={0}
          aria-valuemax={progress.total}
          aria-valuenow={progress.done}
        >
          <span
            className={styles.filled}
            style={{ width: `${((progress.done / progress.total) * 100).toString()}%` }}
          />
        </div>
      )}
      {task.checklist.length > 0 && (
        <ul className={styles.items}>
          {task.checklist.map((item, index) => (
            <ItemRow
              key={item.id}
              item={item}
              isFirst={index === 0}
              isLast={index === last}
              onTick={(done) => {
                void commands.tick(task.id, item.id, done);
              }}
              onRename={(text) => {
                void commands.rename(task.id, item.id, text);
              }}
              onMove={(place) => {
                void commands.move(task.id, item.id, place);
              }}
              onRemove={() => {
                void commands.remove(task.id, item.id);
              }}
              place={index}
            />
          ))}
        </ul>
      )}
      <ItemComposer onAdd={(text) => commands.add(task.id, text)} />
    </div>
  );
}

interface ItemRowProps {
  item: ChecklistItem;
  /** The place of the item in the checklist, from 0. */
  place: number;
  isFirst: boolean;
  isLast: boolean;
  onTick: (done: boolean) => void;
  onRename: (text: string) => void;
  onMove: (place: number) => void;
  onRemove: () => void;
}

/** One item: its box, its text, and a menu to move or remove it. */
function ItemRow({
  item,
  place,
  isFirst,
  isLast,
  onTick,
  onRename,
  onMove,
  onRemove,
}: ItemRowProps) {
  return (
    <li className={classNames(styles.item, item.done && styles.done)}>
      <button
        type="button"
        role="checkbox"
        aria-checked={item.done}
        aria-label={item.text}
        className={styles.box}
        onClick={() => {
          onTick(!item.done);
        }}
      >
        {item.done && <Check aria-hidden />}
      </button>
      <TextEditor key={`${item.id}:${item.text}`} text={item.text} onRename={onRename} />
      <ActionMenu
        label={`The item ${item.text}`}
        actions={[
          {
            label: 'Move up',
            icon: <ArrowUp />,
            isDisabled: isFirst,
            onSelect: () => {
              onMove(place - 1);
            },
          },
          {
            label: 'Move down',
            icon: <ArrowDown />,
            isDisabled: isLast,
            onSelect: () => {
              onMove(place + 1);
            },
          },
          { label: 'Remove item', icon: <Trash2 />, isDestructive: true, onSelect: onRemove },
        ]}
      >
        <IconButton label="Item actions" className={styles.actions}>
          <Ellipsis />
        </IconButton>
      </ActionMenu>
    </li>
  );
}

interface TextEditorProps {
  text: string;
  onRename: (text: string) => void;
}

/** The text of an item, edited in place. Enter or leaving the field saves; Escape undoes. */
function TextEditor({ text, onRename }: TextEditorProps) {
  const [draft, setDraft] = useState(text);

  function save() {
    const tidyDraft = draft.trim();
    if (tidyDraft === '') {
      setDraft(text);
    } else if (tidyDraft !== text) {
      onRename(tidyDraft);
    }
  }

  function handleKeyDown(event: KeyboardEvent<HTMLInputElement>) {
    if (event.key === 'Enter') {
      event.currentTarget.blur();
    } else if (event.key === 'Escape') {
      // Escape undoes the edit. It must not also close the panel.
      event.stopPropagation();
      setDraft(text);
    }
  }

  return (
    <input
      className={styles.text}
      value={draft}
      maxLength={200}
      aria-label="Text of the item"
      title={text}
      onChange={(event) => {
        setDraft(event.target.value);
      }}
      onBlur={save}
      onKeyDown={handleKeyDown}
    />
  );
}

interface ItemComposerProps {
  /** Adds an item with the given text. Resolves to whether it was added. */
  onAdd: (text: string) => Promise<boolean>;
}

/** The line at the end of the checklist where the next item is typed. */
function ItemComposer({ onAdd }: ItemComposerProps) {
  const [text, setText] = useState('');

  async function add(event: FormEvent) {
    event.preventDefault();
    if (text.trim() === '') {
      return;
    }
    if (await onAdd(text)) {
      setText('');
    }
  }

  return (
    <form className={styles.composer} onSubmit={(event) => void add(event)}>
      <Plus aria-hidden className={styles.plus} />
      <input
        className={styles.text}
        value={text}
        maxLength={200}
        placeholder="Add an item"
        aria-label="Text of a new item"
        onChange={(event) => {
          setText(event.target.value);
        }}
        onKeyDown={(event) => {
          if (event.key === 'Escape' && text !== '') {
            event.stopPropagation();
            setText('');
          }
        }}
      />
    </form>
  );
}
