import { useState, type FormEvent } from 'react';

import { useNavigation } from '@/app/navigation';
import type { Status, WorkflowTemplate } from '@/shared/api/types';
import { classNames } from '@/shared/lib/classNames';
import { Button } from '@/shared/ui/Button';
import { Dialog } from '@/shared/ui/Dialog';
import { Field, TextArea, TextInput } from '@/shared/ui/Field';
import { describeFailure } from '@/shared/ui/Toast';

import { StatusMark } from '../tasks/TaskBadges';
import styles from './CreateProjectDialog.module.css';
import { useCreateProject } from './projectQueries';

interface WorkflowChoice {
  template: WorkflowTemplate;
  name: string;
  /** The statuses the workflow starts with, in order, as the backend creates them. */
  statuses: Pick<Status, 'name' | 'category' | 'color'>[];
}

const WORKFLOW_CHOICES: WorkflowChoice[] = [
  {
    template: 'standard',
    name: 'Standard',
    statuses: [
      { name: 'To do', category: 'todo', color: 'gray' },
      { name: 'In progress', category: 'active', color: 'blue' },
      { name: 'In review', category: 'review', color: 'amber' },
      { name: 'Done', category: 'done', color: 'green' },
    ],
  },
  {
    template: 'engineering',
    name: 'Engineering',
    statuses: [
      { name: 'To do', category: 'todo', color: 'gray' },
      { name: 'In progress', category: 'active', color: 'blue' },
      { name: 'In revision', category: 'active', color: 'purple' },
      { name: 'In test', category: 'review', color: 'amber' },
      { name: 'Done', category: 'done', color: 'green' },
    ],
  },
];

interface CreateProjectDialogProps {
  open: boolean;
  onClose: () => void;
}

/** The dialog in which a project is started. */
export function CreateProjectDialog({ open, onClose }: CreateProjectDialogProps) {
  return (
    <Dialog open={open} title="New project" onClose={onClose}>
      <CreateProjectForm onDone={onClose} />
    </Dialog>
  );
}

/**
 * The form inside the dialog. It is a component of its own so that its
 * fields start empty every time the dialog opens.
 */
function CreateProjectForm({ onDone }: { onDone: () => void }) {
  const createProject = useCreateProject();
  const navigation = useNavigation();
  const [name, setName] = useState('');
  const [key, setKey] = useState('');
  const [description, setDescription] = useState('');
  const [workflow, setWorkflow] = useState<WorkflowTemplate>('standard');
  const [refusal, setRefusal] = useState<string | null>(null);
  const [isSubmitting, setIsSubmitting] = useState(false);

  async function submit() {
    setIsSubmitting(true);
    setRefusal(null);
    try {
      const project = await createProject({
        name,
        key: key.trim() || undefined,
        description,
        workflow,
      });
      // A new project is empty. It opens where it can be planned: with the
      // Advisor, which starts from what was just said about the project.
      // The board is one press away for whoever wants to add tasks by hand.
      navigation.openAdvisorOf(project.id);
      onDone();
    } catch (failure) {
      setRefusal(describeFailure(failure));
      setIsSubmitting(false);
    }
  }

  function handleSubmit(event: FormEvent) {
    event.preventDefault();
    void submit();
  }

  return (
    <form className={styles.form} onSubmit={handleSubmit}>
      <div className={styles.nameAndKey}>
        <Field label="Name">
          <TextInput
            value={name}
            onChange={(event) => {
              setName(event.target.value);
            }}
            placeholder="Smart sensor node"
            data-initial-focus
            required
          />
        </Field>
        <Field label="Key">
          <TextInput
            value={key}
            onChange={(event) => {
              setKey(event.target.value.toUpperCase());
            }}
            placeholder="Auto"
            maxLength={6}
            className={styles.keyInput}
          />
        </Field>
      </div>
      <p className={styles.hint}>
        The key prefixes the task numbers, as in IOT-12. Left empty, it is derived from the name.
      </p>

      <Field label="Description">
        <TextArea
          value={description}
          onChange={(event) => {
            setDescription(event.target.value);
          }}
          placeholder="What is this project for?"
          rows={2}
        />
      </Field>

      <fieldset className={styles.workflows}>
        <legend className={classNames('label', styles.legend)}>Workflow</legend>
        {WORKFLOW_CHOICES.map((choice) => (
          <label
            key={choice.template}
            className={classNames(
              styles.workflow,
              workflow === choice.template && styles.chosenWorkflow,
            )}
          >
            <input
              type="radio"
              name="workflow"
              className={styles.radio}
              checked={workflow === choice.template}
              onChange={() => {
                setWorkflow(choice.template);
              }}
            />
            <span className={styles.workflowName}>{choice.name}</span>
            <span className={styles.workflowStatuses}>
              {choice.statuses.map((status) => (
                <span key={status.name} className={styles.workflowStatus}>
                  <StatusMark status={status} />
                  {status.name}
                </span>
              ))}
            </span>
          </label>
        ))}
      </fieldset>

      {refusal !== null && (
        <p className={styles.refusal} role="alert">
          {refusal}
        </p>
      )}

      <div className={styles.actions}>
        <Button variant="ghost" onClick={onDone}>
          Cancel
        </Button>
        <Button variant="primary" type="submit" disabled={isSubmitting || name.trim() === ''}>
          Create project
        </Button>
      </div>
    </form>
  );
}
