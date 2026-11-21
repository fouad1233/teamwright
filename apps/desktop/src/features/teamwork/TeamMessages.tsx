import { ArrowRight } from 'lucide-react';

import type { TeamMessage } from '@/shared/api/types';
import { classNames } from '@/shared/lib/classNames';
import { formatMoment } from '@/shared/lib/format';

import { useTeamMembers } from '../team/TeamContext';
import { useTeamTalkOfProject } from '../work/WorkContext';
import styles from './TeamMessages.module.css';
import { kindInWords, messagesLatestFirst, nameOf, waitingCount } from './teamworkModel';

/**
 * The messages between the specialists of a project, the latest first: who
 * wrote to whom, a question or a note, what it says, and whether its
 * receiver has read it. The manager reads them here; nobody answers here,
 * because the specialists answer each other.
 */
export function TeamMessages() {
  const talk = useTeamTalkOfProject();
  const members = useTeamMembers();
  const messages = messagesLatestFirst(talk?.messages ?? []);
  const waiting = waitingCount(messages);

  return (
    <section className={styles.section} aria-labelledby="between-teammates">
      <header className={styles.header}>
        <h2 id="between-teammates" className="label">
          Between teammates
        </h2>
        {messages.length > 0 && (
          <span className={styles.count}>
            {messages.length.toString()} {messages.length === 1 ? 'message' : 'messages'}
            {waiting > 0 && ` · ${waiting.toString()} waiting`}
          </span>
        )}
      </header>
      {messages.length === 0 ? (
        <p className={styles.empty}>
          No messages yet. A specialist writes to another when it needs something from it; the
          message comes with the other one's next turn.
        </p>
      ) : (
        <ol className={styles.messages}>
          {messages.map((message) => (
            <li key={message.id} className={styles.message}>
              <MessageView
                message={message}
                from={nameOf(members, message.fromId)}
                to={nameOf(members, message.toId)}
              />
            </li>
          ))}
        </ol>
      )}
    </section>
  );
}

interface MessageViewProps {
  message: TeamMessage;
  from: string;
  to: string;
}

function MessageView({ message, from, to }: MessageViewProps) {
  const isWaiting = message.deliveredTurnId === null;
  return (
    <>
      <p className={styles.route}>
        <span className={styles.member}>{from}</span>
        <ArrowRight className={styles.arrow} aria-label="to" />
        <span className={styles.member}>{to}</span>
        <span className={classNames('label', styles.kind)}>{kindInWords(message.kind)}</span>
        <span className={styles.space} />
        <span className={styles.moment}>{formatMoment(message.sentAt)}</span>
      </p>
      <p className={styles.text} data-selectable>
        {message.text}
      </p>
      <p className={classNames(styles.state, isWaiting && styles.waiting)}>
        {isWaiting
          ? `Waits for the next turn of ${to}`
          : `Read by ${to} ${formatMoment(message.deliveredAt ?? message.sentAt)}`}
      </p>
    </>
  );
}
