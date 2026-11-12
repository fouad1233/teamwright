import { useNavigation } from '@/app/navigation';
import type { Conversation, Project } from '@/shared/api/types';
import { classNames } from '@/shared/lib/classNames';
import { formatMoment, taskReference } from '@/shared/lib/format';
import { Button } from '@/shared/ui/Button';
import { describeFailure } from '@/shared/ui/Toast';

import { advisorWaitsFor } from '../advisor/advisorModel';
import { useTasks } from '../tasks/taskQueries';
import { MemberAvatar } from '../team/Assignee';
import { useTeam } from '../team/teamQueries';
import { DecisionCard } from '../work/DecisionCard';
import { appearsAt, waitsForManager } from '../work/waitsForManager';
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
 * specialists asked instead of guessing, and what the Advisor asks or
 * proposes, the one that has waited longest first. A question of a
 * specialist can be answered where it stands. What the Advisor wants leads
 * to the Advisor, where there is room for it.
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
            options with what each leads to. The questions and the proposals of the Advisor wait
            here too.
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
                  {conversation.isWithAdvisor ? (
                    <FromAdvisor project={project} conversation={conversation} />
                  ) : (
                    <Question project={project} conversation={conversation} />
                  )}
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

/**
 * What the Advisor of a project waits for: answers to its questions, or a
 * word about its proposal. Both need more room than the inbox has, so this
 * says what it is and leads there.
 */
function FromAdvisor({ project, conversation }: QuestionProps) {
  const navigation = useNavigation();
  const waitsFor = advisorWaitsFor(conversation);

  if (waitsFor === null) {
    return null;
  }

  return (
    <article className={styles.item}>
      <header className={styles.about}>
        <span className={styles.project}>{project.name}</span>
        <span className={styles.taskTitle}>Advisor</span>
        <span className={classNames(styles.when)}>{formatMoment(askedAt(conversation))}</span>
      </header>
      <div className={classNames(waitsForManager.frame, styles.advisor)}>
        <p className={classNames(waitsForManager.part, styles.wants)}>The Advisor {waitsFor}.</p>
        <Button
          className={waitsForManager.part}
          style={appearsAt(1)}
          variant="secondary"
          onClick={() => {
            navigation.openAdvisorOf(project.id);
          }}
        >
          Open
        </Button>
      </div>
    </article>
  );
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
