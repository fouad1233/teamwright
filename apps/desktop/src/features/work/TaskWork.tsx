import { Play } from 'lucide-react';
import { useState } from 'react';

import type { Project, Task } from '@/shared/api/types';
import { Button } from '@/shared/ui/Button';

import { useTeamMember } from '../team/TeamContext';
import { ConversationView } from './ConversationView';
import styles from './TaskWork.module.css';
import { useConversationOfTask } from './WorkContext';
import { whyProgramIsNotReady } from './workModel';
import { useAgentProgram, useWorkCommands } from './workQueries';

interface TaskWorkProps {
  project: Project;
  task: Task;
}

/**
 * The work of an AI specialist on a task: the way to start it, what the
 * specialist is doing while it works, its report, and a line to answer it.
 */
export function TaskWork({ project, task }: TaskWorkProps) {
  const member = useTeamMember(
    task.assignee?.kind === 'member' ? task.assignee.memberId : undefined,
  );
  const conversation = useConversationOfTask(task.id);
  const program = useAgentProgram();
  const commands = useWorkCommands(project.id);
  const [isStarting, setIsStarting] = useState(false);

  if (conversation) {
    return <ConversationView project={project} conversation={conversation} />;
  }

  if (member?.kind !== 'agent') {
    return (
      <p className={styles.nothing}>
        Give the task to an AI specialist, and the specialist can work on it.
      </p>
    );
  }

  const obstacle = program.data ? whyProgramIsNotReady(program.data) : null;
  // Asking the program whether it is there takes a few seconds. The button
  // waits for the answer, and the line next to it says what it waits for.
  const hint = program.isPending
    ? 'Looking for Claude Code on this computer…'
    : (obstacle ??
      `${member.name} gets a brief of this task and works in the folder of the project.`);

  async function start() {
    setIsStarting(true);
    await commands.start(task.id);
    setIsStarting(false);
  }

  return (
    <div className={styles.start}>
      <Button
        variant="secondary"
        icon={<Play />}
        disabled={isStarting || program.isPending || obstacle !== null}
        onClick={() => {
          void start();
        }}
      >
        Start work
      </Button>
      <p className={styles.hint}>{hint}</p>
    </div>
  );
}
