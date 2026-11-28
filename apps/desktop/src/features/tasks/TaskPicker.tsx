import { Plus } from 'lucide-react';
import { Popover } from 'radix-ui';
import { useState, type KeyboardEvent, type ReactElement, type ReactNode } from 'react';

import type { Id, Project, Task } from '@/shared/api/types';
import { classNames } from '@/shared/lib/classNames';
import { taskReference } from '@/shared/lib/format';
import { IconButton } from '@/shared/ui/Button';
import { TextInput } from '@/shared/ui/Field';

import { StatusMark } from './TaskBadges';
import styles from './TaskPicker.module.css';
import { statusOf } from './taskModel';

/** The most tasks the list shows at once. Typing narrows it down. */
const MOST_MATCHES = 40;

/** One task in the list of a picker. */
export interface PickerEntry {
  /** What the picker knows the task by, and gives back when it is picked. */
  key: string;
  /** The short reference that stands before the title: "AQN-12", or "3" in a plan. */
  reference: string;
  title: string;
  /** A mark before the reference, such as the mark of the status. */
  mark?: ReactNode;
}

interface PickerProps {
  /** What picking a task does, as the tooltip of the button: "Add a task to wait for". */
  label: string;
  /** The tasks that can be picked. */
  entries: readonly PickerEntry[];
  onPick: (key: string) => void;
  /**
   * What opens the list, such as the value a task has now. Left out, it is
   * a button with a plus.
   */
  trigger?: ReactElement;
}

/**
 * A button that opens a list of tasks to pick one from. Typing narrows the
 * list by reference or title; the arrow keys and Enter pick with the
 * keyboard. It works on any list of tasks: those of a project, or those of
 * a plan that is not approved yet.
 */
export function Picker({ label, entries, onPick, trigger }: PickerProps) {
  const [isOpen, setIsOpen] = useState(false);
  const [query, setQuery] = useState('');
  const [highlighted, setHighlighted] = useState(0);

  const wanted = query.trim().toLowerCase();
  const matches = entries
    .filter(
      (entry) =>
        entry.title.toLowerCase().includes(wanted) ||
        entry.reference.toLowerCase().includes(wanted),
    )
    .slice(0, MOST_MATCHES);

  function changeOpen(open: boolean) {
    setIsOpen(open);
    setQuery('');
    setHighlighted(0);
  }

  function pick(key: string) {
    onPick(key);
    changeOpen(false);
  }

  function handleKeyDown(event: KeyboardEvent) {
    if (event.key === 'ArrowDown') {
      event.preventDefault();
      setHighlighted((current) => Math.min(current + 1, matches.length - 1));
    } else if (event.key === 'ArrowUp') {
      event.preventDefault();
      setHighlighted((current) => Math.max(current - 1, 0));
    } else if (event.key === 'Enter') {
      event.preventDefault();
      const chosen = matches[highlighted];
      if (chosen) {
        pick(chosen.key);
      }
    }
  }

  return (
    <Popover.Root open={isOpen} onOpenChange={changeOpen}>
      <Popover.Trigger asChild>
        {trigger ?? (
          <IconButton label={label}>
            <Plus />
          </IconButton>
        )}
      </Popover.Trigger>
      <Popover.Portal>
        <Popover.Content
          className={styles.picker}
          // A list opened from a value lines up with it; one opened from
          // the button with a plus at the end of a heading, with that end.
          align={trigger ? 'start' : 'end'}
          sideOffset={4}
        >
          <TextInput
            value={query}
            placeholder="Find a task…"
            aria-label="Find a task"
            autoFocus
            onChange={(event) => {
              setQuery(event.target.value);
              setHighlighted(0);
            }}
            onKeyDown={handleKeyDown}
          />
          {matches.length === 0 ? (
            <p className={styles.none}>No task matches.</p>
          ) : (
            <ul className={styles.matches} role="listbox" aria-label={label}>
              {matches.map((entry, position) => (
                <li key={entry.key} role="option" aria-selected={position === highlighted}>
                  <button
                    type="button"
                    className={classNames(
                      styles.match,
                      position === highlighted && styles.highlighted,
                    )}
                    tabIndex={-1}
                    onPointerMove={() => {
                      setHighlighted(position);
                    }}
                    onClick={() => {
                      pick(entry.key);
                    }}
                  >
                    {entry.mark}
                    <span className={styles.reference}>{entry.reference}</span>
                    <span className={styles.title}>{entry.title}</span>
                  </button>
                </li>
              ))}
            </ul>
          )}
        </Popover.Content>
      </Popover.Portal>
    </Popover.Root>
  );
}

interface TaskPickerProps {
  /** What picking a task does, as the tooltip of the button: "Add a task to wait for". */
  label: string;
  project: Project;
  /** The tasks that can be picked. */
  candidates: Task[];
  onPick: (taskId: Id) => void;
  /** What opens the list, if not a button with a plus. */
  trigger?: ReactElement;
}

/** A picker of the tasks of a project, each with the mark of its status. */
export function TaskPicker({ label, project, candidates, onPick, trigger }: TaskPickerProps) {
  const entries = candidates.map((task): PickerEntry => {
    const status = statusOf(project, task);
    return {
      key: task.id,
      reference: taskReference(project, task),
      title: task.title,
      mark: status && <StatusMark status={status} />,
    };
  });
  return <Picker label={label} entries={entries} onPick={onPick} trigger={trigger} />;
}
