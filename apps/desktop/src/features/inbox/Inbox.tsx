import { useNavigation } from '@/app/navigation';
import type { Conversation, PermissionRequest, Project } from '@/shared/api/types';
import { classNames } from '@/shared/lib/classNames';
import { formatMoment, taskReference } from '@/shared/lib/format';
import { Button } from '@/shared/ui/Button';
import { describeFailure } from '@/shared/ui/Toast';

import { advisorWaitsFor } from '../advisor/advisorModel';
import { BudgetQuestions } from '../coordinator/BudgetQuestions';
import { useBudgetQuestions } from '../coordinator/coordinatorQueries';
import { AskToNotify } from '../notifications/AskToNotify';
import { useCommandsSwitchedOn } from '../permissions/commandQueries';
import { PermissionCard } from '../permissions/PermissionCard';
import { useOpenPermissionRequests, usePermissionCommands } from '../permissions/permissionQueries';
import { useComputerOf } from '../sync/RunsElsewhere';
import { useTasks } from '../tasks/taskQueries';
import { MemberAvatar } from '../team/Assignee';
import { useTeam } from '../team/teamQueries';
import { DecisionCard } from '../work/DecisionCard';
import { appearsAt, waitsForManager } from '../work/waitsForManager';
import { askedAt, openDecision } from '../work/workModel';
import { useConversationsWaiting, useWorkCommands } from '../work/workQueries';
import styles from './Inbox.module.css';
import { inOrderOfAsking, waitingInWords } from './inboxModel';

interface InboxProps {
  /** The projects that are not put away, to name the project of a question. */
  projects: readonly Project[];
}

/**
 * What waits for the manager, in every project: the questions that
 * specialists asked instead of guessing, their requests for a permission,
 * and what the Advisor asks or proposes, the one that has waited longest
 * first. A question or a request of a specialist can be answered where it
 * stands. What the Advisor wants leads to the Advisor, where there is room
 * for it.
 */
export function Inbox({ projects }: InboxProps) {
  const waiting = useConversationsWaiting();
  const permissions = useOpenPermissionRequests();
  const conversations = waiting.data ?? [];
  // The questions of the coordinator about used-up budgets come first.
  const budgetQuestions = useBudgetQuestions().data ?? [];
  const items = inOrderOfAsking(conversations, permissions.data ?? [], askedAt);

  return (
    <section className={styles.inbox} aria-label="Inbox">
      <header className={styles.header}>
        <h1 className={styles.title}>Inbox</h1>
        {waiting.data && (
          <p className={styles.count}>{waitingInWords(items.length + budgetQuestions.length)}</p>
        )}
      </header>

      {waiting.isError && (
        <p className={styles.note} role="alert">
          The inbox could not be loaded. {describeFailure(waiting.error)}
        </p>
      )}
      {waiting.isPending && <p className={styles.note}>Loading…</p>}

      {budgetQuestions.length > 0 && <BudgetQuestions questions={budgetQuestions} />}

      {waiting.data && items.length === 0 && budgetQuestions.length === 0 && (
        <div className={classNames(styles.empty, 'pattern-band')}>
          <p className="label">Nothing to decide</p>
          <p className={styles.emptyText}>
            A specialist that meets a choice that is yours to make does not guess. It stops and
            asks, and its question appears here: what it needs to know, why it matters, and the
            options with what each leads to. A specialist that needs to run a command or fetch from
            the web asks here too, and you allow or refuse exactly that. The questions and the
            proposals of the Advisor wait here as well.
          </p>
        </div>
      )}

      {items.length > 0 && <AskToNotify />}

      {items.length > 0 && (
        <ol className={styles.items}>
          {items.map((item) => {
            if (item.kind === 'permission') {
              const project = projects.find((known) => known.id === item.request.projectId);
              return (
                project && (
                  <li key={item.request.id}>
                    <Permission project={project} request={item.request} />
                  </li>
                )
              );
            }
            const conversation = item.conversation;
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
  const computer = useComputerOf(conversation);
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
        <p className={classNames(waitsForManager.part, styles.wants)}>
          The Advisor {waitsFor}
          {conversation.isHere ? '.' : `, on ${computer}.`}
        </p>
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
  const computer = useComputerOf(conversation);
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
        answeredOn={conversation.isHere ? undefined : computer}
      />
    </article>
  );
}

interface PermissionProps {
  project: Project;
  request: PermissionRequest;
}

/** A request for a permission: where it comes from, who asks, and exactly what and why. */
function Permission({ project, request }: PermissionProps) {
  const navigation = useNavigation();
  const tasks = useTasks(project.id);
  const team = useTeam(project.id);
  const commands = usePermissionCommands(project.id);
  const commandsSwitchedOn = useCommandsSwitchedOn();
  const task = tasks.data?.find((known) => known.id === request.taskId);
  const member = team.data?.find((known) => known.id === request.memberId);

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
        <span className={classNames(styles.when)}>{formatMoment(request.askedAt)}</span>
      </header>
      <PermissionCard
        request={request}
        askedBy={member?.name ?? 'A specialist'}
        commandsSwitchedOn={commandsSwitchedOn}
        onAnswer={(answer) => commands.answer(request.id, answer)}
      />
    </article>
  );
}
