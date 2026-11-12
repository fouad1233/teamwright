import { CornerDownRight, Ellipsis, Trash2, X } from 'lucide-react';
import { useEffect, useMemo, useRef, useState, type KeyboardEvent, type ReactNode } from 'react';

import { useNavigation } from '@/app/navigation';
import type { Assignee, Id, Project, Task } from '@/shared/api/types';
import { classNames } from '@/shared/lib/classNames';
import { formatEstimate, parseEstimate, taskReference } from '@/shared/lib/format';
import { Button, IconButton, ValueButton } from '@/shared/ui/Button';
import { DateField } from '@/shared/ui/DateField';
import { Dialog } from '@/shared/ui/Dialog';
import { ActionMenu, ChoiceMenu, type Choice } from '@/shared/ui/Menu';
import { useLeaving } from '@/shared/ui/motion';

import {
  AssigneeAvatar,
  AssigneeName,
  MANAGER_NAME,
  MemberAvatar,
  NOBODY_NAME,
} from '../team/Assignee';
import { PointsEditor } from '../sprints/PointsEditor';
import { useSprintCommands, useSprints } from '../sprints/sprintQueries';
import { useTeamMembers } from '../team/TeamContext';
import { TaskWork } from '../work/TaskWork';
import { PRIORITIES, PriorityFlag, StatusLabel, StatusMark, priorityLabel } from './TaskBadges';
import { TaskComposer } from './TaskComposer';
import { TaskHistory } from './TaskHistory';
import styles from './TaskPanel.module.css';
import { TaskPicker } from './TaskPicker';
import { indexById, statusOf, subtasksOf } from './taskModel';
import { useTaskCommands } from './taskQueries';

/** The value of the assignee menu that stands for "nobody". */
const UNASSIGNED = 'unassigned';
/** The value of the assignee menu that stands for the manager. */
const MANAGER = 'manager';
/** Moving a task to this position puts it at the bottom of a status. */
const BOTTOM = Number.MAX_SAFE_INTEGER;

interface TaskPanelProps {
  project: Project;
  task: Task;
  /** Every task of the project, to show the tasks this one is connected to. */
  tasks: Task[];
  onClose: () => void;
}

