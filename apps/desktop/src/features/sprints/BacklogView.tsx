import {
  DndContext,
  DragOverlay,
  PointerSensor,
  pointerWithin,
  useSensor,
  useSensors,
  type DragEndEvent,
} from '@dnd-kit/core';
import { Ellipsis, Pencil, Play, Plus, Trash2 } from 'lucide-react';
import { useMemo, useState } from 'react';

import { useNavigation } from '@/app/navigation';
import type { Id, Project, Sprint, Task } from '@/shared/api/types';
import { classNames } from '@/shared/lib/classNames';
import { today as todayWhereTheUserIs } from '@/shared/lib/format';
import { Button, IconButton } from '@/shared/ui/Button';
import { ActionMenu, type Action } from '@/shared/ui/Menu';
import { describeFailure } from '@/shared/ui/Toast';

import { TaskComposer } from '../tasks/TaskComposer';
import { indexById } from '../tasks/taskModel';
import { useTaskCommands } from '../tasks/taskQueries';
import { DraggedRow } from './BacklogRow';
import styles from './BacklogView.module.css';
import { BurndownChart } from './BurndownChart';
import { CompletedSprints } from './CompletedSprints';
import {
  CompleteSprintDialog,
  DeleteSprintDialog,
  SprintDialog,
  StartSprintDialog,
} from './SprintDialogs';
import { BACKLOG_DROP_ID, SprintSection } from './SprintSection';
import {
  daysLeft,
  finishedOf,
  formatDaysLeft,
  formatSprintDates,
  layoutOf,
  openSprints,
  reportVersionOf,
  rowsOf,
} from './sprintModel';
import { useSprintCommands, useSprintReport, useSprints } from './sprintQueries';

/** How far the pointer must move before a press becomes a drag, in pixels. */
const DRAG_START_DISTANCE = 6;

/** Which dialog is open, and for which sprint. */
type OpenDialog =
  { kind: 'create' } | { kind: 'change' | 'start' | 'complete' | 'delete'; sprint: Sprint } | null;

interface BacklogViewProps {
  project: Project;
  tasks: Task[];
}

/**
 * The work of a project as Scrum plans it, read from top to bottom: the
 * active sprint, the sprints planned to come, the backlog of everything
 * that is not planned yet, and at the foot the sprints that are over.
 */
