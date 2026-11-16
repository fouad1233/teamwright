import { useState } from 'react';

import { classNames } from '@/shared/lib/classNames';

import type { KindProps } from '../views/viewKinds';
import { FormEditor } from './FormEditor';
import { FormFill } from './FormFill';
import styles from './FormView.module.css';
import { useTaskForm } from './useTaskForm';

/** The two faces of the view. */
type Face = 'fill' | 'edit';

const FACES: readonly { face: Face; label: string }[] = [
  { face: 'fill', label: 'Fill in' },
  { face: 'edit', label: 'Edit the form' },
];

/**
 * The form view: a form, set like a printed sheet in the middle of the
 * view, that makes a task of this project when it is sent. "Fill in" is
 * the form as it is filled in; "Edit the form" is the same sheet with
 * handles, to choose, order and name its fields.
 */
export function FormView({ view, project }: KindProps) {
  const [form, change] = useTaskForm(view);
  const [face, setFace] = useState<Face>('fill');

  return (
    <div className={styles.view}>
      <div className={styles.bar}>
        <span className={styles.barNote}>
          {face === 'edit' ? 'Changes are kept as you make them.' : null}
        </span>
        <div className={styles.switch} role="group" aria-label="Face of the form">
          {FACES.map((choice) => (
            <button
              key={choice.face}
              type="button"
              className={classNames(styles.choice, face === choice.face && styles.chosen)}
              aria-pressed={face === choice.face}
              onClick={() => {
                setFace(choice.face);
              }}
            >
              {choice.label}
            </button>
          ))}
        </div>
      </div>
      <div className={styles.ground}>
        {face === 'fill' ? (
          <FormFill project={project} form={form} />
        ) : (
          <FormEditor project={project} form={form} onChange={change} />
        )}
      </div>
    </div>
  );
}
