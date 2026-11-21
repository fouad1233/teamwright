import type { Project, Task } from '@/shared/api/types';
import { taskReference } from '@/shared/lib/format';

import { BudgetLine } from './BudgetLine';
import styles from './Coordinator.module.css';
import { budgetOf } from './coordinatorModel';
import { useCoordinator, useCoordinatorCommands } from './coordinatorQueries';

interface TaskBudgetProps {
  project: Project;
  task: Task;
}

/**
 * The budget of a task in its panel, against what its conversations spent.
 * A budget that is used up does not stop a press of the manager's: it is
 * shown here in the color of danger, and the manager decides
 * (decision 0037).
 */
export function TaskBudget({ project, task }: TaskBudgetProps) {
  const { data: coordinator } = useCoordinator(project.id);
  const commands = useCoordinatorCommands(project.id);
  const budget = budgetOf(coordinator, 'task', task.id);
  const isUsedUp = budget !== undefined && budget.used >= budget.tokens;
  return (
    <>
      <BudgetLine
        coordinator={coordinator}
        scope={{ scope: 'task', targetId: task.id }}
        label={taskReference(project, task)}
        onSet={commands.setBudget}
      />
      {isUsedUp && (
        <p className={styles.problem}>
          The budget is used up. The coordinator starts nothing more on it; what you start by hand
          still runs.
        </p>
      )}
    </>
  );
}
