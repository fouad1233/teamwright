import { Plus } from 'lucide-react';
import { useState, type FormEvent, type KeyboardEvent } from 'react';

import { TextInput } from '@/shared/ui/Field';

import styles from './TaskComposer.module.css';

interface TaskComposerProps {
  /** Opens a task with the given title. Resolves to whether it was opened. */
  onCreate: (title: string) => Promise<boolean>;
}

/**
 * The "Add task" line at the bottom of a status. After a task is added the
 * line stays open for the next one, so a list of tasks can be typed in one
 * go. Escape, or leaving the empty line, closes it.
 */
export function TaskComposer({ onCreate }: TaskComposerProps) {
  const [isOpen, setIsOpen] = useState(false);
  const [title, setTitle] = useState('');

  function close() {
    setIsOpen(false);
    setTitle('');
  }

  async function createTask() {
    if (title.trim() === '') {
      close();
      return;
    }
    const wasCreated = await onCreate(title);
    if (wasCreated) {
      setTitle('');
    }
  }

  function handleSubmit(event: FormEvent) {
    event.preventDefault();
    void createTask();
  }

  function handleKeyDown(event: KeyboardEvent) {
    if (event.key === 'Escape') {
      close();
    }
  }

  function closeIfEmpty() {
    if (title.trim() === '') {
      close();
    }
  }

  if (!isOpen) {
    return (
      <button
        type="button"
        className={styles.opener}
        onClick={() => {
          setIsOpen(true);
        }}
      >
        <Plus />
        Add task
      </button>
    );
  }

  return (
    <form className={styles.form} onSubmit={handleSubmit}>
      <TextInput
        value={title}
        onChange={(event) => {
          setTitle(event.target.value);
        }}
        onKeyDown={handleKeyDown}
        onBlur={closeIfEmpty}
        placeholder="What needs to be done?"
        aria-label="Title of the new task"
        autoFocus
      />
    </form>
  );
}
