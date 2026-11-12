import { Check, ExternalLink, Send, Square, X } from 'lucide-react';
import { useState, type KeyboardEvent } from 'react';

import { useIsDemo } from '@/shared/api/BackendProvider';
import type { Conversation, Project, Step, Turn } from '@/shared/api/types';
import { classNames } from '@/shared/lib/classNames';
import { formatMoment } from '@/shared/lib/format';
import { Button } from '@/shared/ui/Button';
import { Prose } from '@/shared/ui/Prose';

import { MemberAvatar } from '../team/Assignee';
import { useTeamMember } from '../team/TeamContext';
import styles from './ConversationView.module.css';
import { DecisionCard } from './DecisionCard';
import {
  decisionInWords,
  durationInWords,
  hasSession,
  openDecision,
  runningTurn,
  tokensInWords,
} from './workModel';
import { useWorkCommands } from './workQueries';

interface ConversationViewProps {
  project: Project;
  conversation: Conversation;
  /** Leaves the specialist's name out, where the place it is shown in already says it. */
  hidesMember?: boolean;
}

/** A conversation with a specialist: who it is, every turn, and what can be done next. */
export function ConversationView({
  project,
  conversation,
  hidesMember = false,
}: ConversationViewProps) {
  const member = useTeamMember(conversation.memberId ?? undefined);
  const commands = useWorkCommands(project.id);
  const isDemo = useIsDemo();
  const working = runningTurn(conversation);
  const question = openDecision(conversation);
  const name = member?.name ?? 'A specialist who has left the team';
  // The first line names the specialist and holds what can be done about
  // the conversation as a whole. Where there is nothing of either to show,
  // the line is left out instead of standing empty.
  // The demo has no real session behind a conversation, so it has nothing
  // to open in the Claude app.
  const canOpenInClaude = !isDemo && hasSession(conversation);
  const hasFirstLine = !hidesMember || working !== undefined || canOpenInClaude;

  return (
    <div className={styles.conversation}>
      <div className={styles.who} hidden={!hasFirstLine}>
        {!hidesMember && (
          <>
            {member && <MemberAvatar member={member} />}
            <span className={styles.name}>{name}</span>
          </>
        )}
        <span className={styles.space} />
        {working ? (
          <>
            <span className={classNames('label', styles.working)}>Working</span>
            <Button
              variant="ghost"
              icon={<Square />}
              onClick={() => {
                void commands.stop(conversation.id);
              }}
            >
              Stop
            </Button>
          </>
        ) : (
          canOpenInClaude && (
            <Button
              variant="ghost"
              icon={<ExternalLink />}
              onClick={() => {
                void commands.openInClaude(conversation.id);
              }}
            >
              Open in Claude
            </Button>
          )
        )}
      </div>

      <ol className={styles.turns}>
        {conversation.turns.map((turn) => (
          <li key={turn.id}>
            <TurnView turn={turn} />
          </li>
        ))}
      </ol>

      {!working && !member && (
        <p className={styles.nothing}>The specialist has left the team, so nobody can answer.</p>
      )}
      {/* A question of the specialist takes the place of the line to write
          in: answering it is the only way to go on. */}
      {!working && member && question && (
        <DecisionCard
          key={conversation.turns.length}
          decision={question}
          askedBy={member.name}
          onDecide={(answer) => commands.decide(conversation.id, answer)}
        />
      )}
      {!working && member && !question && (
        <ReplyBox
          label={`Message to ${member.name}`}
          placeholder={`Answer ${member.name}, or say what to change…`}
          onSend={(message) => commands.reply(conversation.id, message)}
        />
      )}
    </div>
  );
}

/** Who a request came from, as the line over it says. */
const ORIGIN_LABELS: Record<Turn['origin'], string> = {
  brief: 'Brief',
  message: 'You',
  decision: 'Your decision',
  interview: 'Your answers',
};

