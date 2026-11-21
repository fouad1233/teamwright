import { useState, type FormEvent } from 'react';

import { useNavigation } from '@/app/navigation';
import type { WorkflowTemplate } from '@/shared/api/types';
import { Button } from '@/shared/ui/Button';
import { Dialog } from '@/shared/ui/Dialog';
import { Field, TextArea, TextInput } from '@/shared/ui/Field';
import { describeFailure } from '@/shared/ui/Toast';

import styles from './CreateProjectDialog.module.css';
import { useCreateProject, useProjectTemplates } from './projectQueries';
import { TemplateChoice } from './TemplateChoice';

interface CreateProjectDialogProps {
  open: boolean;
  onClose: () => void;
}

/** The dialog in which a project is started. */
export function CreateProjectDialog({ open, onClose }: CreateProjectDialogProps) {
  return (
    <Dialog open={open} title="New project" size="wide" onClose={onClose}>
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
  const templates = useProjectTemplates();
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

      <TemplateChoice templates={templates.data ?? []} chosen={workflow} onChoose={setWorkflow} />

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
