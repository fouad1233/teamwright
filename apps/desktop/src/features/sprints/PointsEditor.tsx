import { useState } from 'react';

import { classNames } from '@/shared/lib/classNames';

import styles from './PointsEditor.module.css';
import { parsePoints } from './sprintModel';

interface PointsEditorProps {
  points: number | null;
  onChange: (points: number | null) => Promise<void>;
  /** `row` for a row of the backlog, set to the right; `panel` for the panel of a task. */
  look: 'row' | 'panel';
  /** What the field is about, for screen readers: "Points of SSN-4". */
  label: string;
}

/**
 * The story points of a task, changed where they stand. Enter or leaving
 * the field saves; Escape undoes. An empty field takes the points away.
 */
export function PointsEditor({ points, onChange, look, label }: PointsEditorProps) {
  const stored = points === null ? '' : points.toString();
  const [draft, setDraft] = useState(stored);
  const parsed = parsePoints(draft);
  const isUnderstood = parsed !== undefined;

  return (
    <input
      className={classNames(styles.points, styles[look], !isUnderstood && styles.notUnderstood)}
      value={draft}
      placeholder={look === 'row' ? '–' : 'None'}
      inputMode="numeric"
      maxLength={3}
      aria-label={label}
      aria-invalid={!isUnderstood}
      title={isUnderstood ? 'Story points: a whole number' : 'Not understood. Write a whole number'}
      // The row around the field opens its task and can be dragged. A press
      // in the field is meant for the field.
      onPointerDown={(event) => {
        event.stopPropagation();
      }}
      onClick={(event) => {
        event.stopPropagation();
      }}
      onChange={(event) => {
        setDraft(event.target.value);
      }}
      onBlur={() => {
        if (parsed === undefined) {
          setDraft(stored);
        } else if (parsed !== points) {
          void onChange(parsed);
        }
      }}
      onKeyDown={(event) => {
        event.stopPropagation();
        if (event.key === 'Enter') {
          event.currentTarget.blur();
        } else if (event.key === 'Escape') {
          setDraft(stored);
        }
      }}
    />
  );
}
