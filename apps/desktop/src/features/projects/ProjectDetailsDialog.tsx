import { useState, type FormEvent } from 'react';

import type { Project } from '@/shared/api/types';
import { Button } from '@/shared/ui/Button';
import { Dialog } from '@/shared/ui/Dialog';
import { Field, TextArea, TextInput } from '@/shared/ui/Field';
import { Form, FormActions, FormNote, Refusal } from '@/shared/ui/Form';
import { describeFailure } from '@/shared/ui/Toast';

import { useProjectCommands } from './projectQueries';

interface ProjectDetailsDialogProps {
  open: boolean;
  project: Project;
  onClose: () => void;
}

/** The dialog in which a project gets another name or another description. */
export function ProjectDetailsDialog({ open, project, onClose }: ProjectDetailsDialogProps) {
  return (
    <Dialog open={open} title="Name and description" onClose={onClose}>
      <ProjectDetailsForm project={project} onDone={onClose} />
    </Dialog>
  );
}

/**
 * The form inside the dialog. It is a component of its own so that its
 * fields start from the stored values every time the dialog opens.
 */
function ProjectDetailsForm({ project, onDone }: { project: Project; onDone: () => void }) {
  const commands = useProjectCommands();
  const [name, setName] = useState(project.name);
  const [description, setDescription] = useState(project.description);
  const [refusal, setRefusal] = useState<string | null>(null);
  const [isSubmitting, setIsSubmitting] = useState(false);

  async function submit() {
    setIsSubmitting(true);
    setRefusal(null);
    try {
      await commands.change(project, { name, description });
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
    <Form onSubmit={handleSubmit}>
      <Field label="Name">
        <TextInput
          value={name}
          onChange={(event) => {
            setName(event.target.value);
          }}
          data-initial-focus
          required
        />
      </Field>

      <Field label="Description">
        <TextArea
          value={description}
          onChange={(event) => {
            setDescription(event.target.value);
          }}
          placeholder="What is this project for?"
          rows={3}
        />
      </Field>

      <FormNote>
        The key {project.key} stays as it is. It is part of the number of every task.
      </FormNote>

      {refusal !== null && <Refusal>{refusal}</Refusal>}

      <FormActions>
        <Button variant="ghost" onClick={onDone}>
          Cancel
        </Button>
        <Button variant="primary" type="submit" disabled={isSubmitting || name.trim() === ''}>
          Save
        </Button>
      </FormActions>
    </Form>
  );
}
