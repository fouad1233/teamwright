import { useState } from 'react';

import type { Project } from '@/shared/api/types';
import { describeFailure, useNotifier } from '@/shared/ui/Toast';

import { useProjectCommands } from '../projects/projectQueries';
import styles from './ProjectNote.module.css';

/**
 * The note the manager keeps about the project: its description, written
 * in place. It is saved when the field loses the focus, the way the
 * description of a task is, and Escape puts back what was stored.
 */
export function ProjectNote({ project }: { project: Project }) {
  const commands = useProjectCommands();
  const notifier = useNotifier();
  const [draft, setDraft] = useState(project.description);

  async function save() {
    try {
      await commands.change(project, { name: project.name, description: draft });
    } catch (failure) {
      notifier.showError(describeFailure(failure));
      setDraft(project.description);
    }
  }

  return (
    <section className={styles.note} aria-label="Your note">
      <h2 className="label">Your note</h2>
      <textarea
        className={styles.text}
        value={draft}
        rows={4}
        placeholder="What the project is for, and what you want to remember about it."
        aria-label="Your note about the project"
        onChange={(event) => {
          setDraft(event.target.value);
        }}
        onBlur={() => {
          if (draft.trim() !== project.description) {
            void save();
          }
        }}
        onKeyDown={(event) => {
          if (event.key === 'Escape') {
            event.stopPropagation();
            setDraft(project.description);
          }
        }}
      />
    </section>
  );
}
