import { useNavigation } from '@/app/navigation';
import type { Conversation, Project } from '@/shared/api/types';
import { classNames } from '@/shared/lib/classNames';
import { formatMoment, taskReference } from '@/shared/lib/format';
import { describeFailure } from '@/shared/ui/Toast';

import { useTasks } from '../tasks/taskQueries';
import { MemberAvatar } from '../team/Assignee';
import { useTeam } from '../team/teamQueries';
import { DecisionCard } from '../work/DecisionCard';
import { askedAt, openDecision } from '../work/workModel';
import { useConversationsWaiting, useWorkCommands } from '../work/workQueries';
import styles from './Inbox.module.css';
import { waitingInWords } from './inboxModel';

interface InboxProps {
  /** The projects that are not put away, to name the project of a question. */
  projects: readonly Project[];
}

/**
 * What waits for the manager, in every project: the questions that
 * specialists asked instead of guessing, the one that has waited longest
 * first. Each can be answered where it stands.
 */
export function Inbox({ projects }: InboxProps) {
  const waiting = useConversationsWaiting();
  const conversations = waiting.data ?? [];

  return (
    <section className={styles.inbox} aria-label="Inbox">
      <header className={styles.header}>
        <h1 className={styles.title}>Inbox</h1>
        {waiting.data && <p className={styles.count}>{waitingInWords(conversations.length)}</p>}
      </header>

      {waiting.isError && (
        <p className={styles.note} role="alert">
          The inbox could not be loaded. {describeFailure(waiting.error)}
        </p>
      )}
      {waiting.isPending && <p className={styles.note}>Loading…</p>}

      {waiting.data && conversations.length === 0 && (
        <div className={styles.empty}>
          <p className="label">Nothing to decide</p>
          <p className={styles.emptyText}>
            A specialist that meets a choice that is yours to make does not guess. It stops and
            asks, and its question appears here: what it needs to know, why it matters, and the
            options with what each leads to.
          </p>
        </div>
      )}

      {conversations.length > 0 && (
        <ol className={styles.items}>
          {conversations.map((conversation) => {
            const project = projects.find((known) => known.id === conversation.projectId);
            return (
              project && (
                <li key={conversation.id}>
                  <Question project={project} conversation={conversation} />
                </li>
              )
            );
          })}
        </ol>
      )}
    </section>
  );
}

interface QuestionProps {
  project: Project;
  conversation: Conversation;
}

/** One question: where it comes from, who asks, and the question itself with its options. */
function Question({ project, conversation }: QuestionProps) {
  const navigation = useNavigation();
  const tasks = useTasks(project.id);
  const team = useTeam(project.id);
  const commands = useWorkCommands(project.id);
  const decision = openDecision(conversation);
  const task = tasks.data?.find((known) => known.id === conversation.taskId);
  const member = team.data?.find((known) => known.id === conversation.memberId);

  if (!decision) {
    return null;
  }

  return (
    <article className={styles.item}>
      <header className={styles.about}>
        {member && <MemberAvatar member={member} />}
        <span className={styles.project}>{project.name}</span>
        {task ? (
          <button
            type="button"
            className={styles.task}
            onClick={() => {
              navigation.openTaskOf(project.id, task.id);
            }}
          >
            <span className={styles.reference}>{taskReference(project, task)}</span>
            <span className={styles.taskTitle}>{task.title}</span>
          </button>
        ) : (
          <span className={styles.taskTitle}>A question of yours</span>
        )}
        <span className={classNames(styles.when)}>{formatMoment(askedAt(conversation))}</span>
      </header>
      <DecisionCard
        decision={decision}
        askedBy={member?.name ?? 'A specialist'}
        onDecide={(answer) => commands.decide(conversation.id, answer)}
      />
    </article>
  );
}