/** One request to a specialist: what was asked, what happened, and what came back. */
function TurnView({ turn }: { turn: Turn }) {
  // A brief is long and written by the program, so it is folded away. What
  // the manager wrote is short and is shown.
  const [isRequestShown, setIsRequestShown] = useState(turn.origin !== 'brief');
  const decided = turn.decision === null ? null : decisionInWords(turn.decision);

  return (
    <article className={styles.turn}>
      <header className={styles.asked}>
        <span className="label">
          {ORIGIN_LABELS[turn.origin]} · {formatMoment(turn.startedAt)}
        </span>
        {turn.origin === 'brief' && (
          <button
            type="button"
            className={styles.fold}
            aria-expanded={isRequestShown}
            onClick={() => {
              setIsRequestShown(!isRequestShown);
            }}
          >
            {isRequestShown ? 'Hide' : 'Show'}
          </button>
        )}
      </header>
      {isRequestShown && (
        <p className={styles.request} data-selectable>
          {turn.request}
        </p>
      )}

      {turn.steps.length > 0 && (
        <ol className={styles.steps}>
          {turn.steps.map((step, position) => (
            // Steps are only ever added at the end, so their place identifies them.
            <li key={position}>
              <StepView step={step} />
            </li>
          ))}
        </ol>
      )}

      {/* A turn ends in a report or in a question. What a specialist says
          next to its question is no report, so it gets no such title. */}
      {turn.answer !== '' && turn.decision === null && (
        <>
          <h4 className={classNames('label', styles.reportTitle)}>Report</h4>
          <Prose text={turn.answer} />
        </>
      )}
      {turn.decision !== null && decided !== null && (
        <div className={styles.decided}>
          <h4 className={classNames('label', styles.reportTitle)}>Asked</h4>
          <p data-selectable>{turn.decision.question}</p>
          <h4 className={classNames('label', styles.reportTitle)}>Decided</h4>
          <p data-selectable>{decided}</p>
        </div>
      )}

      {turn.failure !== null && (
        <p
          className={classNames(styles.ending, turn.state === 'failed' && styles.failed)}
          role={turn.state === 'failed' ? 'alert' : undefined}
        >
          {turn.failure}
        </p>
      )}

      {turn.endedAt !== null && (
        <p className={styles.used}>
          {[
            turn.model,
            turn.tokens ? tokensInWords(turn.tokens) : null,
            durationInWords(turn.startedAt, turn.endedAt),
          ]
            .filter((part) => part !== null)
            .join(' · ')}
        </p>
      )}
    </article>
  );
}

/** One thing a specialist did or said. */
function StepView({ step }: { step: Step }) {
  switch (step.kind) {
    case 'text':
      return (
        <div className={styles.said}>
          <Prose text={step.text} />
        </div>
      );
    case 'tool':
      return (
        <p className={styles.tool}>
          <span className={classNames('label', styles.toolName)}>{step.tool}</span>
          <span className={styles.toolSummary} data-selectable>
            {step.summary}
          </span>
          <ToolOutcome outcome={step.outcome} />
        </p>
      );
    case 'denied':
      return (
        <p className={classNames(styles.tool, styles.denied)}>
          <span className={classNames('label', styles.toolName)}>{step.tool}</span>
          <span className={styles.toolSummary}>Not allowed</span>
        </p>
      );
    case 'notice':
      return <Notice message={step.message} />;
  }
}

/**
 * Something the program said about the run itself: a retry, a warning about
 * its own setup. It is rarely about the work, so it takes one line until
 * the manager asks for all of it.
 */
function Notice({ message }: { message: string }) {
  const [isOpen, setIsOpen] = useState(false);

  return (
    <p className={styles.notice} data-open={isOpen ? '' : undefined}>
      <button
        type="button"
        className={classNames('label', styles.noticeName)}
        aria-expanded={isOpen}
        onClick={() => {
          setIsOpen(!isOpen);
        }}
      >
        Note
      </button>
      <span className={styles.noticeText} title={isOpen ? undefined : message} data-selectable>
        {message}
      </span>
    </p>
  );
}

/** The mark at the end of the line of a tool: still running, done, or failed. */
function ToolOutcome({ outcome }: { outcome: 'running' | 'done' | 'failed' }) {
  if (outcome === 'running') {
    return <span className={styles.busy} role="img" aria-label="Running" />;
  }
  return outcome === 'done' ? (
    <Check className={styles.done} aria-label="Done" />
  ) : (
    <X className={styles.broken} aria-label="Failed" />
  );
}

interface ReplyBoxProps {
  /** What the field is for, for screen readers. */
  label: string;
  /** What stands in the field while it is empty. */
  placeholder: string;
  /** Sends the message. Resolves to whether it was sent. */
  onSend: (message: string) => Promise<boolean>;
}

/** The field in which the manager writes to a specialist. Ctrl and Enter send it. */
export function ReplyBox({ label, placeholder, onSend }: ReplyBoxProps) {
  const [message, setMessage] = useState('');
  const [isSending, setIsSending] = useState(false);
  const canSend = message.trim() !== '' && !isSending;

  async function send() {
    if (!canSend) {
      return;
    }
    setIsSending(true);
    const wasSent = await onSend(message);
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
      // Escape clears what was typed. It must not also close the panel.
      event.stopPropagation();
      setMessage('');
    }
  }

  return (
    <div className={styles.reply}>
      <textarea
        className={styles.replyText}
        value={message}
        rows={2}
        placeholder={placeholder}
        aria-label={label}
        onChange={(event) => {
          setMessage(event.target.value);
        }}
        onKeyDown={handleKeyDown}
      />
      <Button
        variant="secondary"
        icon={<Send />}
        disabled={!canSend}
        title="Ctrl and Enter"
        onClick={() => {
          void send();
        }}
      >
        Send
      </Button>
    </div>
  );
}
