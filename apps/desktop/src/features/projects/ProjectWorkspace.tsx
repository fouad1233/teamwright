import { ChevronRight, Plus } from 'lucide-react';
import { useMemo, useState } from 'react';

import { useNavigation, type ProjectPlace } from '@/app/navigation';
import type { Area, Conversation, Member, Project, Task, View } from '@/shared/api/types';
import { classNames } from '@/shared/lib/classNames';
import { Button } from '@/shared/ui/Button';
import { describeFailure } from '@/shared/ui/Toast';

import { AdvisorView } from '../advisor/AdvisorView';
import { OpenAreaProvider } from '../areas/AreaContext';
import { areaToShow, tasksOfArea } from '../areas/areaModel';
import { useAreas } from '../areas/areaQueries';
import { OfficeView } from '../office/OfficeView';
import { TaskPanel } from '../tasks/TaskPanel';
import { useTasks } from '../tasks/taskQueries';
import { TeamProvider } from '../team/TeamContext';
import { TeamStrip } from '../team/TeamStrip';
import { TeamView } from '../team/TeamView';
import { useTeam } from '../team/teamQueries';
import { FilterLine } from '../views/FilterLine';
import { kindOf } from '../views/viewKinds';
import {
  EVERY_TASK,
  assigneeForNewTasks,
  isFilterable,
  shownTaskIds,
  viewToShow,
} from '../views/viewModel';
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

/** The areas while they are loading. */
const NO_AREAS: readonly Area[] = [];

/** One project: its name and state, the tabs of its views and places, and the one shown. */
export function ProjectWorkspace({ project }: { project: Project }) {
  const navigation = useNavigation();
  const tasks = useTasks(project.id);
  const team = useTeam(project.id);
  const views = useViews(project.id);
  const areas = useAreas(project.id);
  // An area that is gone, or that belongs to another project, gives way to
  // the whole project. So does the one area of a project that has one.
  const openArea = areaToShow(areas.data ?? NO_AREAS, navigation.areaId);
  const areaTasks = useMemo(
    () => (tasks.data === undefined ? undefined : tasksOfArea(tasks.data, openArea)),
    [tasks.data, openArea],
  );
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
        <OpenAreaProvider area={openArea}>
          <div className={styles.workspace}>
            <header className={styles.header}>
              <div className={styles.heading}>
                {openArea ? (
                  <div className={styles.withArea}>
                    <h1 className={classNames(styles.name, styles.projectOfArea)}>
                      <button
                        type="button"
                        className={styles.up}
                        title="Show every area of the project"
                        onClick={() => {
                          navigation.openProject(project.id);
                        }}
                      >
                        {project.name}
                      </button>
                    </h1>
                    <ChevronRight className={styles.step} aria-hidden />
                    <span className={classNames(styles.name, styles.areaName)}>
                      {openArea.name}
                    </span>
                  </div>
                ) : (
                  <h1 className={styles.name} title={project.description || undefined}>
                    {project.name}
                  </h1>
                )}
                <span className={classNames('label', styles.key)}>{project.key}</span>
                <TeamStrip
                  onOpen={() => {
                    navigation.showPlace('team');
                  }}
                />
                <span className={styles.space} />
                {areaTasks && (
                  <TaskTally project={project} tasks={areaTasks} areaName={openArea?.name} />
                )}
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
              {isReady && shownView && areaTasks && (
                <ShownView
                  // A view of its own for every view and area, so that nothing
                  // of one lingers in another.
                  key={`${shownView.id}:${openArea?.id ?? 'every area'}`}
                  view={shownView}
                  project={project}
                  area={openArea}
                  tasks={areaTasks}
                  everyTask={tasks.data}
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
              // A task made while a view of one person's tasks is open goes to
              // that person, so that it stays in the view.
              assignee={
                shownView && isFilterable(shownView.kind)
                  ? assigneeForNewTasks(shownView.filter)
                  : undefined
              }
              onClose={() => {
                setIsAddingTask(false);
              }}
            />
          </div>
        </OpenAreaProvider>
      </WorkProvider>
    </TeamProvider>
  );
}

interface ShownViewProps {
  view: View;
  project: Project;
  /** The area that is open, or nothing for every area. */
  area: Area | undefined;
  /** The tasks of the open area, or of the whole project. */
  tasks: Task[];
  /** Every task of the project, whatever its area. */
  everyTask: Task[];
  onClearFilter: () => void;
}

/**
 * The tasks of the project in one of its views. The open area and the
 * filter of the view are applied here, where the tasks are handed to the
 * view, so that the view itself does not need to know of them. A filtered
 * view says so above it.
 */
function ShownView({ view, project, area, tasks, everyTask, onClearFilter }: ShownViewProps) {
  const filter = isFilterable(view.kind) ? view.filter : EVERY_TASK;
  const shownIds = useMemo(
    () => shownTaskIds(tasks, project.statuses, filter),
    [tasks, project.statuses, filter],
  );
  const shownTasks = useMemo(
    () => (shownIds === null ? tasks : tasks.filter((task) => shownIds.has(task.id))),
    [tasks, shownIds],
  );
  // The board keeps every task of the project and is told which get a card:
  // those the filter lets through, in the open area.
  const shows = useMemo(() => {
    if (shownIds !== null) {
      return (task: Task) => shownIds.has(task.id);
    }
    if (area !== undefined) {
      return (task: Task) => task.areaId === area.id;
    }
    return undefined;
  }, [shownIds, area]);
  const Component = kindOf(view.kind).component;

  return (
    <div className={styles.view}>
      <FilterLine view={{ ...view, filter }} onClear={onClearFilter} />
      <div className={styles.viewBody}>
        <Component
          project={project}
          tasks={shownTasks}
          everyTask={everyTask}
          shows={shows}
          assigneeOfNewTasks={assigneeForNewTasks(filter)}
        />
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
