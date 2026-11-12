import { useState } from 'react';

import type { Member, Project } from '@/shared/api/types';
import { Button } from '@/shared/ui/Button';
import { SidePanel } from '@/shared/ui/SidePanel';

import { MemberAvatar } from '../team/Assignee';
import { ConversationView, ReplyBox } from './ConversationView';
import styles from './MemberChat.module.css';
import { useConversationsOfProject } from './WorkContext';
import { chatsWith, runningTurn, whyProgramIsNotReady } from './workModel';
import { useAgentProgram, useWorkCommands } from './workQueries';

interface MemberChatProps {
  project: Project;
  /** The AI specialist the manager talks to. */
  member: Member;
  onClose: () => void;
}

/**
 * The conversation of the manager with one specialist that is not about a
 * task: a question, and what the specialist answers from what it finds in
 * the folder of the project.
 *
 * The panel shows the latest such conversation. A new one starts with a
 * fresh session, in which the specialist remembers nothing of the earlier
 * ones.
 */
export function MemberChat({ project, member, onClose }: MemberChatProps) {
  const commands = useWorkCommands(project.id);
  const program = useAgentProgram();
  const latest = chatsWith(useConversationsOfProject(), member.id).at(-1);
  const [isStartingOver, setIsStartingOver] = useState(false);
  const obstacle = program.data ? whyProgramIsNotReady(program.data) : null;
  const shown = isStartingOver ? undefined : latest;

  async function ask(message: string): Promise<boolean> {
    const conversation = await commands.ask(member.id, message);
    if (conversation) {
      setIsStartingOver(false);
    }
    return conversation !== undefined;
  }

  return (
    <SidePanel
      label={`Conversation with ${member.name}`}
      header={
        <>
          <MemberAvatar member={member} />
          <span className={styles.name}>{member.name}</span>
        </>
      }
      onClose={onClose}
    >
      {member.responsibility !== '' && (
        <p className={styles.responsibility}>{member.responsibility}</p>
      )}

      {shown ? (
        <>
          <ConversationView project={project} conversation={shown} hidesMember />
          {!runningTurn(shown) && (
            <Button
              variant="ghost"
              className={styles.startOver}
              onClick={() => {
                setIsStartingOver(true);
              }}
            >
              Start a new conversation
            </Button>
          )}
        </>
      ) : (
        <>
          <p className={styles.hint}>
            {obstacle ??
              `Ask ${member.name} something that is not about one task. The answer comes from what is in the folder of the project. For work on a task, open the task and start it there.`}
          </p>
          {obstacle === null && (
            <ReplyBox
              label={`Question for ${member.name}`}
              placeholder={`Ask ${member.name} something…`}
              onSend={ask}
            />
          )}
          {isStartingOver && (
            <Button
              variant="ghost"
              className={styles.startOver}
              onClick={() => {
                setIsStartingOver(false);
              }}
            >
              Back to the last conversation
            </Button>
          )}
        </>
      )}
    </SidePanel>
  );
}
