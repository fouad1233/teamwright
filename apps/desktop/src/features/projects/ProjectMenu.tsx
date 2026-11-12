import { Archive, Columns3, Ellipsis, Pencil, Trash2 } from 'lucide-react';
import { useState } from 'react';

import type { Project } from '@/shared/api/types';
import { Button, IconButton } from '@/shared/ui/Button';
import { Dialog } from '@/shared/ui/Dialog';
import { FormActions } from '@/shared/ui/Form';
import { ActionMenu } from '@/shared/ui/Menu';

import { useTeamMembers } from '../team/TeamContext';
import { ProjectDetailsDialog } from './ProjectDetailsDialog';
import styles from './ProjectMenu.module.css';
import { StatusesDialog } from './StatusesDialog';
import { useProjectCommands } from './projectQueries';

/** The dialogs the menu can open. One at a time. */
type OpenDialog = 'details' | 'statuses' | 'removal';

interface ProjectMenuProps {
  project: Project;
  /** How many tasks the project has, to say what goes with it when it is deleted. */
  taskCount: number;
}

/**
 * The menu of a project: change its name and description, edit its
 * statuses, put it away, or delete it.
 */
export function ProjectMenu({ project, taskCount }: ProjectMenuProps) {
  const commands = useProjectCommands();
  const memberCount = useTeamMembers().length;
  const [openDialog, setOpenDialog] = useState<OpenDialog | null>(null);

  function close() {
    setOpenDialog(null);
  }

  async function deleteProject() {
    await commands.remove(project.id);
    close();
  }

  return (
    <>
      <ActionMenu
        label="Project"
        actions={[
          {
            label: 'Name and description',
            icon: <Pencil />,
            onSelect: () => {
              setOpenDialog('details');
            },
          },
          {
            label: 'Statuses',
            icon: <Columns3 />,
            onSelect: () => {
              setOpenDialog('statuses');
            },
          },
          {
            label: 'Archive project',
            icon: <Archive />,
            onSelect: () => {
              void commands.archive(project.id);
            },
          },
          {
            label: 'Delete project',
            icon: <Trash2 />,
            isDestructive: true,
            onSelect: () => {
              setOpenDialog('removal');
            },
          },
        ]}
      >
        <IconButton label="Project">
          <Ellipsis />
        </IconButton>
      </ActionMenu>

      <ProjectDetailsDialog open={openDialog === 'details'} project={project} onClose={close} />
      <StatusesDialog open={openDialog === 'statuses'} project={project} onClose={close} />

      <Dialog open={openDialog === 'removal'} title={`Delete ${project.name}?`} onClose={close}>
        <p className={styles.warning}>
          {whatGoesWithIt(taskCount, memberCount)} This cannot be undone. To get the project out of
          the way without losing it, archive it instead.
        </p>
        <FormActions>
          <Button variant="ghost" data-initial-focus onClick={close}>
            Cancel
          </Button>
          <Button
            variant="danger"
            onClick={() => {
              void deleteProject();
            }}
          >
            Delete project
          </Button>
        </FormActions>
      </Dialog>
    </>
  );
}

/** Says what is removed together with a project. */
function whatGoesWithIt(taskCount: number, memberCount: number): string {
  const parts: string[] = [];
  if (taskCount > 0) {
    parts.push(taskCount === 1 ? 'its task' : `its ${taskCount.toString()} tasks`);
  }
  if (memberCount > 0) {
    parts.push(
      memberCount === 1 ? 'its team member' : `its ${memberCount.toString()} team members`,
    );
  }
  return parts.length === 0
    ? 'The project will be removed.'
    : `The project will be removed with ${parts.join(' and ')}.`;
}
