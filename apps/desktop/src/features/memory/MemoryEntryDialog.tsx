import { useState, type FormEvent } from 'react';

import type { MemoryEntry } from '@/shared/api/types';
import { Button } from '@/shared/ui/Button';
import { Dialog } from '@/shared/ui/Dialog';
import { Field, TextArea, TextInput } from '@/shared/ui/Field';
import { Form, FormActions, FormNote, Refusal } from '@/shared/ui/Form';
import { describeFailure } from '@/shared/ui/Toast';

import { KIND_LABELS } from './memoryModel';

interface MemoryEntryDialogProps {
  open: boolean;
  /** The entry to change, or nothing to add a fact. */
  entry: MemoryEntry | null;
  /** Stores the words. Throws what the backend refused. */
  onSave: (title: string, text: string) => Promise<unknown>;
  onClose: () => void;
}

/** The dialog in which the manager adds a fact or changes an entry. */
export function MemoryEntryDialog({ open, entry, onSave, onClose }: MemoryEntryDialogProps) {
  const title =
    entry === null ? 'Add a fact' : `Change the ${KIND_LABELS[entry.kind].toLowerCase()}`;
  return (
    <Dialog open={open} title={title} onClose={onClose}>
      {/* A form of its own each time, so that it starts from the entry. */}
      <EntryForm key={entry?.id ?? 'new'} entry={entry} onSave={onSave} onDone={onClose} />
    </Dialog>
  );
}

interface EntryFormProps {
  entry: MemoryEntry | null;
  onSave: (title: string, text: string) => Promise<unknown>;
  onDone: () => void;
}

function EntryForm({ entry, onSave, onDone }: EntryFormProps) {
  const [title, setTitle] = useState(entry?.title ?? '');
  const [text, setText] = useState(entry?.text ?? '');
  const [refusal, setRefusal] = useState<string | null>(null);
  const [isSaving, setIsSaving] = useState(false);

  async function save() {
    setIsSaving(true);
    setRefusal(null);
    try {
      await onSave(title, text);
      onDone();
    } catch (failure) {
      setRefusal(describeFailure(failure));
      setIsSaving(false);
    }
  }

  function handleSubmit(event: FormEvent) {
    event.preventDefault();
    void save();
  }

  return (
    <Form onSubmit={handleSubmit}>
      <Field label="Title">
        <TextInput
          value={title}
          onChange={(event) => {
            setTitle(event.target.value);
          }}
          placeholder="Height of the enclosure"
          maxLength={120}
          data-initial-focus
          required
        />
      </Field>
      <Field label={entry === null ? 'What is true' : 'Text'}>
        <TextArea
          value={text}
          onChange={(event) => {
            setText(event.target.value);
          }}
          placeholder="The enclosure must stay under 12 mm."
          rows={entry?.kind === 'contract' ? 8 : 4}
          required
        />
      </Field>
      <FormNote>
        {entry === null
          ? 'Every specialist of the project finds it when it searches the memory, and the brief of a task carries it when it belongs there.'
          : 'Your words become a newer entry. The one you change stays, marked as replaced, so that what the team relied on can still be read.'}
      </FormNote>
      {refusal !== null && <Refusal>{refusal}</Refusal>}
      <FormActions>
        <Button variant="ghost" onClick={onDone}>
          Cancel
        </Button>
        <Button variant="primary" type="submit" disabled={isSaving}>
          {entry === null ? 'Add fact' : 'Save as newer entry'}
        </Button>
      </FormActions>
    </Form>
  );
}
