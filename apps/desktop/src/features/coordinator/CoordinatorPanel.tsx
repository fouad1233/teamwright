import { Minus, Plus } from 'lucide-react';
import { useQueryClient } from '@tanstack/react-query';

import { useBackend } from '@/shared/api/BackendProvider';
import type { Member, Project, Task } from '@/shared/api/types';
import { formatMoment, taskReference } from '@/shared/lib/format';
import { Button, IconButton } from '@/shared/ui/Button';
import { Switch } from '@/shared/ui/Switch';

import { FULL_ACCESS_FOR_THE_COORDINATOR } from '../projects/placesModel';
import { useProjectPlaces } from '../projects/projectQueries';
import { tasksKey } from '../tasks/taskQueries';
import { BudgetLine } from './BudgetLine';
import styles from './Coordinator.module.css';
import { WHAT_A_BUDGET_COUNTS, WHAT_IT_DOES, runsInWords } from './coordinatorModel';
import { useCoordinator, useCoordinatorCommands } from './coordinatorQueries';

interface CoordinatorPanelProps {
  project: Project;
  tasks: readonly Task[];
  members: readonly Member[];
}

/**
 * The coordinator of a project, in the Team place (decision 0037): the
 * switch, how many specialists may work at once, the budgets of the project
 * and of each specialist against what they spent, what waits and why, and
 * what the coordinator did, newest first.
 */
export function CoordinatorPanel({ project, tasks, members }: CoordinatorPanelProps) {
  const { data: coordinator } = useCoordinator(project.id);
  const commands = useCoordinatorCommands(project.id);
  const fullAccess = useProjectPlaces(project.id).data?.fullAccess === true;
  const backend = useBackend();
  const queryClient = useQueryClient();
  const specialists = members.filter((member) => member.kind === 'agent');
  const nameOfTask = (taskId: string | null) => {
    const task = tasks.find((candidate) => candidate.id === taskId);
    return task ? `${taskReference(project, task)} ${task.title}` : 'A turn for messages';
  };

  async function accept(taskId: string) {
    const done = project.statuses.find((status) => status.category === 'done');
    if (done) {
      await backend.moveTask(taskId, done.id, 0);
      await queryClient.invalidateQueries({ queryKey: tasksKey(project.id) });
    }
  }

  return (
    <section className={styles.section} aria-labelledby="the-coordinator">
      <header className={styles.header}>
        <h2 id="the-coordinator" className="label">
          The coordinator
        </h2>
      </header>

      <Switch
        on={coordinator?.runs === 'here'}
        disabled={!coordinator}
        label="Start ready work by itself"
        hint={coordinator ? `${WHAT_IT_DOES} ${runsInWords(coordinator)}` : WHAT_IT_DOES}
        onChange={(on) => {
          void commands.switchTo(on);
        }}
      />
      {fullAccess && <p className={styles.note}>{FULL_ACCESS_FOR_THE_COORDINATOR}</p>}

      {coordinator && (
        <div className={styles.limit}>
          <span className={styles.limitWords}>Specialists at work at once</span>
          <IconButton
            label="One fewer"
            disabled={coordinator.limit <= coordinator.leastLimit}
            onClick={() => {
              void commands.setLimit(coordinator.limit - 1);
            }}
          >
            <Minus />
          </IconButton>
          <span className={styles.limitValue}>{coordinator.limit}</span>
          <IconButton
            label="One more"
            disabled={coordinator.limit >= coordinator.mostLimit}
            onClick={() => {
              void commands.setLimit(coordinator.limit + 1);
            }}
          >
            <Plus />
          </IconButton>
        </div>
      )}

      <div className={styles.budgets}>
        <h3 className="label">Budgets in tokens</h3>
        <p className={styles.note}>{WHAT_A_BUDGET_COUNTS}</p>
        <BudgetLine
          coordinator={coordinator}
          scope={{ scope: 'project', targetId: project.id }}
          label="The project"
          onSet={commands.setBudget}
        />
        {specialists.map((member) => (
          <BudgetLine
            key={member.id}
            coordinator={coordinator}
            scope={{ scope: 'member', targetId: member.id }}
            label={member.name}
            onSet={commands.setBudget}
          />
        ))}
      </div>

      {coordinator && coordinator.waiting.length > 0 && (
        <div className={styles.block}>
          <h3 className="label">Waiting</h3>
          <ul className={styles.list}>
            {coordinator.waiting.map((waiting) => (
              <li
                key={`${waiting.taskId ?? 'messages'}-${waiting.memberId}`}
                className={styles.item}
              >
                <span className={styles.itemWords}>
                  <span className={styles.subject}>{nameOfTask(waiting.taskId)}</span>:{' '}
                  {waiting.words}.
                </span>
                {waiting.checkTaskId !== null && (
                  <Button
                    variant="secondary"
                    onClick={() => {
                      if (waiting.checkTaskId !== null) {
                        void accept(waiting.checkTaskId);
                      }
                    }}
                  >
                    Accept {nameOfTask(waiting.checkTaskId).split(' ')[0]}
                  </Button>
                )}
              </li>
            ))}
          </ul>
        </div>
      )}

      <div className={styles.block}>
        <h3 className="label">What it did</h3>
        {!coordinator || coordinator.entries.length === 0 ? (
          <p className={styles.note}>Nothing yet. What it starts, and why, is written down here.</p>
        ) : (
          <ol className={styles.list}>
            {coordinator.entries.map((entry) => (
              <li key={entry.id} className={styles.entry}>
                <span className={styles.moment}>{formatMoment(entry.at)}</span>
                <span className={styles.entryText} data-selectable>
                  {entry.text}
                </span>
              </li>
            ))}
          </ol>
        )}
      </div>
    </section>
  );
}
