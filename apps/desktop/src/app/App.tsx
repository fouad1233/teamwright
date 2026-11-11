import { ProjectWorkspace } from '@/features/projects/ProjectWorkspace';
import { Sidebar } from '@/features/projects/Sidebar';
import { Welcome } from '@/features/projects/Welcome';
import { useProjects } from '@/features/projects/projectQueries';
import { describeFailure } from '@/shared/ui/Toast';

import styles from './App.module.css';
import { useNavigation } from './navigation';

/** The window: the sidebar on the left and the open project next to it. */
export function App() {
  const navigation = useNavigation();
  const projects = useProjects();

  // The project the user was in last, or the first one if that is gone.
  const openProject =
    projects.data?.find((project) => project.id === navigation.projectId) ?? projects.data?.[0];

  return (
    <div className={styles.app}>
      <Sidebar projects={projects.data ?? []} activeProjectId={openProject?.id ?? null} />

      <main className={styles.main}>
        {projects.isPending && <p className={styles.note}>Loading…</p>}
        {projects.isError && (
          <p className={styles.note} role="alert">
            The projects could not be loaded. {describeFailure(projects.error)}
          </p>
        )}
        {projects.data &&
          (openProject ? (
            // The key gives every project a fresh workspace, so nothing of
            // one project lingers on screen when another is opened.
            <ProjectWorkspace key={openProject.id} project={openProject} />
          ) : (
            <Welcome />
          ))}
      </main>
    </div>
  );
}
