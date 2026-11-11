import { Columns3, List } from 'lucide-react';
import type { ReactNode } from 'react';

import { useNavigation, type ProjectView } from '@/app/navigation';
import type { Project } from '@/shared/api/types';
import { classNames } from '@/shared/lib/classNames';
import { describeFailure } from '@/shared/ui/Toast';

import { BoardView } from '../board/BoardView';
import { TaskDrawer } from '../tasks/TaskDrawer';
import { TaskListView } from '../tasks/TaskListView';
import { useTasks } from '../tasks/taskQueries';
import styles from './ProjectWorkspace.module.css';

/** The ways a project can be looked at, in the order of their tabs. */
const VIEWS: { view: ProjectView; label: string; icon: ReactNode }[] = [
  { view: 'board', label: 'Board', icon: <Columns3 /> },
  { view: 'list', label: 'List', icon: <List /> },
];

/** One project: its name, the tabs to switch views, and the chosen view. */
export function ProjectWorkspace({ project }: { project: Project }) {
  const navigation = useNavigation();
  const tasks = useTasks(project.id);
  const openTask = tasks.data?.find((task) => task.id === navigation.taskId);

  return (
    <div className={styles.workspace}>
      <header className={styles.header}>
        <div className={styles.heading}>
          <span className={styles.key}>{project.key}</span>
          <h1 className={styles.name}>{project.name}</h1>
        </div>
        {project.description !== '' && (
          <p className={styles.description} data-selectable>
            {project.description}
          </p>
        )}

        <div className={styles.tabs} role="tablist" aria-label="Views">
          {VIEWS.map(({ view, label, icon }) => (
            <button
              key={view}
              type="button"
              role="tab"
              aria-selected={navigation.view === view}
              className={classNames(styles.tab, navigation.view === view && styles.activeTab)}
              onClick={() => {
                navigation.showView(view);
              }}
            >
              {icon}
              {label}
            </button>
          ))}
        </div>
      </header>

      <div className={styles.content}>
        {tasks.isPending && <p className={styles.note}>Loading the tasks…</p>}
        {tasks.isError && (
          <p className={styles.note} role="alert">
            The tasks could not be loaded. {describeFailure(tasks.error)}
          </p>
        )}
        {tasks.data &&
          (navigation.view === 'board' ? (
            <BoardView project={project} tasks={tasks.data} />
          ) : (
            <TaskListView project={project} tasks={tasks.data} />
          ))}

        {tasks.data && openTask && (
          <TaskDrawer
            project={project}
            task={openTask}
            tasks={tasks.data}
            onClose={navigation.closeTask}
          />
        )}
      </div>
    </div>
  );
}
