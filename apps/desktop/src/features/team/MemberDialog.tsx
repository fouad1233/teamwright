import { useState, type FormEvent } from 'react';

import type { Member, MemberKind, Project } from '@/shared/api/types';
import { classNames } from '@/shared/lib/classNames';
import { Button } from '@/shared/ui/Button';
import { Dialog } from '@/shared/ui/Dialog';
import { Field, TextArea, TextInput } from '@/shared/ui/Field';
import { describeFailure } from '@/shared/ui/Toast';

import { MemberAvatar } from './Assignee';
import styles from './MemberDialog.module.css';
import { kindLabel } from './teamModel';
import { useTeamCommands } from './teamQueries';

interface KindChoice {
  kind: MemberKind;
  /** What choosing this kind means, said honestly about what works today. */
  meaning: string;
  /** A name of this kind, to show how its avatar looks. */
  example: string;
}

const KIND_CHOICES: KindChoice[] = [
  {
    kind: 'agent',
    meaning:
      'Carries out the tasks you give it and answers your questions, through the Claude Code ' +
      'on this computer. It reads and writes files in the folder of the project, nothing else.',
    example: 'Hardware engineer',
  },
  {
    kind: 'human',
    meaning: 'Someone you name as responsible for a task. Teamwright does not notify them.',
    example: 'Dana Ortiz',
  },
];

interface MemberDialogProps {
  open: boolean;
  project: Project;
  /** The member to change. Without one, the dialog puts a new member on the team. */
  member?: Member;
  onClose: () => void;
}

/** The dialog in which a member is put on the team, or changed. */
export function MemberDialog({ open, project, member, onClose }: MemberDialogProps) {
  return (
    <Dialog open={open} title={member ? 'Change member' : 'Add to the team'} onClose={onClose}>
      <MemberForm project={project} member={member} onDone={onClose} />
    </Dialog>
  );
}

interface MemberFormProps {
  project: Project;
  member?: Member;
  onDone: () => void;
}

/**
 * The form inside the dialog. It is a component of its own so that its
 * fields start from the member, or empty, every time the dialog opens.
 */
function MemberForm({ project, member, onDone }: MemberFormProps) {
  const commands = useTeamCommands(project.id);
  const [kind, setKind] = useState<MemberKind>(member?.kind ?? 'agent');
  const [name, setName] = useState(member?.name ?? '');
  const [responsibility, setResponsibility] = useState(member?.responsibility ?? '');
  const [refusal, setRefusal] = useState<string | null>(null);
  const [isSubmitting, setIsSubmitting] = useState(false);
  const chosen = KIND_CHOICES.find((choice) => choice.kind === kind);

  async function submit() {
    setIsSubmitting(true);
    setRefusal(null);
    try {
      if (member) {
        await commands.change(member, { name, responsibility });
      } else {
        await commands.add({ name, kind, responsibility });
      }
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
      {/* What a member is cannot change afterwards: a person does not turn
          into a program. */}
      {!member && (
        <fieldset className={styles.kinds}>
          <legend className={classNames('label', styles.legend)}>Kind</legend>
          <div className={styles.kindChoices}>
            {KIND_CHOICES.map((choice) => (
              <label
                key={choice.kind}
                className={classNames(styles.kind, kind === choice.kind && styles.chosenKind)}
              >
                <input
                  type="radio"
                  name="kind"
                  className={styles.radio}
                  checked={kind === choice.kind}
                  onChange={() => {
                    setKind(choice.kind);
                  }}
                />
                <MemberAvatar member={{ name: choice.example, kind: choice.kind, color: 'gray' }} />
                <span className={styles.kindName}>{kindLabel(choice.kind)}</span>
              </label>
            ))}
          </div>
          {chosen && <p className={styles.meaning}>{chosen.meaning}</p>}
        </fieldset>
      )}

      <Field label="Name">
        <TextInput
          value={name}
          onChange={(event) => {
            setName(event.target.value);
          }}
          placeholder={chosen?.example}
          maxLength={60}
          data-initial-focus
          required
        />
      </Field>

      <Field label="Responsible for">
        <TextArea
          value={responsibility}
          onChange={(event) => {
            setResponsibility(event.target.value);
          }}
          placeholder="The parts, the schematic and the board."
          rows={2}
        />
      </Field>

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
          {member ? 'Save' : 'Add to the team'}
        </Button>
      </div>
    </form>
  );
}
