import { Plus } from 'lucide-react';
import { useState } from 'react';

import { useNavigation, type ProjectView } from '@/app/navigation';
import type { Project } from '@/shared/api/types';
import { classNames } from '@/shared/lib/classNames';
import { Button } from '@/shared/ui/Button';
import { describeFailure } from '@/shared/ui/Toast';

import { BoardView } from '../board/BoardView';
import { TaskListView } from '../tasks/TaskListView';
import { TaskPanel } from '../tasks/TaskPanel';
import { useTasks } from '../tasks/taskQueries';
import { NewTaskDialog } from './NewTaskDialog';
import styles from './ProjectWorkspace.module.css';
import { TaskTally } from './TaskTally';

/** The ways a project can be looked at, in the order of their tabs. */
const VIEWS: { view: ProjectView; label: string }[] = [
  { view: 'board', label: 'Board' },
  { view: 'list', label: 'List' },
];

/** One project: its name and state, the tabs to switch views, and the chosen view. */
export function ProjectWorkspace({ project }: { project: Project }) {
  const navigation = useNavigation();
  const tasks = useTasks(project.id);
  const [isAddingTask, setIsAddingTask] = useState(false);
  const openTask = tasks.data?.find((task) => task.id === navigation.taskId);

  return (
    <div className={styles.workspace}>
      <header className={styles.header}>
        <div className={styles.heading}>
          <h1 className={styles.name} title={project.description || undefined}>
            {project.name}
          </h1>
          <span className={classNames('label', styles.key)}>{project.key}</span>
          <span className={styles.space} />
          {tasks.data && <TaskTally project={project} tasks={tasks.data} />}
          <Button
            variant="primary"
            icon={<Plus />}
            onClick={() => {
              setIsAddingTask(true);
            }}
          >
            New task
          </Button>
        </div>

        <div className={styles.tabs} role="tablist" aria-label="Views">
          {VIEWS.map(({ view, label }) => (
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
          <TaskPanel
            project={project}
            task={openTask}
            tasks={tasks.data}
            onClose={navigation.closeTask}
          />
        )}
      </div>

      <NewTaskDialog
        open={isAddingTask}
        project={project}
        onClose={() => {
          setIsAddingTask(false);
        }}
      />
    </div>
  );
}
