import { Plus } from 'lucide-react';
import { useMemo, useState } from 'react';

import { useNavigation, type ProjectPlace } from '@/app/navigation';
import type { Conversation, Member, Project, Task, View } from '@/shared/api/types';
import { classNames } from '@/shared/lib/classNames';
import { Button } from '@/shared/ui/Button';
import { describeFailure } from '@/shared/ui/Toast';

import { AdvisorView } from '../advisor/AdvisorView';
import { OfficeView } from '../office/OfficeView';
import { TaskPanel } from '../tasks/TaskPanel';
import { useTasks } from '../tasks/taskQueries';
import { TeamProvider } from '../team/TeamContext';
import { TeamStrip } from '../team/TeamStrip';
import { TeamView } from '../team/TeamView';
import { useTeam } from '../team/teamQueries';
import { FilterLine } from '../views/FilterLine';
import { kindOf } from '../views/viewKinds';
import { EVERY_TASK, isFilterable, shownTaskIds, viewToShow } from '../views/viewModel';
import { useViewCommands, useViews } from '../views/viewQueries';
import { ViewTabs } from '../views/ViewTabs';
import { MemberChat } from '../work/MemberChat';
import { WorkProvider } from '../work/WorkContext';
import { useAgentProgram, useConversations } from '../work/workQueries';
import { NewTaskDialog } from './NewTaskDialog';
import { ProjectMenu } from './ProjectMenu';
import styles from './ProjectWorkspace.module.css';
import { TaskTally } from './TaskTally';

/** The team while it is loading. It is one value, so that nothing is drawn again for it. */
const NO_MEMBERS: readonly Member[] = [];

/** The conversations while they are loading. */
const NO_CONVERSATIONS: readonly Conversation[] = [];

/** The views while they are loading. */
const NO_VIEWS: readonly View[] = [];

/** One project: its name and state, the tabs of its views and places, and the one shown. */
export function ProjectWorkspace({ project }: { project: Project }) {
  const navigation = useNavigation();
  const tasks = useTasks(project.id);
  const team = useTeam(project.id);
  const views = useViews(project.id);
  const viewCommands = useViewCommands(project.id);
  const conversations = useConversations(project.id);
  // Whether the program for the specialists is there takes a few seconds to
  // find out. It is asked here, when the project opens, so that the answer
  // is usually known by the time a task or the team is opened.
  useAgentProgram();
  const [isAddingTask, setIsAddingTask] = useState(false);
  const openTask = tasks.data?.find((task) => task.id === navigation.taskId);
  const chatMember = team.data?.find((member) => member.id === navigation.chatMemberId);
  const showing = navigation.showing;
  const shownPlace = 'place' in showing ? showing.place : null;
  // A view that is gone, or that belongs to another project, gives way to
  // the first view of this one.
  const shownView =
    'view' in showing ? viewToShow(views.data ?? NO_VIEWS, showing.view) : undefined;
  const isReady = tasks.data !== undefined && !team.isPending;

  return (
    <TeamProvider members={team.data ?? NO_MEMBERS}>
      <WorkProvider conversations={conversations.data ?? NO_CONVERSATIONS}>
        <div className={styles.workspace}>
          <header className={styles.header}>
            <div className={styles.heading}>
              <h1 className={styles.name} title={project.description || undefined}>
                {project.name}
              </h1>
              <span className={classNames('label', styles.key)}>{project.key}</span>
              <TeamStrip
                onOpen={() => {
                  navigation.showPlace('team');
                }}
              />
              <span className={styles.space} />
              {tasks.data && <TaskTally project={project} tasks={tasks.data} />}
              <ProjectMenu project={project} taskCount={tasks.data?.length ?? 0} />
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

            <ViewTabs
              views={views.data ?? NO_VIEWS}
              shownView={shownView}
              shownPlace={shownPlace}
              commands={viewCommands}
            />
          </header>

          <div className={styles.content}>
            {(tasks.isPending || team.isPending || views.isPending) && (
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
            {views.isError && (
              <p className={styles.note} role="alert">
                The views could not be loaded. {describeFailure(views.error)}
              </p>
            )}
            {/* The views wait for the team as well, so that a task never shows a
              placeholder where its assignee is about to appear. */}
            {isReady && shownPlace !== null && (
              <ShownPlace place={shownPlace} project={project} tasks={tasks.data} />
            )}
            {isReady && shownView && (
              <ShownView
                // A view of its own for every view, so that nothing of one lingers in another.
                key={shownView.id}
                view={shownView}
                project={project}
                tasks={tasks.data}
                onClearFilter={() => {
                  void viewCommands.setFilter(shownView.id, EVERY_TASK);
                }}
              />
            )}

            {tasks.data && openTask && (
              <TaskPanel
                project={project}
                task={openTask}
                tasks={tasks.data}
                onClose={navigation.closeTask}
              />
            )}

            {chatMember && (
              <MemberChat
                // A panel of its own for every member, so nothing typed for one lingers for another.
                key={chatMember.id}
                project={project}
                member={chatMember}
                onClose={navigation.closeChat}
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
      </WorkProvider>
    </TeamProvider>
  );
}

interface ShownViewProps {
  view: View;
  project: Project;
  tasks: Task[];
  onClearFilter: () => void;
}

/**
 * The tasks of the project in one of its views. The filter of the view is
 * applied here, where the tasks are handed to the view, so that the view
 * itself does not need to know of it. A filtered view says so above it.
 */
function ShownView({ view, project, tasks, onClearFilter }: ShownViewProps) {
  const filter = isFilterable(view.kind) ? view.filter : EVERY_TASK;
  const shownIds = useMemo(
    () => shownTaskIds(tasks, project.statuses, filter),
    [tasks, project.statuses, filter],
  );
  const shownTasks = useMemo(
    () => (shownIds === null ? tasks : tasks.filter((task) => shownIds.has(task.id))),
    [tasks, shownIds],
  );
  const shows = useMemo(
    () => (shownIds === null ? undefined : (task: Task) => shownIds.has(task.id)),
    [shownIds],
  );
  const Component = kindOf(view.kind).component;

  return (
    <div className={styles.view}>
      <FilterLine view={{ ...view, filter }} onClear={onClearFilter} />
      <div className={styles.viewBody}>
        <Component project={project} tasks={shownTasks} everyTask={tasks} shows={shows} />
      </div>
    </div>
  );
}

interface ShownPlaceProps {
  place: ProjectPlace;
  project: Project;
  tasks: Task[];
}

/** One of the places of the project: its team, its office or its Advisor. */
function ShownPlace({ place, project, tasks }: ShownPlaceProps) {
  switch (place) {
    case 'team':
      return <TeamView project={project} tasks={tasks} />;
    case 'office':
      return <OfficeView project={project} tasks={tasks} />;
    case 'advisor':
      return <AdvisorView project={project} />;
  }
}
