import { useState, type FormEvent } from 'react';

import { useNavigation } from '@/app/navigation';
import type { Assignee, Project } from '@/shared/api/types';
import { Button } from '@/shared/ui/Button';
import { Dialog } from '@/shared/ui/Dialog';
import { Field, TextInput } from '@/shared/ui/Field';

import { useTaskCommands } from '../tasks/taskQueries';
import styles from './NewTaskDialog.module.css';

interface NewTaskDialogProps {
  open: boolean;
  project: Project;
  /**
   * Who the new task is given to, when the view that is open shows the
   * tasks of one person. Left out, nobody.
   */
  assignee?: Assignee;
  onClose: () => void;
}

/**
 * The dialog in which a task is opened by its title alone. The new task is
 * shown at once, so the manager can go on to assign it and fill in the rest.
 */
export function NewTaskDialog({ open, project, assignee, onClose }: NewTaskDialogProps) {
  return (
    <Dialog open={open} title="New task" onClose={onClose}>
      <NewTaskForm project={project} assignee={assignee} onDone={onClose} />
    </Dialog>
  );
}

/** The form inside the dialog. Its field starts empty every time the dialog opens. */
interface NewTaskFormProps {
  project: Project;
  assignee: Assignee | undefined;
  onDone: () => void;
}

function NewTaskForm({ project, assignee, onDone }: NewTaskFormProps) {
  const commands = useTaskCommands(project.id);
  const navigation = useNavigation();
  const [title, setTitle] = useState('');
  const [isSubmitting, setIsSubmitting] = useState(false);
  const firstStatus = project.statuses.find((status) => status.category === 'todo');

  async function createTask() {
    setIsSubmitting(true);
    const created = await commands.create({ title, assignee });
    setIsSubmitting(false);
    if (created) {
      onDone();
      navigation.openTask(created.id);
    }
  }

  function handleSubmit(event: FormEvent) {
    event.preventDefault();
    void createTask();
  }

  return (
    <form className={styles.form} onSubmit={handleSubmit}>
      <Field
        label="Title"
        hint={
          firstStatus
            ? `The task starts in "${firstStatus.name}". Assign it and add details once it is open.`
            : undefined
        }
      >
        <TextInput
          value={title}
          onChange={(event) => {
            setTitle(event.target.value);
          }}
          placeholder="What needs to be done?"
          data-initial-focus
          required
        />
      </Field>

      <div className={styles.actions}>
        <Button variant="ghost" onClick={onDone}>
          Cancel
        </Button>
        <Button variant="primary" type="submit" disabled={isSubmitting || title.trim() === ''}>
          Create task
        </Button>
      </div>
    </form>
  );
}
