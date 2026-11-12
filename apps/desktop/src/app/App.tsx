import { Inbox } from '@/features/inbox/Inbox';
import { ProjectWorkspace } from '@/features/projects/ProjectWorkspace';
import { Sidebar } from '@/features/projects/Sidebar';
import { Welcome } from '@/features/projects/Welcome';
import { useProjects } from '@/features/projects/projectQueries';
import { ComputersPlace } from '@/features/sync/ComputersPlace';
import { useFreshDataFromOtherComputers } from '@/features/sync/syncQueries';
import { describeFailure } from '@/shared/ui/Toast';

import styles from './App.module.css';
import { useNavigation } from './navigation';

/**
 * The window: the sidebar on the left, and next to it the open project, the
 * inbox or the computers.
 */
export function App() {
  const navigation = useNavigation();
  const projects = useProjects();
  // What another computer changed is shown wherever the manager looks.
  useFreshDataFromOtherComputers();

  // The project the user was in last, or the first one if that is gone.
  const openProject =
    projects.data?.find((project) => project.id === navigation.projectId) ?? projects.data?.[0];

  return (
    <div className={styles.app}>
      <Sidebar
        projects={projects.data ?? []}
        activeProjectId={navigation.place === 'project' ? (openProject?.id ?? null) : null}
      />

      <main className={styles.main}>
        {projects.isPending && <p className={styles.note}>Loading…</p>}
        {projects.isError && (
          <p className={styles.note} role="alert">
            The projects could not be loaded. {describeFailure(projects.error)}
          </p>
        )}
        {projects.data && navigation.place === 'inbox' && <Inbox projects={projects.data} />}
        {projects.data && navigation.place === 'computers' && (
          <ComputersPlace projects={projects.data} />
        )}
        {projects.data &&
          navigation.place === 'project' &&
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