export function BacklogView({ project, tasks }: BacklogViewProps) {
  const navigation = useNavigation();
  const sprints = useSprints(project.id);
  const commands = useSprintCommands(project.id);
  const taskCommands = useTaskCommands(project.id);
  const [dialog, setDialog] = useState<OpenDialog>(null);
  const [draggedId, setDraggedId] = useState<Id | null>(null);
  // Today is read once when the view opens, like the rest of what it shows.
  const [today] = useState(() => todayWhereTheUserIs());

  const tasksById = useMemo(() => indexById(tasks), [tasks]);
  const layout = layoutOf(sprints.data ?? []);
  const destinations = openSprints(sprints.data ?? []);

  const sensors = useSensors(
    // A press only becomes a drag after some movement, so a click opens the task.
    useSensor(PointerSensor, { activationConstraint: { distance: DRAG_START_DISTANCE } }),
  );

  function plan(taskId: Id, sprintId: Id | null) {
    void commands.plan(taskId, sprintId);
  }

  function handleDragEnd(event: DragEndEvent) {
    setDraggedId(null);
    const task = tasksById.get(String(event.active.id));
    if (!task || !event.over) {
      return;
    }
    const target = String(event.over.id);
    const sprintId = target === BACKLOG_DROP_ID ? null : target;
    if (task.sprintId !== sprintId) {
      plan(task.id, sprintId);
    }
  }

  const closeDialog = () => {
    setDialog(null);
  };
  const rowsOfSprint = (sprint: Sprint) => rowsOf(project, tasks, sprint.id);
  const shared = {
    project,
    destinations,
    onOpenTask: navigation.openTask,
    onPlan: plan,
    onSetPoints: (taskId: Id, points: number | null) => commands.setPoints(taskId, points),
  };

  /** The menu of a sprint: what is done less often than starting or completing it. */
  function sprintMenu(sprint: Sprint) {
    const actions: Action[] = [
      {
        label: 'Change sprint',
        icon: <Pencil />,
        onSelect: () => {
          setDialog({ kind: 'change', sprint });
        },
      },
    ];
    if (sprint.state === 'planned') {
      actions.push({
        label: 'Delete sprint',
        icon: <Trash2 />,
        isDestructive: true,
        onSelect: () => {
          setDialog({ kind: 'delete', sprint });
        },
      });
    }
    return (
      <ActionMenu label={`${sprint.name} actions`} actions={actions}>
        <IconButton label="More">
          <Ellipsis />
        </IconButton>
      </ActionMenu>
    );
  }

  if (sprints.isPending) {
    return <p className={styles.note}>Loading the sprints…</p>;
  }
  if (sprints.isError) {
    return (
      <p className={styles.note} role="alert">
        The sprints could not be loaded. {describeFailure(sprints.error)}
      </p>
    );
  }

  const { active, planned, completed } = layout;
  const activeRows = active ? rowsOfSprint(active) : [];
  const finishedInActive = finishedOf(project, activeRows).length;
  const draggedTask = draggedId === null ? undefined : tasksById.get(draggedId);
  const dialogSprint = (kind: 'change' | 'start' | 'complete' | 'delete') =>
    dialog?.kind === kind ? dialog.sprint : null;

  return (
    <DndContext
      sensors={sensors}
      collisionDetection={pointerWithin}
      onDragStart={(event) => {
        setDraggedId(String(event.active.id));
      }}
      onDragEnd={handleDragEnd}
      onDragCancel={() => {
        setDraggedId(null);
      }}
    >
      <div className={styles.backlog}>
        <div className={styles.sheet}>
          {active && (
            <SprintSection
              {...shared}
              sprint={active}
              name={active.name}
              state={<span className={classNames('label', styles.active)}>Active</span>}
              facts={[
                formatSprintDates(active),
                ...(active.endOn === null ? [] : [formatDaysLeft(daysLeft(active.endOn, today))]),
              ]}
              rows={activeRows}
              chart={
                <ActiveBurndown
                  sprintId={active.id}
                  version={reportVersionOf(tasks, active.id)}
                  today={today}
                />
              }
              actions={
                <>
                  <Button
                    onClick={() => {
                      setDialog({ kind: 'complete', sprint: active });
                    }}
                  >
                    Complete sprint
                  </Button>
                  {sprintMenu(active)}
                </>
              }
              emptyNote="Nothing is planned into this sprint. Drag tasks here from the backlog."
            />
          )}

          {planned.map((sprint) => (
            <SprintSection
              key={sprint.id}
              {...shared}
              sprint={sprint}
              name={sprint.name}
              state={<span className="label">Planned</span>}
              facts={[formatSprintDates(sprint)]}
              rows={rowsOfSprint(sprint)}
              actions={
                <>
                  {!active && (
                    <Button
                      icon={<Play />}
                      onClick={() => {
                        setDialog({ kind: 'start', sprint });
                      }}
                    >
                      Start sprint
                    </Button>
                  )}
                  {sprintMenu(sprint)}
                </>
              }
              emptyNote="Nothing is planned into this sprint yet. Drag tasks here from the backlog."
            />
          ))}

          <SprintSection
            {...shared}
            sprint={null}
            name="Backlog"
            rows={rowsOf(project, tasks, null)}
            actions={
              <Button
                icon={<Plus />}
                onClick={() => {
                  setDialog({ kind: 'create' });
                }}
              >
                New sprint
              </Button>
            }
            emptyNote="The backlog is empty: every task is planned into a sprint."
            footer={
              <div className={styles.composer}>
                <TaskComposer
                  onCreate={async (title) => {
                    const created = await taskCommands.create({ title });
                    return created !== undefined;
                  }}
                />
              </div>
            }
          />

          <CompletedSprints
            project={project}
            sprints={completed}
            tasks={tasks}
            onOpenTask={navigation.openTask}
          />
        </div>
      </div>

      <DragOverlay dropAnimation={null}>
        {draggedTask && <DraggedRow project={project} task={draggedTask} />}
      </DragOverlay>

      <SprintDialog
        open={dialog?.kind === 'create' || dialog?.kind === 'change'}
        // A form of its own for every sprint, so nothing typed for one lingers for another.
        key={dialog?.kind === 'change' ? dialog.sprint.id : 'new'}
        commands={commands}
        sprint={dialogSprint('change') ?? undefined}
        onClose={closeDialog}
      />
      <StartSprintDialog
        sprint={dialogSprint('start')}
        commands={commands}
        today={today}
        onClose={closeDialog}
      />
      <CompleteSprintDialog
        sprint={dialogSprint('complete')}
        finished={finishedInActive}
        total={activeRows.length}
        plannedSprints={planned}
        commands={commands}
        onClose={closeDialog}
      />
      <DeleteSprintDialog
        sprint={dialogSprint('delete')}
        taskCount={(() => {
          const sprint = dialogSprint('delete');
          return sprint ? rowsOfSprint(sprint).length : 0;
        })()}
        onDelete={(sprintId) => {
          closeDialog();
          void commands.remove(sprintId);
        }}
        onClose={closeDialog}
      />
    </DndContext>
  );
}

interface ActiveBurndownProps {
  sprintId: Id;
  /** Changes whenever the tasks of the sprint change, so the report is asked for again. */
  version: string;
  today: string;
}

/** The burndown of the active sprint, once its report has come. */
function ActiveBurndown({ sprintId, version, today }: ActiveBurndownProps) {
  const report = useSprintReport(sprintId, version);
  return report.data ? <BurndownChart report={report.data} today={today} /> : null;
}
