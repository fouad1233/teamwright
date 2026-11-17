import { Send, Square } from 'lucide-react';
import { useLayoutEffect, useRef, useState, type KeyboardEvent } from 'react';

import { useNavigation } from '@/app/navigation';
import type { Project, Task, Turn } from '@/shared/api/types';
import { classNames } from '@/shared/lib/classNames';
import { taskReference } from '@/shared/lib/format';
import { Button } from '@/shared/ui/Button';
import { ChoiceInput, TextArea } from '@/shared/ui/Field';
import { ChoiceMenu } from '@/shared/ui/Menu';

import { MemberAvatar } from '../team/Assignee';
import { useTeamMember, useTeamMembers } from '../team/TeamContext';
import type { KindProps } from '../views/viewKinds';
import { ContextMeter } from '../work/ContextMeter';
import { TurnView, WorkingMark } from '../work/ConversationView';
import { DecisionCard } from '../work/DecisionCard';
import { useConversationsOfProject } from '../work/WorkContext';
import { openDecision, whyProgramIsNotReady } from '../work/workModel';
import { useAgentProgram, useWorkCommands } from '../work/workQueries';
import styles from './ChatView.module.css';
import {
  ADVISOR_NAME,
  addressees,
  canSend,
  firstRecipient,
  howToSend,
  runsOfTalk,
  sendingInWords,
  type Addressee,
  type Recipient,
  type Run,
} from './chatModel';

/** How close to its end the talk must be read for a new turn to scroll it, in pixels. */
const NEAR_THE_END = 160;

/**
 * The chat: the manager's conversations with the team that are not work on
 * a task, as one talk in the order of time, and at its foot the field to
 * write to one of them. It keeps nothing of its own: everything it shows
 * is the conversations of the project.
 */
export function ChatView({ project, everyTask }: KindProps) {
  const conversations = useConversationsOfProject();
  const team = useTeamMembers();
  const runs = runsOfTalk(conversations);
  const offered = addressees(team, conversations);
  const talk = useRef<HTMLDivElement>(null);
  const wasNearTheEnd = useRef(true);

  // Everything that makes the talk longer: a turn, a step, an answer.
  const length = runs
    .map((run) => run.turns.map((turn) => `${turn.steps.length.toString()}${turn.state}`).join(','))
    .join(';');

  // The talk opens at its end, and follows it while it is read there.
  useLayoutEffect(() => {
    const element = talk.current;
    if (element && wasNearTheEnd.current) {
      element.scrollTop = element.scrollHeight;
    }
  }, [length]);

  function rememberWhereRead() {
    const element = talk.current;
    if (element) {
      wasNearTheEnd.current =
        element.scrollHeight - element.scrollTop - element.clientHeight < NEAR_THE_END;
    }
  }

  return (
    <div className={styles.view}>
      <div className={styles.talk} ref={talk} onScroll={rememberWhereRead}>
        <div className={styles.column}>
          {runs.length === 0 ? (
            <div className={classNames(styles.empty, 'pattern-band')}>
              <h2 className={styles.emptyTitle}>Talk with the team</h2>
              <p className={styles.emptyText}>
                Ask the specialists, and the Advisor once a plan is started, what is not about one
                task, and read every answer here as one talk. The work on a task is talked about in
                the panel of that task.
              </p>
            </div>
          ) : (
            <ol className={styles.runs}>
              {runs.map((run) => (
                <li key={run.key}>
                  <RunView project={project} run={run} />
                </li>
              ))}
            </ol>
          )}
        </div>
      </div>

      <Composer
        project={project}
        tasks={everyTask}
        offered={offered}
        initial={firstRecipient(runs, offered)}
        onSent={() => {
          wasNearTheEnd.current = true;
        }}
      />
    </div>
  );
}

interface RunViewProps {
  project: Project;
  run: Run;
}

/**
 * Turns that follow each other in one conversation, under a quiet rule
 * with the name of whom they are with. What can be done about the
 * conversation stands at the end of the rule: stopping it while it runs.
 */
