import { Plus } from 'lucide-react';
import { useState } from 'react';

import { Button } from '@/shared/ui/Button';

import { CreateProjectDialog } from './CreateProjectDialog';
import { PlateFigure } from './PlateFigure';
import styles from './Welcome.module.css';

/** What the user sees before the first project exists. */
export function Welcome() {
  const [isCreatingProject, setIsCreatingProject] = useState(false);

  return (
    <section className={styles.welcome}>
      <div className={styles.words}>
        <p className="label">No projects yet</p>
        <h1 className={styles.headline}>
          You manage.
          <br />
          Your team does the work.
        </h1>
        <p className={styles.text}>
          A project holds the work: tasks on a board, the order they depend on, and who is
          responsible for each.
        </p>
        <div>
          <Button
            variant="primary"
            icon={<Plus />}
            onClick={() => {
              setIsCreatingProject(true);
            }}
          >
            Create a project
          </Button>
        </div>
        <p className={styles.note}>
          This version plans and tracks projects, and an AI specialist on your team can carry out a
          task. A team that plans the project with you and works together is still to come.
        </p>
      </div>

      <div className={styles.figure}>
        <PlateFigure />
      </div>

      <CreateProjectDialog
        open={isCreatingProject}
        onClose={() => {
          setIsCreatingProject(false);
        }}
      />
    </section>
  );
}
