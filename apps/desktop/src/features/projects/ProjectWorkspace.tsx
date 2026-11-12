import { Plus } from 'lucide-react';
import { useState } from 'react';

import { useNavigation, type ProjectView } from '@/app/navigation';
import type { Member, Project, Task } from '@/shared/api/types';
import { classNames } from '@/shared/lib/classNames';
import { Button } from '@/shared/ui/Button';
import { describeFailure } from '@/shared/ui/Toast';

import { BoardView } from '../board/BoardView';
import { GanttView } from '../gantt/GanttView';
import { TaskListView } from '../tasks/TaskListView';
import { TaskPanel } from '../tasks/TaskPanel';
import { useTasks } from '../tasks/taskQueries';
import { TeamProvider } from '../team/TeamContext';
import { TeamStrip } from '../team/TeamStrip';
import { TeamView } from '../team/TeamView';
import { useTeam } from '../team/teamQueries';
import { NewTaskDialog } from './NewTaskDialog';
import styles from './ProjectWorkspace.module.css';
import { TaskTally } from './TaskTally';

/** The ways a project can be looked at, in the order of their tabs. */
const VIEWS: { view: ProjectView; label: string }[] = [
  { view: 'board', label: 'Board' },
  { view: 'list', label: 'List' },
  { view: 'gantt', label: 'Gantt' },
  { view: 'team', label: 'Team' },
];

/** The team while it is loading. It is one value, so that nothing is drawn again for it. */
const NO_MEMBERS: readonly Member[] = [];

/** One project: its name and state, the tabs to switch views, and the chosen view. */
export function ProjectWorkspace({ project }: { project: Project }) {
  const navigation = useNavigation();
  const tasks = useTasks(project.id);
  const team = useTeam(project.id);
  const [isAddingTask, setIsAddingTask] = useState(false);
  const openTask = tasks.data?.find((task) => task.id === navigation.taskId);

  return (
    <TeamProvider members={team.data ?? NO_MEMBERS}>
      <div className={styles.workspace}>
        <header className={styles.header}>
          <div className={styles.heading}>
            <h1 className={styles.name} title={project.description || undefined}>
              {project.name}
            </h1>
            <span className={classNames('label', styles.key)}>{project.key}</span>
            <TeamStrip
              onOpen={() => {
                navigation.showView('team');
              }}
            />
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
          {(tasks.isPending || team.isPending) && (
            <p className={styles.note}>Loading the project…</p>
          )}
          {tasks.isError && (
            <p className={styles.note} role="alert">
              The tasks could not be loaded. {describeFailure(tasks.error)}
            </p>
          )}
          {team.isError && (
            <p className={styles.note} role="alert">
              The team could not be loaded. {describeFailure(team.error)}
            </p>
          )}
          {/* The views wait for the team as well, so that a task never shows a
              placeholder where its assignee is about to appear. */}
          {tasks.data && !team.isPending && (
            <ChosenView view={navigation.view} project={project} tasks={tasks.data} />
          )}

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
    </TeamProvider>
  );
}

interface ChosenViewProps {
  view: ProjectView;
  project: Project;
  tasks: Task[];
}

/** The project in the view the user chose. */
function ChosenView({ view, project, tasks }: ChosenViewProps) {
  switch (view) {
    case 'board':
      return <BoardView project={project} tasks={tasks} />;
    case 'list':
      return <TaskListView project={project} tasks={tasks} />;
    case 'gantt':
      return <GanttView project={project} tasks={tasks} />;
    case 'team':
      return <TeamView project={project} tasks={tasks} />;
  }
}