function RunView({ project, run }: RunViewProps) {
  const { conversation } = run;
  const member = useTeamMember(conversation.memberId ?? undefined);
  const commands = useWorkCommands(project.id);
  const working = run.turns.some((turn) => turn.state === 'running');
  const name = conversation.isWithAdvisor
    ? ADVISOR_NAME
    : (member?.name ?? 'A specialist who has left the team');
  // The question a specialist asked is answered where it was asked, once,
  // under the last of its turns.
  const question = run.isLastOfConversation && !working ? openDecision(conversation) : undefined;

  return (
    <section className={styles.run} aria-label={`Conversation with ${name}`}>
      <header className={styles.rule}>
        {member && <MemberAvatar member={member} />}
        <span className={styles.name}>{name}</span>
        {run.startsConversation && (
          <span className={classNames('label', styles.began)}>New conversation</span>
        )}
        <span className={styles.line} aria-hidden />
        {working && (
          <>
            <WorkingMark />
            {conversation.isHere && (
              <Button
                variant="ghost"
                icon={<Square />}
                onClick={() => {
                  void commands.stop(conversation.id);
                }}
              >
                Stop
              </Button>
            )}
          </>
        )}
      </header>

      <ol className={styles.turns}>
        {run.turns.map((turn) => (
          <li key={turn.id}>
            {conversation.isWithAdvisor ? (
              <TurnView turn={turn} reportTitle={null}>
                <AdvisorOutcome turn={turn} />
              </TurnView>
            ) : (
              <TurnView turn={turn} />
            )}
          </li>
        ))}
      </ol>

      {question && member && conversation.isHere && (
        <DecisionCard
          key={conversation.turns.length}
          decision={question}
          askedBy={member.name}
          onDecide={(answer) => commands.decide(conversation.id, answer)}
        />
      )}
    </section>
  );
}

/**
 * What the Advisor asked or proposed in a turn. Its questions and its
 * proposals need the room of the Advisor itself, so the chat names them
 * and leads there.
 */
function AdvisorOutcome({ turn }: { turn: Turn }) {
  const navigation = useNavigation();
  const what = turn.interview
    ? turn.interview.answers === null
      ? 'The Advisor asked questions.'
      : 'You answered the questions of the Advisor.'
    : turn.proposal
      ? turn.proposal.approvedAt === null
        ? 'The Advisor proposed a team and a plan.'
        : 'You approved the proposal of the Advisor.'
      : null;
  if (what === null) {
    return null;
  }
  return (
    <p className={styles.outcome}>
      <span>{what}</span>
      <button
        type="button"
        className={styles.link}
        onClick={() => {
          navigation.showPlace('advisor');
        }}
      >
        Open the Advisor
      </button>
    </p>
  );
}

interface ComposerProps {
  project: Project;
  tasks: readonly Task[];
  offered: readonly Addressee[];
  /** Whom the field is for when the page opens. */
  initial: Recipient | null;
  /** Tells that a message went out, so that the talk follows its answer. */
  onSent: () => void;
}

/**
 * The field at the foot: whom the message is for, the text, and "Send".
 * Ctrl and Enter send it; Escape clears it. A message goes out only when
 * the program would take it: a specialist works on one thing at a time, so
 * while it answers, the line under the field says so and nothing goes.
 */
