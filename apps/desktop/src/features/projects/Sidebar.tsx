import { Inbox, Plus } from 'lucide-react';
import { useState } from 'react';

import { useNavigation } from '@/app/navigation';
import { useIsDemo } from '@/shared/api/BackendProvider';
import type { Id, Project } from '@/shared/api/types';
import { classNames } from '@/shared/lib/classNames';
import { IconButton } from '@/shared/ui/Button';
import { Logo } from '@/shared/ui/Logo';

import { areaToShow, showsAreas } from '../areas/areaModel';
import { useAreas } from '../areas/areaQueries';
import { SidebarAreas } from '../areas/SidebarAreas';
import { SyncStateLine } from '../sync/SyncStateLine';
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
 * inbox, the list of projects, and at its foot the way to the computers
 * with the state of syncing.
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
            {projects.map((project) => (
              <ProjectEntry
                key={project.id}
                project={project}
                isActive={project.id === activeProjectId}
              />
            ))}
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

      <div className={styles.foot}>
        <SyncStateLine />
      </div>

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

interface ProjectEntryProps {
  project: Project;
  /** Whether the project is the one on screen. */
  isActive: boolean;
}

/**
 * One project in the list, with its areas indented under it once it has
 * two. Brass marks where the manager is: the project when all of it is
 * shown, or the area that is open.
 */
function ProjectEntry({ project, isActive }: ProjectEntryProps) {
  const navigation = useNavigation();
  const areas = useAreas(project.id).data ?? [];
  const openArea = isActive ? areaToShow(areas, navigation.areaId) : undefined;

  return (
    <li>
      <button
        type="button"
        className={classNames(
          styles.project,
          isActive && (openArea ? styles.holdsOpenArea : styles.activeProject),
        )}
        aria-current={isActive && !openArea ? 'page' : undefined}
        title={openArea ? 'Show every area of the project' : undefined}
        onClick={() => {
          navigation.openProject(project.id);
        }}
      >
        <span className={styles.projectName}>{project.name}</span>
        <span className={styles.projectKey}>{project.key}</span>
      </button>
      {showsAreas(areas) && (
        <SidebarAreas
          areas={areas}
          openAreaId={openArea?.id ?? null}
          onOpen={(areaId) => {
            navigation.openArea(project.id, areaId);
          }}
        />
      )}
    </li>
  );
}
