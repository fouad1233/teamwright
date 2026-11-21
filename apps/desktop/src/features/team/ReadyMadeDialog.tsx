import { useState } from 'react';

import type { Project, ReadyMadeSpecialist } from '@/shared/api/types';
import { classNames } from '@/shared/lib/classNames';
import { Button } from '@/shared/ui/Button';
import { Dialog } from '@/shared/ui/Dialog';
import { describeFailure } from '@/shared/ui/Toast';

import { Portrait } from '../office/characters/Portrait';
import styles from './ReadyMadeDialog.module.css';
import { profileInWords } from './teamModel';
import { useReadyMadeSpecialists, useTeamCommands } from './teamQueries';

interface ReadyMadeDialogProps {
  open: boolean;
  project: Project;
  onClose: () => void;
}

/**
 * The chooser of ready-made specialists (decision 0031): the roles at the
 * left, and at the right everything about the chosen one that it joins
 * with, before it is added.
 */
export function ReadyMadeDialog({ open, project, onClose }: ReadyMadeDialogProps) {
  return (
    <Dialog open={open} title="Add a ready-made specialist" size="wide" onClose={onClose}>
      <Chooser project={project} onDone={onClose} />
    </Dialog>
  );
}

interface ChooserProps {
  project: Project;
  onDone: () => void;
}

/**
 * The inside of the dialog, a component of its own so that it starts at the
 * first role every time the dialog opens.
 */
function Chooser({ project, onDone }: ChooserProps) {
  const specialists = useReadyMadeSpecialists();
  const commands = useTeamCommands(project.id);
  const [chosenKey, setChosenKey] = useState<string | null>(null);
  const [refusal, setRefusal] = useState<string | null>(null);
  const [isAdding, setIsAdding] = useState(false);
  const roles = specialists.data ?? [];
  const chosen = roles.find((role) => role.key === chosenKey) ?? roles[0];

  async function add(role: ReadyMadeSpecialist) {
    setIsAdding(true);
    setRefusal(null);
    try {
      await commands.addReadyMade(role.key);
      onDone();
    } catch (failure) {
      setRefusal(describeFailure(failure));
      setIsAdding(false);
    }
  }

  if (specialists.isError) {
    return (
      <p className={styles.refusal} role="alert">
        {describeFailure(specialists.error)}
      </p>
    );
  }

  return (
    <div className={styles.chooser}>
      <div className={styles.panes}>
        <fieldset className={styles.roles}>
          <legend className={styles.hidden}>Role</legend>
          {roles.map((role) => (
            <label
              key={role.key}
              className={classNames(styles.role, role === chosen && styles.chosenRole)}
            >
              <input
                type="radio"
                name="ready-made"
                className={styles.radio}
                checked={role === chosen}
                onChange={() => {
                  setChosenKey(role.key);
                }}
                data-initial-focus={role === chosen ? true : undefined}
              />
              <Portrait sitter={sitterOf(role)} name={role.name} />
              <span className={styles.roleName}>{role.name}</span>
            </label>
          ))}
        </fieldset>

        {chosen && (
          <section className={styles.detail} aria-label={chosen.name}>
            <h3 className={styles.name}>{chosen.name}</h3>
            <p className={styles.description}>{chosen.description}</p>
            <p className={classNames('label', styles.profile)}>
              {profileInWords({ model: chosen.model, effort: chosen.effort })}
            </p>
            <div className={styles.instructions}>
              <span className="label">Instructions</span>
              <p className={styles.text}>{chosen.instructions}</p>
            </div>
            <p className={styles.note}>
              It joins as an AI specialist with all of this. Change anything afterwards, the
              instructions too.
            </p>
          </section>
        )}
      </div>

      {refusal !== null && (
        <p className={styles.refusal} role="alert">
          {refusal}
        </p>
      )}

      <div className={styles.actions}>
        <Button variant="ghost" onClick={onDone}>
          Cancel
        </Button>
        <Button
          variant="primary"
          disabled={chosen === undefined || isAdding}
          onClick={() => {
            if (chosen) {
              void add(chosen);
            }
          }}
        >
          Add to the team
        </Button>
      </div>
    </div>
  );
}

/**
 * Who the portrait of a role shows: the character of the role, read from
 * its name and description as the office reads a member's, so that it
 * wears and works with what the member will. The face and the color come
 * when the member joins.
 */
function sitterOf(role: ReadyMadeSpecialist) {
  return {
    kind: 'member' as const,
    member: {
      id: role.key,
      name: role.name,
      responsibility: role.description,
      color: 'gray' as const,
    },
  };
}