/** The panel that shows one task in full and lets the manager change it. */
export function TaskPanel({ project, task, tasks, onClose }: TaskPanelProps) {
  const commands = useTaskCommands(project.id);
  const sprintCommands = useSprintCommands(project.id);
  const navigation = useNavigation();
  const team = useTeamMembers();
  const panel = useRef<HTMLElement>(null);
  // The panel closes with a movement of its own. Opening another task on
  // the way calls the closing off.
  const { isLeaving, leave } = useLeaving(panel, task.id, onClose);
  const [isConfirmingDeletion, setIsConfirmingDeletion] = useState(false);

  const tasksById = useMemo(() => indexById(tasks), [tasks]);
  const reference = taskReference(project, task);
  const status = statusOf(project, task);
  const parent = task.parentId === null ? undefined : tasksById.get(task.parentId);
  const byNumber = (first: Task, second: Task) => first.number - second.number;
  const blockers = tasks.filter((other) => task.blockedBy.includes(other.id)).sort(byNumber);
  const possibleBlockers = tasks
    .filter((other) => other.id !== task.id && !task.blockedBy.includes(other.id))
    .sort(byNumber);

  const statusChoices: Choice<Id>[] = project.statuses.map((choice) => ({
    value: choice.id,
    label: choice.name,
    icon: <StatusMark status={choice} />,
  }));
  const assigneeChoices: Choice<string>[] = [
    { value: UNASSIGNED, label: NOBODY_NAME, icon: <AssigneeAvatar assignee={null} /> },
    {
      value: MANAGER,
      label: MANAGER_NAME,
      icon: <AssigneeAvatar assignee={{ kind: 'manager' }} />,
    },
    ...team.map((member) => ({
      value: member.id,
      label: member.name,
      icon: <MemberAvatar member={member} />,
    })),
  ];
  const priorityChoices = PRIORITIES.map((priority) => ({
    value: priority,
    label: priorityLabel(priority),
    icon: <PriorityFlag priority={priority} />,
  }));

  // Moving the focus into the panel lets Escape close it right away.
  useEffect(() => {
    panel.current?.focus();
  }, [task.id]);

  function closeOnEscape(event: KeyboardEvent) {
    if (event.key === 'Escape') {
      leave();
    }
  }

  async function deleteTask() {
    const wasRemoved = await commands.remove(task.id);
    if (wasRemoved) {
      onClose();
    }
  }

  return (
    <aside
      ref={panel}
      className={classNames(styles.panel, isLeaving && styles.leaving)}
      aria-label={`Task ${reference}`}
      tabIndex={-1}
      onKeyDown={closeOnEscape}
    >
      <header className={styles.header}>
        <span className={styles.reference}>{reference}</span>
        {parent && (
          <button
            type="button"
            className={styles.parent}
            onClick={() => {
              navigation.openTask(parent.id);
            }}
          >
            <CornerDownRight />
            Part of {taskReference(project, parent)}
          </button>
        )}
        <span className={styles.space} />
        <ActionMenu
          label="Task actions"
          actions={[
            {
              label: 'Delete task',
              icon: <Trash2 />,
              isDestructive: true,
              onSelect: () => {
                setIsConfirmingDeletion(true);
              },
            },
          ]}
        >
          <IconButton label="More">
            <Ellipsis />
          </IconButton>
        </ActionMenu>
        <IconButton label="Close" onClick={leave}>
          <X />
        </IconButton>
      </header>

      <div className={styles.body}>
        {/* Each editor keeps a draft while the user types. The key makes it
            start again from the stored value whenever that value changes. */}
        <TitleEditor
          key={`title:${task.id}:${task.title}`}
          title={task.title}
          onRename={(title) => commands.rename(task.id, title)}
        />

        <div className={styles.properties}>
          <Property label="Status">
            <ChoiceMenu
              label="Status"
              choices={statusChoices}
              value={task.statusId}
              onChoose={(statusId) => {
                void commands.move(task.id, statusId, BOTTOM);
              }}
            >
              <ValueButton aria-label="Status">
                {status && <StatusLabel status={status} />}
              </ValueButton>
            </ChoiceMenu>
          </Property>

          <Property label="Assignee">
            <ChoiceMenu
              label="Assignee"
              choices={assigneeChoices}
              value={assigneeValue(task.assignee)}
              onChoose={(value) => {
                void commands.assign(task.id, assigneeOf(value));
              }}
            >
              <ValueButton aria-label="Assignee" isEmpty={task.assignee === null}>
                <AssigneeAvatar assignee={task.assignee} />
                <AssigneeName assignee={task.assignee} />
              </ValueButton>
            </ChoiceMenu>
          </Property>

          <Property label="Priority">
            <ChoiceMenu
              label="Priority"
              choices={priorityChoices}
              value={task.priority}
              onChoose={(priority) => {
                void commands.prioritize(task.id, priority);
              }}
            >
              <ValueButton aria-label="Priority">
                <PriorityFlag priority={task.priority} withLabel />
              </ValueButton>
            </ChoiceMenu>
          </Property>

          <Property label="Estimate">
            <EstimateEditor
              key={`estimate:${task.id}:${String(task.estimateMinutes)}`}
              minutes={task.estimateMinutes}
              onEstimate={(minutes) => commands.estimate(task.id, minutes)}
            />
          </Property>

          <Property label="Start">
            <DateField
              label="Start date"
              value={task.startOn}
              latest={task.dueOn}
              onChange={(startOn) => {
                void commands.reschedule(task.id, startOn, task.dueOn);
              }}
            />
          </Property>

          <Property label="Due">
            <DateField
              label="Due date"
              value={task.dueOn}
              earliest={task.startOn}
              onChange={(dueOn) => {
                void commands.reschedule(task.id, task.startOn, dueOn);
              }}
            />
          </Property>

          <Property label="Sprint">
            <SprintChoice project={project} task={task} parent={parent} />
          </Property>

          <Property label="Points">
            <PointsEditor
              key={`points:${task.id}:${String(task.points)}`}
              look="panel"
              label="Points"
              points={task.points}
              onChange={(points) => sprintCommands.setPoints(task.id, points)}
            />
          </Property>
        </div>

        <Section title="Description">
          <DescriptionEditor
            key={`description:${task.id}:${task.description}`}
            description={task.description}
            onDescribe={(description) => commands.describe(task.id, description)}
          />
        </Section>

        <Section title="Work">
          <TaskWork project={project} task={task} />
        </Section>

        <Section
          title="Waits for"
          action={
            possibleBlockers.length > 0 && (
              <TaskPicker
                label="Add a task to wait for"
                project={project}
                candidates={possibleBlockers}
                onPick={(blockerId) => {
                  void commands.addDependency(task.id, blockerId);
                }}
              />
            )
          }
        >
          {blockers.length === 0 ? (
            <p className={styles.nothing}>Nothing. This task can start at any time.</p>
          ) : (
            <TaskLinks
              project={project}
              tasks={blockers}
              onOpen={navigation.openTask}
              onRemove={(blockerId) => {
                void commands.removeDependency(task.id, blockerId);
              }}
              removeLabel="Stop waiting for this task"
            />
          )}
        </Section>

        <Section title="Subtasks">
          <TaskLinks
            project={project}
            tasks={subtasksOf(tasks, task.id)}
            onOpen={navigation.openTask}
          />
          <TaskComposer
            onCreate={async (title) => {
              const created = await commands.create({ title, parentId: task.id });
              return created !== undefined;
            }}
          />
        </Section>

        <Section title="History">
          <TaskHistory project={project} task={task} tasks={tasks} />
        </Section>
      </div>

      <Dialog
        open={isConfirmingDeletion}
        title={`Delete ${reference}?`}
        onClose={() => {
          setIsConfirmingDeletion(false);
        }}
      >
        <p className={styles.warning}>
          The task and its subtasks will be removed. This cannot be undone.
        </p>
        <div className={styles.dialogActions}>
          <Button
            variant="ghost"
            data-initial-focus
            onClick={() => {
              setIsConfirmingDeletion(false);
            }}
          >
            Cancel
          </Button>
          <Button
            variant="danger"
            onClick={() => {
              void deleteTask();
            }}
          >
            Delete task
          </Button>
        </div>
      </Dialog>
    </aside>
  );
}

