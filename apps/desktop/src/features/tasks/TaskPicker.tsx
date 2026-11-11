import { Plus } from 'lucide-react';
import { Popover } from 'radix-ui';
import { useState, type KeyboardEvent } from 'react';

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

interface TaskPickerProps {
  /** What picking a task does, as the tooltip of the button: "Add a task to wait for". */
  label: string;
  project: Project;
  /** The tasks that can be picked. */
  candidates: Task[];
  onPick: (taskId: Id) => void;
}

/**
 * A button that opens a list of tasks to pick one from. Typing narrows the
 * list by reference or title; the arrow keys and Enter pick with the
 * keyboard.
 */
export function TaskPicker({ label, project, candidates, onPick }: TaskPickerProps) {
  const [isOpen, setIsOpen] = useState(false);
  const [query, setQuery] = useState('');
  const [highlighted, setHighlighted] = useState(0);

  const wanted = query.trim().toLowerCase();
  const matches = candidates
    .filter(
      (task) =>
        task.title.toLowerCase().includes(wanted) ||
        taskReference(project, task).toLowerCase().includes(wanted),
    )
    .slice(0, MOST_MATCHES);

  function changeOpen(open: boolean) {
    setIsOpen(open);
    setQuery('');
    setHighlighted(0);
  }

  function pick(taskId: Id) {
    onPick(taskId);
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
        pick(chosen.id);
      }
    }
  }

  return (
    <Popover.Root open={isOpen} onOpenChange={changeOpen}>
      <Popover.Trigger asChild>
        <IconButton label={label}>
          <Plus />
        </IconButton>
      </Popover.Trigger>
      <Popover.Portal>
        <Popover.Content className={styles.picker} align="end" sideOffset={4}>
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
              {matches.map((task, position) => {
                const status = statusOf(project, task);
                return (
                  <li key={task.id} role="option" aria-selected={position === highlighted}>
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
                        pick(task.id);
                      }}
                    >
                      {status && <StatusMark status={status} />}
                      <span className={styles.reference}>{taskReference(project, task)}</span>
                      <span className={styles.title}>{task.title}</span>
                    </button>
                  </li>
                );
              })}
            </ul>
          )}
        </Popover.Content>
      </Popover.Portal>
    </Popover.Root>
  );
}