function Composer({ project, tasks, offered, initial, onSent }: ComposerProps) {
  const conversations = useConversationsOfProject();
  const commands = useWorkCommands(project.id);
  const program = useAgentProgram();
  const navigation = useNavigation();
  const [chosen, setChosen] = useState<Recipient | null>(initial);
  const [message, setMessage] = useState('');
  const [isSending, setIsSending] = useState(false);
  const field = useRef<HTMLTextAreaElement>(null);

  // A specialist who left, or the first one who joined, changes whom the field can be for.
  const recipient = offered.some((addressee) => addressee.recipient === chosen) ? chosen : initial;

  const addressee = offered.find((each) => each.recipient === recipient);
  const sending = howToSend(conversations, recipient);
  const obstacle = program.data ? whyProgramIsNotReady(program.data) : null;
  const isOpen = canSend(sending) && obstacle === null;
  const isReady = isOpen && message.trim() !== '' && !isSending;
  const name = addressee?.name ?? '';
  const taskName = (taskId: string) => {
    const task = tasks.find((each) => each.id === taskId);
    return task ? taskReference(project, task) : 'a task';
  };
  const busy = sending.kind === 'busy' ? sending : undefined;
  const continued = sending.kind === 'reply' ? sending.conversation : undefined;

  async function send() {
    if (!isReady) {
      return;
    }
    setIsSending(true);
    onSent();
    const wasSent =
      sending.kind === 'reply'
        ? await commands.reply(sending.conversation.id, message)
        : recipient !== null && (await commands.ask(recipient, message)) !== undefined;
    setIsSending(false);
    if (wasSent) {
      setMessage('');
    }
  }

  function handleKeyDown(event: KeyboardEvent<HTMLTextAreaElement>) {
    if (event.key === 'Enter' && (event.ctrlKey || event.metaKey)) {
      event.preventDefault();
      void send();
    } else if (event.key === 'Escape' && message !== '') {
      event.stopPropagation();
      setMessage('');
    }
  }

  return (
    <div className={styles.foot}>
      <div className={styles.column}>
        <div className={styles.address}>
          <span className="label">To</span>
          {addressee ? (
            <ChoiceMenu
              label="Whom the message is for"
              choices={offered.map((each) => ({
                value: each.recipient,
                label: each.name,
                icon: each.member ? <MemberAvatar member={each.member} /> : undefined,
              }))}
              value={addressee.recipient}
              onChoose={(value) => {
                setChosen(value);
                field.current?.focus();
              }}
            >
              <ChoiceInput className={styles.chooser} aria-label="Whom the message is for">
                <span className={styles.chosen}>
                  {addressee.member && <MemberAvatar member={addressee.member} />}
                  <span>{addressee.name}</span>
                </span>
              </ChoiceInput>
            </ChoiceMenu>
          ) : (
            <span className={styles.nobody}>Nobody yet</span>
          )}
          <span className={styles.space} />
          {continued?.context && <ContextMeter context={continued.context} />}
        </div>

        <div className={styles.write}>
          <TextArea
            ref={field}
            className={styles.text}
            value={message}
            rows={2}
            disabled={addressee === undefined}
            placeholder={addressee ? `Write to ${name}…` : 'Add an AI specialist to the team first'}
            aria-label={addressee ? `Message to ${name}` : 'Message'}
            onChange={(event) => {
              setMessage(event.target.value);
            }}
            onKeyDown={handleKeyDown}
          />
          <Button
            variant="secondary"
            icon={<Send />}
            disabled={!isReady}
            title="Ctrl and Enter"
            onClick={() => {
              void send();
            }}
          >
            Send
          </Button>
        </div>

        <p className={styles.state} role="status">
          {busy && <WorkingMark />}
          <span>{obstacle ?? sendingInWords(sending, name, taskName)}</span>
          {sending.kind === 'interview' && (
            <button
              type="button"
              className={styles.link}
              onClick={() => {
                navigation.showPlace('advisor');
              }}
            >
              Open the Advisor
            </button>
          )}
          {busy?.conversation.taskId != null && <TaskLink taskId={busy.conversation.taskId} />}
          {isOpen && <span className={styles.keys}>Ctrl Enter</span>}
        </p>
      </div>
    </div>
  );
}

/** A press that opens the task a specialist is working on. */
function TaskLink({ taskId }: { taskId: string }) {
  const navigation = useNavigation();
  return (
    <button
      type="button"
      className={styles.link}
      onClick={() => {
        navigation.openTask(taskId);
      }}
    >
      Open the task
    </button>
  );
}