/** The value of the sprint menu that stands for the backlog. */
const BACKLOG = 'backlog';

interface SprintChoiceProps {
  project: Project;
  task: Task;
  /** The task this one is a part of, if it is a subtask. */
  parent: Task | undefined;
}

/**
 * The sprint a task is in, chosen from the sprints it can go to and the
 * backlog. A subtask goes where its task goes, so it only says where that is.
 */
function SprintChoice({ project, task, parent }: SprintChoiceProps) {
  const sprints = useSprints(project.id);
  const commands = useSprintCommands(project.id);
  const all = sprints.data ?? [];
  const current = all.find((sprint) => sprint.id === task.sprintId);
  // While the sprints load, a task in a sprint is not called "Backlog".
  const name = current?.name ?? (task.sprintId === null ? 'Backlog' : '');

  if (parent) {
    return (
      <span
        className={styles.fixedValue}
        title={`A subtask goes where ${taskReference(project, parent)} goes`}
      >
        {name}
      </span>
    );
  }

  // A completed sprint takes no tasks, but the one the task is in is named.
  const choices: Choice<string>[] = [
    { value: BACKLOG, label: 'Backlog' },
    ...all
      .filter((sprint) => sprint.state !== 'completed' || sprint.id === task.sprintId)
      .map((sprint) => ({ value: sprint.id, label: sprint.name })),
  ];
  return (
    <ChoiceMenu
      label="Sprint"
      choices={choices}
      value={task.sprintId ?? BACKLOG}
      onChoose={(value) => {
        const sprintId = value === BACKLOG ? null : value;
        if (sprintId !== task.sprintId) {
          void commands.plan(task.id, sprintId);
        }
      }}
    >
      <ValueButton aria-label="Sprint" isEmpty={task.sprintId === null}>
        {name}
        {current?.state === 'active' && (
          <span className={classNames('label', styles.sprintState)}>Active</span>
        )}
      </ValueButton>
    </ChoiceMenu>
  );
}

/** The value of the assignee menu that stands for an assignee. */
function assigneeValue(assignee: Assignee | null): string {
  if (assignee === null) {
    return UNASSIGNED;
  }
  return assignee.kind === 'manager' ? MANAGER : assignee.memberId;
}

/** The assignee a value of the assignee menu stands for. */
function assigneeOf(value: string): Assignee | null {
  if (value === UNASSIGNED) {
    return null;
  }
  return value === MANAGER ? { kind: 'manager' } : { kind: 'member', memberId: value };
}

/** One property of the task: its name on the left, its value on the right. */
function Property({ label, children }: { label: string; children: ReactNode }) {
  return (
    <div className={styles.property}>
      <span className={classNames('label', styles.propertyName)}>{label}</span>
      <div className={styles.propertyValue}>{children}</div>
    </div>
  );
}

interface SectionProps {
  title: string;
  /** A control shown at the end of the heading, such as an add button. */
  action?: ReactNode;
  children: ReactNode;
}

/** A titled part of the panel. */
function Section({ title, action, children }: SectionProps) {
  return (
    <section className={styles.section}>
      <header className={styles.sectionHeader}>
        <h3 className="label">{title}</h3>
        {action}
      </header>
      {children}
    </section>
  );
}

