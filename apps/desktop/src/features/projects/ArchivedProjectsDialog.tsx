import { useState } from 'react';

import { useNavigation } from '@/app/navigation';
import type { Id, Project } from '@/shared/api/types';
import { calendarDateOf, formatDate } from '@/shared/lib/format';
import { Button } from '@/shared/ui/Button';
import { Dialog } from '@/shared/ui/Dialog';
import { FormActions } from '@/shared/ui/Form';

import styles from './ArchivedProjectsDialog.module.css';
import { useProjectCommands } from './projectQueries';

interface ArchivedProjectsDialogProps {
  open: boolean;
  /** The projects that were put away, the one that was put away last first. */
  projects: Project[];
  onClose: () => void;
}

/**
 * The projects that were put away. Each one can be brought back, or deleted
 * for good.
 */
export function ArchivedProjectsDialog({ open, projects, onClose }: ArchivedProjectsDialogProps) {
  const commands = useProjectCommands();
  const navigation = useNavigation();
  /** The project that is asked whether it should really be deleted. */
  const [doomedId, setDoomedId] = useState<Id | null>(null);

  async function restore(project: Project) {
    const wasRestored = await commands.restore(project.id);
    if (wasRestored) {
      navigation.openProject(project.id);
      onClose();
    }
  }

  async function deleteForGood(project: Project) {
    await commands.remove(project.id);
    setDoomedId(null);
  }

  return (
    <Dialog open={open} title="Archived projects" size="wide" onClose={onClose}>
      {projects.length === 0 ? (
        <p className={styles.nothing}>Nothing is archived.</p>
      ) : (
        <ul className={styles.projects}>
          {projects.map((project) => (
            <li key={project.id} className={styles.project}>
              <div className={styles.words}>
                <span className={styles.name}>{project.name}</span>
                <span className={styles.facts}>
                  {project.key}
                  {project.archivedAt !== null &&
                    ` · archived ${formatDate(calendarDateOf(new Date(project.archivedAt)))}`}
                </span>
              </div>

              {doomedId === project.id ? (
                <>
                  <span className={styles.question}>Delete it for good?</span>
                  <Button
                    variant="ghost"
                    onClick={() => {
                      setDoomedId(null);
                    }}
                  >
                    Keep
                  </Button>
                  <Button
                    variant="danger"
                    onClick={() => {
                      void deleteForGood(project);
                    }}
                  >
                    Delete
                  </Button>
                </>
              ) : (
                <>
                  <Button
                    variant="ghost"
                    onClick={() => {
                      setDoomedId(project.id);
                    }}
                  >
                    Delete
                  </Button>
                  <Button
                    variant="secondary"
                    onClick={() => {
                      void restore(project);
                    }}
                  >
                    Bring back
                  </Button>
                </>
              )}
            </li>
          ))}
        </ul>
      )}

      <FormActions>
        <Button variant="secondary" data-initial-focus onClick={onClose}>
          Done
        </Button>
      </FormActions>
    </Dialog>
  );
}
