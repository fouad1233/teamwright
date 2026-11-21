import type { Handoff, TeamMessage } from '@/shared/api/types';
import { classNames } from '@/shared/lib/classNames';

import { useTeamMembers } from '../team/TeamContext';
import styles from './TalkOfTurnView.module.css';
import { hasTalk, kindInWords, nameOf, type TalkOfTurn } from './teamworkModel';

interface TalkOfTurnViewProps {
  talk: TalkOfTurn;
  /**
   * Which part of it: the messages of teammates that came with the turn,
   * which the specialist read before it began, or what it said itself:
   * the handoff it published and the messages it sent.
   */
  part: 'arrived' | 'said';
}

/**
 * What a turn had to do with the rest of the team, set as steps of the
 * turn. Nothing for a turn that had none.
 */
export function TalkOfTurnView({ talk, part }: TalkOfTurnViewProps) {
  const members = useTeamMembers();
  const isEmpty =
    !hasTalk(talk) ||
    (part === 'arrived'
      ? talk.arrived.length === 0
      : talk.handoffs.length + talk.sent.length === 0);
  if (isEmpty) {
    return null;
  }
  if (part === 'arrived') {
    return (
      <ol className={styles.talk}>
        {talk.arrived.map((message) => (
          <li key={message.id}>
            <MessageLine message={message} heading={`From ${nameOf(members, message.fromId)}`} />
          </li>
        ))}
      </ol>
    );
  }
  return (
    <ol className={styles.talk}>
      {talk.handoffs.map((handoff) => (
        <li key={handoff.id}>
          <HandoffLine handoff={handoff} />
        </li>
      ))}
      {talk.sent.map((message) => (
        <li key={message.id}>
          <MessageLine message={message} heading={`To ${nameOf(members, message.toId)}`} />
        </li>
      ))}
    </ol>
  );
}

/** A handoff the turn published: its summary and its files. */
function HandoffLine({ handoff }: { handoff: Handoff }) {
  return (
    <div className={styles.line}>
      <span className={classNames('label', styles.name)}>Handoff</span>
      <div className={styles.body}>
        <p className={styles.text} data-selectable>
          {handoff.summary}
        </p>
        {handoff.files.length > 0 && (
          <p className={styles.files} data-selectable>
            {handoff.files.join('  ·  ')}
          </p>
        )}
      </div>
    </div>
  );
}

interface MessageLineProps {
  message: TeamMessage;
  /** Who it is from or to. */
  heading: string;
}

/** A message between teammates: who, what kind, and what it says. */
function MessageLine({ message, heading }: MessageLineProps) {
  return (
    <div className={styles.line}>
      <span className={classNames('label', styles.name)}>Message</span>
      <div className={styles.body}>
        <p className={styles.who}>
          {heading} · {kindInWords(message.kind)}
        </p>
        <p className={styles.text} data-selectable>
          {message.text}
        </p>
      </div>
    </div>
  );
}
