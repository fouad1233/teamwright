import { useState } from 'react';

import type { Project, Task } from '@/shared/api/types';
import { classNames } from '@/shared/lib/classNames';
import { today } from '@/shared/lib/format';

import { BoardView } from '../board/BoardView';
import styles from './SprintBoard.module.css';
import { daysLeft, formatDaysLeft, formatSprintDates, layoutOf } from './sprintModel';
import { useSprintCommands, useSprints } from './sprintQueries';

/** What the board shows while a sprint runs: the tasks of the sprint, or all tasks. */
type Scope = 'sprint' | 'all';

interface SprintBoardProps {
  project: Project;
  tasks: Task[];
}

/**
 * The board of a project. While a sprint is active it shows the tasks of
 * that sprint, and a switch in its header shows all tasks again. Without an
 * active sprint it is the board of all tasks.
 */
export function SprintBoard({ project, tasks }: SprintBoardProps) {
  const sprints = useSprints(project.id);
  const commands = useSprintCommands(project.id);
  const [scope, setScope] = useState<Scope>('sprint');
  const active = layoutOf(sprints.data ?? []).active;

  if (!active) {
    return <BoardView project={project} tasks={tasks} />;
  }

  const isSprintShown = scope === 'sprint';
  return (
    <div className={styles.frame}>
      <header className={styles.header}>
        <span className={styles.sprint}>
          <span className={styles.name}>{active.name}</span>
          <span className={styles.facts}>
            {formatSprintDates(active)}
            {active.endOn !== null && (
              <>
                <span className={styles.separator} aria-hidden />
                {formatDaysLeft(daysLeft(active.endOn, today()))}
              </>
            )}
          </span>
        </span>
        <span className={styles.space} />
        <div className={styles.switch} role="group" aria-label="Tasks on the board">
          {(['sprint', 'all'] as const).map((choice) => (
            <button
              key={choice}
              type="button"
              className={classNames(styles.choice, scope === choice && styles.chosen)}
              aria-pressed={scope === choice}
              onClick={() => {
                setScope(choice);
              }}
            >
              {choice === 'sprint' ? 'Sprint' : 'All tasks'}
            </button>
          ))}
        </div>
      </header>
      <div className={styles.board}>
        <BoardView
          project={project}
          tasks={tasks}
          shows={isSprintShown ? (task) => task.sprintId === active.id : undefined}
          // A task opened on the board of the sprint is meant for the sprint.
          onCreated={
            isSprintShown
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