interface TitleEditorProps {
  title: string;
  onRename: (title: string) => Promise<void>;
}

/** The title, edited in place. Enter or leaving the field saves; Escape undoes. */
function TitleEditor({ title, onRename }: TitleEditorProps) {
  const [draft, setDraft] = useState(title);

  function save() {
    const tidyDraft = draft.trim();
    if (tidyDraft === '') {
      setDraft(title);
    } else if (tidyDraft !== title) {
      void onRename(tidyDraft);
    }
  }

  function handleKeyDown(event: KeyboardEvent<HTMLTextAreaElement>) {
    if (event.key === 'Enter') {
      event.preventDefault();
      event.currentTarget.blur();
    } else if (event.key === 'Escape') {
      // Escape undoes the edit. It must not also close the panel.
      event.stopPropagation();
      setDraft(title);
    }
  }

  return (
    <textarea
      className={styles.title}
      value={draft}
      rows={1}
      aria-label="Title"
      onChange={(event) => {
        setDraft(event.target.value);
      }}
      onBlur={save}
      onKeyDown={handleKeyDown}
    />
  );
}

interface DescriptionEditorProps {
  description: string;
  onDescribe: (description: string) => Promise<void>;
}

/** The description. It is saved when the field loses the focus. */
function DescriptionEditor({ description, onDescribe }: DescriptionEditorProps) {
  const [draft, setDraft] = useState(description);

  return (
    <textarea
      className={styles.description}
      value={draft}
      rows={3}
      placeholder="Add details, acceptance criteria, links…"
      aria-label="Description"
      onChange={(event) => {
        setDraft(event.target.value);
      }}
      onBlur={() => {
        if (draft !== description) {
          void onDescribe(draft);
        }
      }}
      onKeyDown={(event) => {
        if (event.key === 'Escape') {
          event.stopPropagation();
          setDraft(description);
        }
      }}
    />
  );
}

interface EstimateEditorProps {
  minutes: number | null;
  onEstimate: (minutes: number | null) => Promise<void>;
}

/** The expected working time, typed as "90m", "2h" or "1h 30m". */
function EstimateEditor({ minutes, onEstimate }: EstimateEditorProps) {
  const stored = minutes === null ? '' : formatEstimate(minutes);
  const [draft, setDraft] = useState(stored);
  const parsed = parseEstimate(draft);
  const isUnderstood = parsed !== undefined;

  return (
    <input
      className={classNames(styles.estimate, !isUnderstood && styles.notUnderstood)}
      value={draft}
      placeholder="None"
      aria-label="Estimate"
      aria-invalid={!isUnderstood}
      title={
        isUnderstood ? 'Write it like 90m, 2h or 1h 30m' : 'Not understood. Try 90m, 2h or 1h 30m'
      }
      onChange={(event) => {
        setDraft(event.target.value);
      }}
      onBlur={() => {
        if (parsed !== undefined && parsed !== minutes) {
          void onEstimate(parsed);
        }
      }}
      onKeyDown={(event) => {
        if (event.key === 'Enter') {
          event.currentTarget.blur();
        } else if (event.key === 'Escape') {
          event.stopPropagation();
          setDraft(stored);
        }
      }}
    />
  );
}

interface TaskLinksProps {
  project: Project;
  tasks: Task[];
  onOpen: (taskId: Id) => void;
  /** If given, each task gets a button that calls this with its identifier. */
  onRemove?: (taskId: Id) => void;
  removeLabel?: string;
}

/** A short list of tasks that can be opened, each with the mark of its status. */
function TaskLinks({ project, tasks, onOpen, onRemove, removeLabel = 'Remove' }: TaskLinksProps) {
  if (tasks.length === 0) {
    return null;
  }
  return (
    <ul className={styles.links}>
      {tasks.map((linked) => {
        const status = statusOf(project, linked);
        return (
          <li key={linked.id} className={styles.link}>
            <button
              type="button"
              className={styles.linkTarget}
              title={status?.name}
              onClick={() => {
                onOpen(linked.id);
              }}
            >
              {status && <StatusMark status={status} />}
              <span className={styles.linkReference}>{taskReference(project, linked)}</span>
              <span className={styles.linkTitle}>{linked.title}</span>
            </button>
            {onRemove && (
              <IconButton
                label={removeLabel}
                className={styles.linkRemove}
                onClick={() => {
                  onRemove(linked.id);
                }}
              >
                <X />
              </IconButton>
            )}
          </li>
        );
      })}
    </ul>
  );
}
