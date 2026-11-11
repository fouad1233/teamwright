import { Plus } from 'lucide-react';
import { useState } from 'react';

import { Button } from '@/shared/ui/Button';

import { CreateProjectDialog } from './CreateProjectDialog';
import styles from './Welcome.module.css';

/** What the user sees before the first project exists. */
export function Welcome() {
  const [isCreatingProject, setIsCreatingProject] = useState(false);

  return (
    <section className={styles.welcome}>
      <img className={styles.logo} src="/teamwright.svg" alt="" />
      <h1 className={styles.headline}>You manage. Your team does the work.</h1>
      <p className={styles.text}>
        A project holds the work: tasks on a board, the order they depend on, and who is responsible
        for each. Start one to plan and track it here.
      </p>
      <p className={styles.note}>
        The AI team that will carry out the tasks is the next milestone of Teamwright. This version
        manages projects and tasks.
      </p>
      <Button
        variant="primary"
        icon={<Plus />}
        onClick={() => {
          setIsCreatingProject(true);
        }}
      >
        Create a project
      </Button>

      <CreateProjectDialog
        open={isCreatingProject}
        onClose={() => {
          setIsCreatingProject(false);
        }}
      />
    </section>
  );
}
