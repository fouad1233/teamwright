import { Inbox, Plus } from 'lucide-react';
import { useState } from 'react';

import { useNavigation } from '@/app/navigation';
import { useIsDemo } from '@/shared/api/BackendProvider';
import type { Id, Project } from '@/shared/api/types';
import { classNames } from '@/shared/lib/classNames';
import { IconButton } from '@/shared/ui/Button';
import { Logo } from '@/shared/ui/Logo';

import { useConversationsWaiting } from '../work/workQueries';
import { ArchivedProjectsDialog } from './ArchivedProjectsDialog';
import { CreateProjectDialog } from './CreateProjectDialog';
import styles from './Sidebar.module.css';
import { useArchivedProjects } from './projectQueries';

interface SidebarProps {
  projects: Project[];
  /** The project that is open, if any. */
  activeProjectId: Id | null;
}

/**
 * The column on the left: the name of the application, the way to the
 * inbox, and the list of projects.
 */
export function Sidebar({ projects, activeProjectId }: SidebarProps) {
  const navigation = useNavigation();
  const isDemo = useIsDemo();
  const archived = useArchivedProjects();
  const [isCreatingProject, setIsCreatingProject] = useState(false);
  const [isShowingArchive, setIsShowingArchive] = useState(false);
  const archivedCount = archived.data?.length ?? 0;
  const waitingCount = useConversationsWaiting().data?.length ?? 0;
  const isInInbox = navigation.place === 'inbox';

  return (
    <aside className={styles.sidebar}>
      <div className={styles.brand}>
        <Logo size={20} />
        <span className={styles.wordmark}>Teamwright</span>
      </div>

      <button
        type="button"
        className={classNames(styles.inbox, isInInbox && styles.activeInbox)}
        aria-current={isInInbox ? 'page' : undefined}
        onClick={navigation.openInbox}
      >
        <Inbox />
        <span className={styles.inboxName}>Inbox</span>
        {waitingCount > 0 && (
          <span
            className={styles.waiting}
            aria-label={`${waitingCount.toString()} waiting for you`}
          >
            {waitingCount}
          </span>
        )}
      </button>

      <nav className={styles.section} aria-label="Projects">
        <div className={styles.sectionHeader}>
          <span className="label">Projects</span>
          <IconButton
            label="New project"
            onClick={() => {
              setIsCreatingProject(true);
            }}
          >
            <Plus />
          </IconButton>
        </div>

        {projects.length === 0 ? (
          <p className={styles.empty}>No projects yet.</p>
        ) : (
          <ul className={styles.projects}>
            {projects.map((project) => {
              const isActive = project.id === activeProjectId;
              return (
                <li key={project.id}>
                  <button
                    type="button"
                    className={classNames(styles.project, isActive && styles.activeProject)}
                    aria-current={isActive ? 'page' : undefined}
                    onClick={() => {
                      navigation.openProject(project.id);
                    }}
                  >
                    <span className={styles.projectName}>{project.name}</span>
                    <span className={styles.projectKey}>{project.key}</span>
                  </button>
                </li>
              );
            })}
          </ul>
        )}

        {archivedCount > 0 && (
          <button
            type="button"
            className={styles.archive}
            onClick={() => {
              setIsShowingArchive(true);
            }}
          >
            Archived
            <span className={styles.archiveCount}>{archivedCount}</span>
          </button>
        )}
      </nav>

      {isDemo && (
        <p className={classNames('label', styles.demoNote)}>
          Demo data
          <br />
          Changes are not saved
        </p>
      )}

      <CreateProjectDialog
        open={isCreatingProject}
        onClose={() => {
          setIsCreatingProject(false);
        }}
      />
      <ArchivedProjectsDialog
        open={isShowingArchive}
        projects={archived.data ?? []}
        onClose={() => {
          setIsShowingArchive(false);
        }}
      />
    </aside>
  );
}
