import { FlaskConical, Plus } from 'lucide-react';
import { useState } from 'react';

import { useNavigation } from '@/app/navigation';
import { useIsDemo } from '@/shared/api/BackendProvider';
import type { Id, Project } from '@/shared/api/types';
import { classNames } from '@/shared/lib/classNames';
import { IconButton } from '@/shared/ui/Button';

import { CreateProjectDialog } from './CreateProjectDialog';
import styles from './Sidebar.module.css';

interface SidebarProps {
  projects: Project[];
  /** The project that is open, if any. */
  activeProjectId: Id | null;
}

/** The column on the left: the name of the application and the list of projects. */
export function Sidebar({ projects, activeProjectId }: SidebarProps) {
  const navigation = useNavigation();
  const isDemo = useIsDemo();
  const [isCreatingProject, setIsCreatingProject] = useState(false);

  return (
    <aside className={styles.sidebar}>
      <div className={styles.brand}>
        <img className={styles.logo} src="/teamwright.svg" alt="" />
        <span className={styles.name}>Teamwright</span>
      </div>

      <nav className={styles.section} aria-label="Projects">
        <div className={styles.sectionHeader}>
          <span className={styles.sectionTitle}>Projects</span>
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
              <li key={project.id}>
                <button
                  type="button"
                  className={classNames(
                    styles.project,
                    project.id === activeProjectId && styles.activeProject,
                  )}
                  aria-current={project.id === activeProjectId ? 'page' : undefined}
                  onClick={() => {
                    navigation.openProject(project.id);
                  }}
                >
                  <span className={styles.projectKey}>{project.key}</span>
                  <span className={styles.projectName}>{project.name}</span>
                </button>
              </li>
            ))}
          </ul>
        )}
      </nav>

      {isDemo && (
        <p className={styles.demoNote}>
          <FlaskConical className={styles.demoIcon} />
          Demo with sample data. Nothing you change here is saved.
        </p>
      )}

      <CreateProjectDialog
        open={isCreatingProject}
        onClose={() => {
          setIsCreatingProject(false);
        }}
      />
    </aside>
  );
}
