import type { Area, Project, Task } from '@/shared/api/types';
import { taskReference } from '@/shared/lib/format';
import { ValueButton } from '@/shared/ui/Button';
import { ChoiceMenu } from '@/shared/ui/Menu';

import styles from './AreaChoice.module.css';
import { useAreaCommands } from './areaQueries';

interface AreaChoiceProps {
  project: Project;
  /** The areas of the project, in their order. */
  areas: readonly Area[];
  task: Task;
  /** The task this one is a part of, if it is a subtask. */
  parent: Task | undefined;
}

/**
 * The area a task is in, for the panel of the task. Choosing another moves
 * the task there with its subtasks. A subtask goes where its task goes, so
 * it only says where that is.
 */
export function AreaChoice({ project, areas, task, parent }: AreaChoiceProps) {
  const commands = useAreaCommands(project.id);
  const current = areas.find((area) => area.id === task.areaId);

  if (parent) {
    return (
      <span
        className={styles.fixed}
        title={`A subtask is in the area of ${taskReference(project, parent)}`}
      >
        {current?.name}
      </span>
    );
  }

  return (
    <ChoiceMenu
      label="Area"
      choices={areas.map((area) => ({ value: area.id, label: area.name }))}
      value={task.areaId}
      onChoose={(areaId) => {
        if (areaId !== task.areaId) {
          void commands.moveTask(task.id, areaId);
        }
      }}
    >
      <ValueButton aria-label="Area">{current?.name}</ValueButton>
    </ChoiceMenu>
  );
}
