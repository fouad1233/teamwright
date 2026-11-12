import { ChevronDown, History } from 'lucide-react';
import { useState } from 'react';

import type { Assignee, Project, Sprint, Task } from '@/shared/api/types';
import { classNames } from '@/shared/lib/classNames';
import { today } from '@/shared/lib/format';
import { Button } from '@/shared/ui/Button';
import { ChoiceMenu } from '@/shared/ui/Menu';

import { BoardView } from '../board/BoardView';
import styles from './SprintBoard.module.css';
import {
  daysLeft,
  formatDaysLeft,
  formatSprintDates,
  formatSummary,
  layoutOf,
  previousSprintOf,
  type BoardScope,
} from './sprintModel';
import { useSprintCommands, useSprints } from './sprintQueries';

interface SprintBoardProps {
  project: Project;
  tasks: Task[];
  /**
   * Which tasks get a card, for a board with a filter of its own. It
   * applies together with the sprint that is shown. The board still knows
   * every task, so that a card dropped among the shown ones lands where it
   * was dropped and a card knows what it waits for. Left out, every task
   * is shown.
   */
  shows?: (task: Task) => boolean;
  /** Who a task opened from a lane is given to. Left out, nobody. */
  assigneeOfNewTasks?: Assignee;
}

/**
 * The board of a project. While a sprint is active it shows the tasks of
 * that sprint, and a switch in its header shows all tasks again. A menu in
 * the header chooses a sprint that is over, and the board then shows the
 * tasks it finished. Without any sprint it is the board of all tasks.
 */
export function SprintBoard({ project, tasks, shows, assigneeOfNewTasks }: SprintBoardProps) {
  const sprints = useSprints(project.id);
  const commands = useSprintCommands(project.id);
  const [scope, setScope] = useState<BoardScope>({ kind: 'current' });
  const { active, completed } = layoutOf(sprints.data ?? []);
  const previous = previousSprintOf(scope, completed);

  if (!active && completed.length === 0) {
    return (
      <BoardView
        project={project}
        tasks={tasks}
        shows={shows}
        assigneeOfNewTasks={assigneeOfNewTasks}
      />
    );
  }

  // A completed sprint takes no tasks, so a task made while one is shown
  // is not planned into it. The sprint that runs takes the tasks made on its board.
  const sprintShown = previous ?? (scope.kind === 'current' ? active : undefined);
  const isRunningShown = sprintShown !== undefined && sprintShown === active;
  const backLabel = active ? `Back to ${active.name}` : 'Back to all tasks';

  return (
    <div className={styles.frame}>
      <header className={styles.header}>
        <span className={styles.sprint}>
          {previous ? (
            <PreviousSprintFacts sprint={previous} />
          ) : active ? (
            <ActiveSprintFacts sprint={active} />
          ) : (
            <>
              <span className={styles.name}>All tasks</span>
              <span className={styles.facts}>No sprint is running</span>
            </>
          )}
        </span>
        <span className={styles.space} />
        {previous && (
          <Button
            onClick={() => {
              setScope({ kind: 'current' });
            }}
          >
            {backLabel}
          </Button>
        )}
        {!previous && active && (
          <div className={styles.switch} role="group" aria-label="Tasks on the board">
            {(['current', 'all'] as const).map((choice) => (
              <button
                key={choice}
                type="button"
                className={classNames(styles.choice, scope.kind === choice && styles.chosen)}
                aria-pressed={scope.kind === choice}
                onClick={() => {
                  setScope({ kind: choice });
                }}
              >
                {choice === 'current' ? 'Sprint' : 'All tasks'}
              </button>
            ))}
          </div>
        )}
        {completed.length > 0 && (
          <ChoiceMenu
            label="Previous sprints"
            choices={completed.map((sprint) => ({
              value: sprint.id,
              label: `${sprint.name} · ${formatSprintDates(sprint)}`,
            }))}
            value={previous?.id ?? ''}
            onChoose={(sprintId) => {
              setScope({ kind: 'previous', sprintId });
            }}
          >
            <Button icon={<History />}>
              Previous sprints
              <ChevronDown />
            </Button>
          </ChoiceMenu>
        )}
      </header>
      <div className={styles.board}>
        <BoardView
          project={project}
          tasks={tasks}
          assigneeOfNewTasks={assigneeOfNewTasks}
          shows={
            sprintShown
              ? (task) => task.sprintId === sprintShown.id && (shows?.(task) ?? true)
              : shows
          }
          onCreated={
            isRunningShown && active
              ? (task) => {
                  void commands.plan(task.id, active.id);
                }
              : undefined
          }
        />
      </div>
    </div>
  );
}

/** Which sprint runs, its dates and how long it still runs. */
function ActiveSprintFacts({ sprint }: { sprint: Sprint }) {
  return (
    <>
      <span className={styles.name}>{sprint.name}</span>
      <span className={styles.facts}>
        {formatSprintDates(sprint)}
        {sprint.endOn !== null && (
          <>
            <span className={styles.separator} aria-hidden />
            {formatDaysLeft(daysLeft(sprint.endOn, today()))}
          </>
        )}
      </span>
    </>
  );
}

/** A sprint that is over: that it is, its dates and what it finished. */
function PreviousSprintFacts({ sprint }: { sprint: Sprint }) {
  return (
    <>
      <span className={styles.name}>{sprint.name}</span>
      <span className="label">Completed</span>
      <span className={styles.facts}>
        {formatSprintDates(sprint)}
        {sprint.summary && (
          <>
            <span className={styles.separator} aria-hidden />
            {formatSummary(sprint.summary)}
          </>
        )}
      </span>
    </>
  );
}
